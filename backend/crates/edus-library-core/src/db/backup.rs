//! Same-workspace backup operations. No caller-provided filesystem paths.
use super::*;
use serde::Serialize;
use std::path::{Component, Path};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    pub file_name: String,
    pub size_bytes: u64,
    pub modified_unix_seconds: u64,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    pub restored_file_name: String,
    pub recovery_file_name: String,
    pub integrity: String,
}

fn backup_name(value: &str) -> Result<(), AppError> {
    let valid = value
        .strip_prefix("library-")
        .and_then(|s| s.strip_suffix(".db"))
        .is_some_and(|id| Uuid::parse_str(id).is_ok_and(|uuid| uuid.to_string() == id));
    if !valid
        || Path::new(value)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(AppError::new(
            "BACKUP_NAME_INVALID",
            "Выберите существующую резервную копию из списка.",
        ));
    }
    Ok(())
}
fn no_reparse(path: &Path) -> Result<(), AppError> {
    for component in path.ancestors() {
        let metadata = fs::symlink_metadata(component)?;
        #[cfg(windows)]
        let reparse = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_attributes() & 0x400 != 0
        };
        #[cfg(not(windows))]
        let reparse = metadata.file_type().is_symlink();
        if reparse {
            return Err(AppError::new(
                "BACKUP_PATH_INVALID",
                "Перенаправленный путь резервной копии запрещён.",
            ));
        }
    }
    Ok(())
}
fn schema(conn: &Connection) -> Result<Vec<(String, String, String)>, AppError> {
    let mut statement = conn.prepare(
        "SELECT type,name,sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY type,name",
    )?;
    let rows = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}
fn schools(conn: &Connection) -> Result<Vec<String>, AppError> {
    let mut statement = conn.prepare("SELECT id FROM schools ORDER BY id")?;
    let rows = statement.query_map([], |r| r.get(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}
fn integrity(conn: &Connection) -> Result<(), AppError> {
    let result: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    let foreign_key_error = conn.prepare("PRAGMA foreign_key_check")?.exists([])?;
    if result != "ok" || foreign_key_error {
        return Err(AppError::new(
            "BACKUP_INTEGRITY_FAILED",
            "Резервная копия не прошла проверку целостности.",
        ));
    }
    Ok(())
}
impl LocalDatabase {
    pub fn list_backups(&self) -> Result<Vec<BackupRecord>, AppError> {
        let directory = self
            .path
            .parent()
            .ok_or_else(|| AppError::new("LOCAL_STORAGE_ERROR", "Путь базы недоступен."))?
            .join("backups");
        if !directory.exists() {
            return Ok(Vec::new());
        }
        no_reparse(&directory)?;
        let mut records = Vec::new();
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if backup_name(&file_name).is_err() {
                continue;
            }
            no_reparse(&entry.path())?;
            let metadata = entry.metadata()?;
            if !metadata.is_file() {
                continue;
            }
            records.push(BackupRecord {
                file_name,
                size_bytes: metadata.len(),
                modified_unix_seconds: metadata
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
            if records.len() > 1000 {
                return Err(AppError::new("BACKUP_LIMIT", "Слишком много резервных копий. Требуется обслуживание каталога администратором."));
            }
        }
        records.sort_by_key(|record| std::cmp::Reverse(record.modified_unix_seconds));
        Ok(records)
    }

    /// Caller must hold its workspace lease and DB mutex; explicitly confirm in
    /// admin IPC before invoking. Production recovery needs Cloud reconciliation
    /// and deliberately fails closed here. All current UAT data are snapshotted
    /// before restoring a historical state through SQLite's atomic Backup API.
    pub fn restore_backup(
        &mut self,
        file_name: &str,
        credentials: &CredentialStore,
    ) -> Result<RestoreReport, AppError> {
        self.require_test_workspace()?;
        if !credentials.is_test_mode() {
            return Err(AppError::new(
                "TEST_MODE_REQUIRED",
                "Восстановление доступно только для UAT.",
            ));
        }
        backup_name(file_name)?;
        let source_path = self
            .path
            .parent()
            .ok_or_else(|| AppError::new("LOCAL_STORAGE_ERROR", "Путь базы недоступен."))?
            .join("backups")
            .join(file_name);
        no_reparse(&source_path)?;
        let metadata = fs::metadata(&source_path)?;
        if !metadata.is_file() || metadata.len() < 4096 || metadata.len() > 8 * 1024 * 1024 * 1024 {
            return Err(AppError::new(
                "BACKUP_SIZE_INVALID",
                "Размер резервной копии недопустим.",
            ));
        }
        if self.disk_available()?
            < metadata
                .len()
                .saturating_mul(3)
                .saturating_add(16 * 1024 * 1024)
        {
            return Err(AppError::new(
                "DISK_FULL",
                "Недостаточно места для безопасного восстановления и страховочной копии.",
            ));
        }
        let source =
            Connection::open_with_flags(&source_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        source.pragma_update(None, "key", credentials.existing_database_key()?)?;
        source.execute_batch("PRAGMA cipher_memory_security=ON; PRAGMA query_only=ON;")?;
        integrity(&source)?;
        if schema(&source).map_err(|e| AppError::new("RESTORE_SOURCE_SCHEMA", e.message))?
            != schema(&self.conn)?
            || schools(&source).map_err(|e| AppError::new("RESTORE_SOURCE_SCHOOL", e.message))?
                != schools(&self.conn)?
        {
            return Err(AppError::new(
                "BACKUP_INCOMPATIBLE",
                "Копия относится к другой схеме или школе.",
            ));
        }
        self.checkpoint_for_maintenance()?;
        let recovery = self.encrypted_backup(credentials)?;
        let backup = rusqlite::backup::Backup::new(&source, &mut self.conn).map_err(|e| {
            AppError::new(
                "RESTORE_INIT_FAILED",
                format!("{:?}", e.sqlite_error_code()),
            )
        })?;
        copy_bounded(&backup)?;
        drop(backup);
        self.conn.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA secure_delete=ON; PRAGMA cipher_memory_security=ON;",
        )?;
        integrity(&self.conn).map_err(|e| AppError::new("RESTORE_TARGET_INTEGRITY", e.message))?;
        self.conn.execute("INSERT INTO service_recovery_events(id,action,reason_code,occurred_at) VALUES(?1,'UAT_BACKUP_RESTORED','CONFIRMED_ADMIN_SAME_WORKSPACE',?2)", params![Uuid::new_v4().to_string(),Self::now()])?;
        self.checkpoint_for_maintenance()?;
        Ok(RestoreReport {
            restored_file_name: file_name.into(),
            recovery_file_name: recovery
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| {
                    AppError::new("BACKUP_NAME_INVALID", "Имя страховочной копии недопустимо.")
                })?
                .into(),
            integrity: "ok".into(),
        })
    }
}

// SQLite rolls back an incomplete destination copy when Backup is dropped.
// A competing database holder must not make administrative shutdown hang forever.
pub(super) fn copy_bounded(backup: &rusqlite::backup::Backup<'_, '_>) -> Result<(), AppError> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        match backup.step(128)? {
            rusqlite::backup::StepResult::Done => return Ok(()),
            _ if std::time::Instant::now() >= deadline => return Err(AppError::new("BACKUP_TIMEOUT", "Копирование не завершено за отведённое время. Повторите после завершения других операций.")),
            _ => std::thread::sleep(std::time::Duration::from_millis(5)),
        }
    }
}
