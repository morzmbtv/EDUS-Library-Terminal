mod backup;
use crate::{
    domain::{
        AppError, BasketItem, BookAvailability, CodeResult, Connection as UiConnection, Copy,
        ExpectedReturn, Loan, OperationResult, Reader, Reservation, Snapshot, TestBookRecord,
        TestCsvPreview, TestCsvPreviewRow, TestReaderInput, TestReaderRecord, TestReaderUpsert,
        Title,
    },
    security::{card_lookup_hash, payload_hash, CredentialStore},
    sync::dto::validate_change,
};
pub use backup::{BackupRecord, RestoreReport};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::Value;
use std::{collections::HashMap, fs, path::PathBuf};
use uuid::Uuid;

/// operation ID, kind, payload, hash, timestamp used by the Cloud transport.
pub type PendingOutboxRecord = (String, String, String, String, String);

pub struct LocalDatabase {
    conn: Connection,
    pub path: PathBuf,
    sqlcipher: bool,
    test_mode: bool,
}

impl LocalDatabase {
    pub fn open(path: PathBuf, credentials: &CredentialStore) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let existed = path.exists();
        let key = if existed {
            credentials.existing_database_key()?
        } else {
            credentials.database_key()?
        };
        let mut conn = Connection::open(&path)?;
        conn.pragma_update(None, "key", &key)?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA secure_delete = ON; PRAGMA cipher_memory_security = ON;")?;
        let version: String = conn.query_row("PRAGMA cipher_version", [], |row| row.get(0))?;
        let hardening: String =
            conn.query_row("PRAGMA cipher_memory_security", [], |row| row.get(0))?;
        if !version.starts_with("4.18.0") || hardening != "1" {
            return Err(AppError::new(
                "DATABASE_SECURITY_POLICY",
                "SQLCipher не соответствует политике безопасности.",
            ));
        }
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("../../../../migrations/0001_initial.sql"))?;
        tx.execute_batch(include_str!(
            "../../../../migrations/0002_cloud_projection.sql"
        ))?;
        tx.execute_batch(include_str!("../../../../migrations/0003_admin_audit.sql"))?;
        tx.execute_batch(include_str!(
            "../../../../migrations/0004_terminal_test_readers.sql"
        ))?;
        tx.execute_batch(include_str!(
            "../../../../migrations/0005_conflict_projection.sql"
        ))?;
        tx.execute_batch(include_str!(
            "../../../../migrations/0006_recovery_action_audit.sql"
        ))?;
        tx.execute_batch(include_str!(
            "../../../../migrations/0007_query_indexes.sql"
        ))?;
        tx.execute_batch(include_str!(
            "../../../../migrations/0008_service_recovery_audit.sql"
        ))?;
        tx.commit()?;
        let mut database = Self {
            conn,
            path,
            sqlcipher: true,
            test_mode: credentials.is_test_mode(),
        };
        if credentials.is_test_mode() && !existed {
            credentials.initialize_uat_card_secret()?;
            database.seed_if_empty(credentials)?;
            // The UAT database is created on first launch and receives only
            // explicitly fictional readers in addition to its book fixtures.
            // Reader seed is an explicit service action, never an automatic production fallback.
        }
        Ok(database)
    }

    fn require_test_workspace(&self) -> Result<(), AppError> {
        if !self.test_mode {
            return Err(AppError::new(
                "TEST_MODE_REQUIRED",
                "Операция доступна только в UAT-пространстве.",
            ));
        }
        Ok(())
    }
    pub fn sqlcipher_enabled(&self) -> bool {
        self.sqlcipher
    }

    pub fn cipher_version(&self) -> Result<String, AppError> {
        self.conn
            .query_row("PRAGMA cipher_version", [], |row| row.get(0))
            .map_err(Into::into)
    }

    pub fn cipher_memory_hardening(&self) -> Result<String, AppError> {
        self.conn
            .query_row("PRAGMA cipher_memory_security", [], |row| row.get(0))
            .map_err(Into::into)
    }

    /// Records a security-relevant terminal administration transition. This
    /// intentionally contains a reason code only: no Windows username,
    /// password, credential buffer or reusable token is persisted.
    pub fn record_admin_audit(&self, event_type: &str, reason_code: &str) -> Result<(), AppError> {
        self.conn.execute(
            "INSERT INTO admin_audit_events(id, event_type, reason_code, occurred_at) VALUES (?1, ?2, ?3, ?4)",
            params![Uuid::new_v4().to_string(), event_type, reason_code, Self::now()],
        )?;
        Ok(())
    }

    /// Checkpoints the encrypted WAL for service stop, workspace switch or backup.
    pub fn checkpoint_for_maintenance(&self) -> Result<(), AppError> {
        let busy: i64 = self
            .conn
            .query_row("PRAGMA wal_checkpoint(FULL)", [], |r| r.get(0))?;
        if busy != 0 {
            return Err(AppError::new(
                "DATABASE_BUSY",
                "Дождитесь завершения записи перед обслуживанием.",
            ));
        }
        Ok(())
    }
    pub fn disk_available(&self) -> Result<u64, AppError> {
        crate::storage::available_bytes(
            self.path
                .parent()
                .ok_or_else(|| AppError::new("LOCAL_STORAGE_ERROR", "Некорректный путь базы."))?,
        )
    }
    fn ensure_write_space(&self) -> Result<(), AppError> {
        crate::storage::require_write_space(self.disk_available()?)
    }
    pub fn health_check(&self) -> Result<String, AppError> {
        let check: String = self
            .conn
            .query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if check != "ok" {
            return Err(AppError::new(
                "DATABASE_INTEGRITY_ERROR",
                "Проверка целостности базы не пройдена.",
            ));
        }
        Ok(check)
    }
    pub fn school_name(&self) -> Result<String, AppError> {
        self.conn
            .query_row("SELECT name FROM schools ORDER BY rowid LIMIT 1", [], |r| {
                r.get(0)
            })
            .optional()
            .map(|v| v.unwrap_or_else(|| "EDUS Library".into()))
            .map_err(Into::into)
    }
    pub fn record_recovery_action(&self, action: &str, reason: &str) -> Result<(), AppError> {
        self.conn.execute("INSERT INTO recovery_action_audit(id,action,reason_code,occurred_at) VALUES(?1,?2,?3,?4)",params![Uuid::new_v4().to_string(),action,reason,Self::now()])?;
        Ok(())
    }
    pub fn encrypted_backup(&self, credentials: &CredentialStore) -> Result<PathBuf, AppError> {
        self.ensure_write_space()?;
        let directory = self
            .path
            .parent()
            .ok_or_else(|| AppError::new("LOCAL_STORAGE_ERROR", "Путь базы недоступен."))?
            .join("backups");
        fs::create_dir_all(&directory)?;
        let target = directory.join(format!("library-{}.db", Uuid::new_v4()));
        let temporary = target.with_extension("partial");
        let mut destination = Connection::open(&temporary).map_err(|e| {
            AppError::new("BACKUP_OPEN_FAILED", format!("{:?}", e.sqlite_error_code()))
        })?;
        destination
            .pragma_update(None, "key", credentials.existing_database_key()?)
            .map_err(|e| {
                AppError::new("BACKUP_KEY_FAILED", format!("{:?}", e.sqlite_error_code()))
            })?;
        destination
            .execute_batch("PRAGMA cipher_memory_security=ON; PRAGMA secure_delete=ON;")
            .map_err(|e| {
                AppError::new(
                    "BACKUP_POLICY_FAILED",
                    format!("{:?}", e.sqlite_error_code()),
                )
            })?;
        let backup = rusqlite::backup::Backup::new(&self.conn, &mut destination).map_err(|e| {
            AppError::new(
                "BACKUP_INIT_FAILED",
                format!("Backup init: {:?}", e.sqlite_error_code()),
            )
        })?;
        backup::copy_bounded(&backup)?;
        drop(backup);
        let check: String = destination
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .map_err(|e| {
                AppError::new(
                    "BACKUP_VERIFY_FAILED",
                    format!("{:?}", e.sqlite_error_code()),
                )
            })?;
        if check != "ok" {
            return Err(AppError::new(
                "BACKUP_FAILED",
                "Резервная копия не прошла проверку.",
            ));
        }
        drop(destination);
        fs::rename(&temporary, &target)?;
        self.record_recovery_action("ENCRYPTED_BACKUP_CREATED", "SQLCIPHER_BACKUP_API")?;
        Ok(target)
    }
    fn now() -> String {
        Utc::now().to_rfc3339()
    }
    fn seed_if_empty(&mut self, credentials: &CredentialStore) -> Result<(), AppError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM schools", [], |row| row.get(0))?;
        if count > 0 {
            return Ok(());
        }
        let now = Self::now();
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO schools(id,name,updated_at) VALUES ('school-demo','Школа-лицей № 23',?1)",
            params![now],
        )?;
        tx.execute("INSERT INTO terminals(id,school_id,name,updated_at) VALUES ('terminal-local','school-demo','EDUS Library local',?1)", params![now])?;
        tx.execute("INSERT INTO classes(id,school_id,name,updated_at) VALUES ('class-7a','school-demo','7 «А» класс',?1),('class-8b','school-demo','8 «Б» класс',?1)", params![now])?;
        tx.execute("INSERT INTO persons(id,school_id,full_name,person_type,class_id,class_name,status,updated_at) VALUES
          ('reader-ali','school-demo','Тестовый ученик Алихан','STUDENT','class-7a','7 «А» класс','ACTIVE',?1),
          ('reader-ayla','school-demo','Тестовый сотрудник Алия','STAFF',NULL,'Сотрудник','ACTIVE',?1),
          ('reader-dana','school-demo','Тестовая ученица Дана','STUDENT','class-8b','8 «Б» класс','ACTIVE',?1)", params![now])?;
        let secret = credentials.card_hmac_secret()?;
        for (id, person, raw) in [
            ("card-ali", "reader-ali", "EDUS-1234567890"),
            ("card-ayla", "reader-ayla", "EDUS-0000000001"),
            ("card-dana", "reader-dana", "EDUS-0000000002"),
        ] {
            tx.execute("INSERT INTO cards(id,person_id,card_lookup_hash,status,updated_at) VALUES (?1,?2,?3,'ACTIVE',?4)", params![id,person,card_lookup_hash(raw, &secret)?,now])?;
        }
        let titles = [
            (
                "title-math",
                "Математика 7 класс",
                "А. Е. Абылкасымова",
                "9786010123456",
                "Алматыкітап",
                "2024",
                "Қазақша",
                "Математика",
                "7",
            ),
            (
                "title-kazakh",
                "Қазақ тілі 7 сынып",
                "Ш. Құрманбайұлы",
                "9786010123457",
                "Атамұра",
                "2024",
                "Қазақша",
                "Қазақ тілі",
                "7",
            ),
            (
                "title-history",
                "История Казахстана",
                "Б. Г. Аяган",
                "9786010123458",
                "Мектеп",
                "2023",
                "Русский",
                "История",
                "7",
            ),
            (
                "title-russian",
                "Русский язык",
                "Л. А. Тростенцова",
                "9786010123459",
                "Просвещение",
                "2023",
                "Русский",
                "Русский язык",
                "7",
            ),
            (
                "title-biolog",
                "Биология 7 класс",
                "Н. И. Сонин",
                "9786010123460",
                "Мектеп",
                "2022",
                "Русский",
                "Биология",
                "7",
            ),
            (
                "title-abai1",
                "Абай жолы. 1-том",
                "Мұхтар Әуезов",
                "9786010212345",
                "Жазушы",
                "1984",
                "Қазақша",
                "Роман",
                "",
            ),
            (
                "title-abai2",
                "Абай жолы. 2-том",
                "Мұхтар Әуезов",
                "9786010212346",
                "Жазушы",
                "1984",
                "Қазақша",
                "Роман",
                "",
            ),
            (
                "title-abai3",
                "Абай жолы. 3-том",
                "Мұхтар Әуезов",
                "9786010212347",
                "Жазушы",
                "1985",
                "Қазақша",
                "Роман",
                "",
            ),
        ];
        for (id, title, authors, isbn, publisher, year, language, subject, grade) in titles {
            tx.execute("INSERT INTO book_titles(id,school_id,isbn,title,authors,publisher,publication_year,language,subject,grade,updated_at) VALUES (?1,'school-demo',?2,?3,?4,?5,?6,?7,?8,?9,?10)", params![id,isbn,title,authors,publisher,year.parse::<i32>().unwrap(),language,subject,grade,now])?;
        }
        let copies = [
            ("copy-math", "title-math", "000123456"),
            ("copy-kazakh", "title-kazakh", "000234567"),
            ("copy-history", "title-history", "000345678"),
            ("copy-biolog", "title-biolog", "000567890"),
            ("copy-abai2a", "title-abai2", "0004512"),
            ("copy-abai2b", "title-abai2", "0004891"),
            ("copy-abai2c", "title-abai2", "0005103"),
            ("copy-abai3a", "title-abai3", "0006301"),
            ("copy-abai3b", "title-abai3", "0006302"),
        ];
        for (id, title, code) in copies {
            tx.execute("INSERT INTO book_copies(id,book_title_id,inventory_number,barcode,status,updated_at) VALUES (?1,?2,?3,?3,'AVAILABLE',?4)", params![id,title,code,now])?;
        }
        tx.execute("INSERT INTO legacy_title_stock(book_title_id,total_quantity,available_quantity,updated_at) VALUES ('title-russian',4,2,?1)", params![now])?;
        for (id, title, copy, mode, qty) in [
            ("loan-math", "title-math", Some("copy-math"), "COPY", 1),
            (
                "loan-kazakh",
                "title-kazakh",
                Some("copy-kazakh"),
                "COPY",
                1,
            ),
            (
                "loan-history",
                "title-history",
                Some("copy-history"),
                "COPY",
                1,
            ),
            ("loan-russian", "title-russian", None, "LEGACY_TITLE", 2),
            (
                "loan-biolog",
                "title-biolog",
                Some("copy-biolog"),
                "COPY",
                1,
            ),
        ] {
            tx.execute("INSERT INTO loans(id,reader_id,book_title_id,book_copy_id,accounting_mode,quantity,issued_at,due_at,status,created_terminal_id,updated_at) VALUES (?1,'reader-ali',?2,?3,?4,?5,?6,'2026-10-01T00:00:00Z','ACTIVE','terminal-local',?6)", params![id,title,copy,mode,qty,now])?;
            if let Some(copy) = copy {
                tx.execute(
                    "UPDATE book_copies SET status='ON_LOAN' WHERE id=?1",
                    params![copy],
                )?;
            }
        }
        tx.execute("UPDATE legacy_title_stock SET available_quantity=2 WHERE book_title_id='title-russian'", [])?;
        tx.commit()?;
        Ok(())
    }

    pub fn snapshot(&self, online: bool) -> Result<Snapshot, AppError> {
        let mut legacy_title_stock = HashMap::new();
        let mut stock = self
            .conn
            .prepare("SELECT book_title_id,total_quantity FROM legacy_title_stock")?;
        for row in stock.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i32>(1)?)))? {
            let (id, count) = row?;
            legacy_title_stock.insert(id, count);
        }
        Ok(Snapshot {
            readers: self.readers("")?,
            titles: self.titles("")?,
            copies: self.copies()?,
            loans: self.loans_all()?,
            reservations: self.reservations()?,
            legacy_stock: legacy_title_stock,
            connection: UiConnection {
                server: true,
                internet: online,
            },
            persistence: "sqlcipher".into(),
        })
    }
    pub fn readers(&self, query: &str) -> Result<Vec<Reader>, AppError> {
        let term = format!("%{}%", query.trim());
        let mut statement=self.conn.prepare("SELECT id,full_name,coalesce(class_name,position_name,'Читатель') FROM persons WHERE deleted_at IS NULL AND status='ACTIVE' AND (full_name LIKE ?1 OR class_name LIKE ?1 OR position_name LIKE ?1) ORDER BY full_name")?;
        let rows = statement.query_map(params![term], |r| {
            Ok(Reader {
                id: r.get(0)?,
                name: r.get(1)?,
                group: r.get(2)?,
                card: "Карта определена".into(),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
    pub fn reader_by_card_hash(&self, hash: &str) -> Result<Reader, AppError> {
        self.conn.query_row("SELECT p.id,p.full_name,coalesce(p.class_name,p.position_name,'Читатель') FROM cards c JOIN persons p ON p.id=c.person_id WHERE c.card_lookup_hash=?1 AND c.status='ACTIVE' AND p.status='ACTIVE' AND p.deleted_at IS NULL", params![hash], |r| Ok(Reader{id:r.get(0)?,name:r.get(1)?,group:r.get(2)?,card:"Карта определена".into()})).map_err(|error| match error { rusqlite::Error::QueryReturnedNoRows => AppError::new("NOT_FOUND","Карта не привязана к активному читателю."), other=>other.into() })
    }
    pub fn titles(&self, query: &str) -> Result<Vec<Title>, AppError> {
        if query.len() > 512 {
            return Err(AppError::new(
                "INVALID_INPUT",
                "Слишком длинный запрос поиска.",
            ));
        }
        let columns="id,title,authors,coalesce(isbn,''),coalesce(publisher,''),coalesce(publication_year,0),language,coalesce(subject,''),coalesce(grade,'')";
        if query.trim().is_empty() {
            let mut st = self.conn.prepare(&format!(
                "SELECT {columns} FROM book_titles WHERE deleted_at IS NULL ORDER BY title"
            ))?;
            return Ok(st
                .query_map([], title_from_row)?
                .collect::<Result<Vec<_>, _>>()?);
        }
        let tokens = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .take(12)
            .map(|s| format!("\"{s}\"*"))
            .collect::<Vec<_>>();
        if tokens.is_empty() {
            return Ok(vec![]);
        }
        let normalized = query.replace(['-', ' '], "").to_uppercase();
        let mut st=self.conn.prepare(&format!("SELECT {columns} FROM book_titles WHERE deleted_at IS NULL AND (rowid IN (SELECT rowid FROM book_title_fts WHERE book_title_fts MATCH ?1) OR replace(replace(upper(isbn),'-',''),' ','')=?2) ORDER BY title LIMIT 200"))?;
        let rows = st
            .query_map(params![tokens.join(" AND "), normalized], title_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
    pub fn title(&self, id: &str) -> Result<Option<Title>, AppError> {
        self.conn.query_row("SELECT id,title,authors,coalesce(isbn,''),coalesce(publisher,''),coalesce(publication_year,0),language,coalesce(subject,''),coalesce(grade,'') FROM book_titles WHERE id=?1 AND deleted_at IS NULL",params![id],title_from_row).optional().map_err(Into::into)
    }
    pub fn copies(&self) -> Result<Vec<Copy>, AppError> {
        let mut st=self.conn.prepare("SELECT c.id,c.book_title_id,coalesce(c.inventory_number,c.barcode,c.safeschool_code,''),c.status,l.name FROM book_copies c LEFT JOIN library_locations l ON l.id=c.location_id WHERE c.deleted_at IS NULL ORDER BY c.id")?;
        let rows = st.query_map([], copy_from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
    pub fn loans_all(&self) -> Result<Vec<Loan>, AppError> {
        let mut st=self.conn.prepare("SELECT id,reader_id,book_title_id,book_copy_id,quantity,accounting_mode,due_at FROM loans WHERE status='ACTIVE' ORDER BY issued_at")?;
        let rows = st.query_map([], loan_from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
    pub fn reader_loans(&self, reader: &str) -> Result<Vec<Loan>, AppError> {
        let mut st=self.conn.prepare("SELECT id,reader_id,book_title_id,book_copy_id,quantity,accounting_mode,due_at FROM loans WHERE status='ACTIVE' AND reader_id=?1 ORDER BY issued_at")?;
        let rows = st.query_map(params![reader], loan_from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
    pub fn reservations(&self) -> Result<Vec<Reservation>, AppError> {
        let mut st=self.conn.prepare("SELECT id,reader_id,book_title_id,created_at,status FROM reservations ORDER BY created_at")?;
        let rows = st.query_map([], reservation_from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
    pub fn resolve_code(&self, code: &str) -> Result<CodeResult, AppError> {
        let code = code.trim();
        if code.is_empty() {
            return Err(AppError::new("INVALID_INPUT", "Введите код книги."));
        }
        let matches:i64=self.conn.query_row("SELECT COUNT(*) FROM book_copies WHERE deleted_at IS NULL AND (?1=inventory_number OR ?1=barcode OR ?1=safeschool_code)",params![code],|row|row.get(0))?;
        if matches > 1 {
            return Err(AppError::new("DUPLICATE_CODE", "Код связан с несколькими экземплярами. Выберите книгу на руках или проверьте инвентарный номер."));
        }
        if let Some(copy)=self.conn.query_row("SELECT c.id,c.book_title_id,coalesce(c.inventory_number,c.barcode,c.safeschool_code,''),c.status,l.name FROM book_copies c LEFT JOIN library_locations l ON l.id=c.location_id WHERE c.deleted_at IS NULL AND (?1=c.inventory_number OR ?1=c.barcode OR ?1=c.safeschool_code)",params![code],copy_from_row).optional()? { let title=self.title(&copy.title_id)?.ok_or_else(||AppError::new("NOT_FOUND","Издание не найдено."))?; let loan=self.conn.query_row("SELECT id,reader_id,book_title_id,book_copy_id,quantity,accounting_mode,due_at FROM loans WHERE book_copy_id=?1 AND status='ACTIVE'",params![copy.id],loan_from_row).optional()?; return Ok(CodeResult{kind:"copy".into(),copy:Some(copy),title:Some(title),loan,titles:vec![],code:None}); }
        let titles=self.conn.prepare("SELECT id,title,authors,coalesce(isbn,''),coalesce(publisher,''),coalesce(publication_year,0),language,coalesce(subject,''),coalesce(grade,'') FROM book_titles WHERE deleted_at IS NULL AND replace(replace(upper(isbn),'-',''),' ','')=?1")?.query_map(params![code.replace(['-', ' '], "").to_uppercase()],title_from_row)?.collect::<Result<Vec<_>,_>>()?;
        if titles.len() == 1 {
            return Ok(CodeResult {
                kind: "title".into(),
                copy: None,
                title: titles.first().cloned(),
                loan: None,
                titles: vec![],
                code: None,
            });
        }
        if titles.len() > 1 {
            return Ok(CodeResult {
                kind: "ambiguous".into(),
                copy: None,
                title: None,
                loan: None,
                titles,
                code: None,
            });
        }
        Ok(CodeResult {
            kind: "not-found".into(),
            copy: None,
            title: None,
            loan: None,
            titles: vec![],
            code: Some(code.into()),
        })
    }
    fn existing_operation(
        &self,
        operation_id: &str,
        hash: &str,
        operation_type: &str,
    ) -> Result<Option<OperationResult>, AppError> {
        let row: Option<(String, String, String)> = self
            .conn
            .query_row(
                "SELECT payload_hash,result_json,type FROM library_operations WHERE operation_id=?1",
                params![operation_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        match row {
            Some((stored, result, stored_type))
                if stored == hash && stored_type == operation_type =>
            {
                Ok(Some(serde_json::from_str(&result).map_err(|_| {
                    AppError::new(
                        "LOCAL_DATABASE_ERROR",
                        "Повреждён сохранённый результат операции.",
                    )
                })?))
            }
            Some(_) => Err(AppError {
                code: "OPERATION_CONFLICT".into(),
                message: "Этот идентификатор операции уже использован с другими данными.".into(),
                operation_id: Some(operation_id.into()),
            }),
            None => Ok(None),
        }
    }
    fn persist_operation(
        tx: &Transaction<'_>,
        result: &OperationResult,
        payload: &str,
        hash: &str,
        entity_type: &str,
        entity_id: &str,
        reader_id: Option<&str>,
    ) -> Result<(), AppError> {
        let now = Self::now();
        // The frontend response is intentionally lower-case (`issue`, `accept`,
        // and so on), while the durable schema uses a constrained canonical
        // operation type for audit and sync records.
        let persisted_operation_type = result.operation_type.to_ascii_uppercase();
        let encoded = serde_json::to_string(result).map_err(|_| {
            AppError::new(
                "LOCAL_DATABASE_ERROR",
                "Не удалось сохранить результат операции.",
            )
        })?;
        tx.execute("INSERT INTO library_operations(id,operation_id,type,entity_type,entity_id,reader_id,terminal_id,occurred_at,status,payload_hash,payload_json,result_json,created_at) VALUES (?1,?2,?3,?4,?5,?6,'terminal-local',?7,'COMMITTED',?8,?9,?10,?7)",params![Uuid::new_v4().to_string(),result.operation_id,persisted_operation_type,entity_type,entity_id,reader_id,now,hash,payload,encoded])?;
        tx.execute("INSERT INTO sync_outbox(id,operation_id,operation_type,payload_json,payload_hash,created_at,status) VALUES (?1,?2,?3,?4,?5,?6,'PENDING')",params![Uuid::new_v4().to_string(),result.operation_id,persisted_operation_type,payload,hash,now])?;
        Ok(())
    }
    pub fn issue(
        &mut self,
        reader: &str,
        items: &[BasketItem],
        operation_id: &str,
    ) -> Result<OperationResult, AppError> {
        validate_basket(items)?;
        let mut titles = std::collections::HashSet::new();
        if items
            .iter()
            .filter(|item| item.mode == "LEGACY_TITLE")
            .any(|item| !titles.insert(&item.title_id))
        {
            return Err(AppError::new(
                "INVALID_INPUT",
                "Объедините количество одного издания в одной позиции.",
            ));
        }
        let items = items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let mut item = item.clone();
                item.loan_id = Some(crate::security::operation_entity_id(
                    operation_id,
                    "loan",
                    index,
                ));
                item
            })
            .collect::<Vec<_>>();
        validate_basket(&items)?;
        let payload = serde_json::to_string(&(reader, &items, operation_id))?;
        let hash = payload_hash(&payload);
        if let Some(result) = self.existing_operation(operation_id, &hash, "ISSUE")? {
            return Ok(result);
        };
        if items.is_empty() {
            return Err(AppError::new(
                "INVALID_INPUT",
                "Выберите хотя бы одну книгу.",
            ));
        }
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        ensure_reader(&tx, reader)?;
        let mut copies = vec![];
        let mut loans = vec![];
        let mut quantity = 0;
        for item in &items {
            if item.mode == "COPY" {
                let copy = item.copy_id.as_deref().ok_or_else(|| {
                    AppError::new("INVALID_INPUT", "Для экземпляра нужен идентификатор.")
                })?;
                let changed=tx.execute("UPDATE book_copies SET status='ON_LOAN',updated_at=?1,version=version+1 WHERE id=?2 AND book_title_id=?3 AND deleted_at IS NULL AND status='AVAILABLE'",params![Self::now(),copy,item.title_id])?;
                if changed == 0 {
                    return Err(AppError::new(
                        "COPY_ISSUED",
                        "Экземпляр недоступен для выдачи.",
                    ));
                }
                let loan_id = item.loan_id.clone().ok_or_else(|| {
                    AppError::new("INVALID_INPUT", "Не задан идентификатор выдачи.")
                })?;
                tx.execute("INSERT INTO loans(id,reader_id,book_title_id,book_copy_id,accounting_mode,quantity,issued_at,status,created_terminal_id,updated_at) VALUES (?1,?2,?3,?4,'COPY',1,?5,'ACTIVE','terminal-local',?5)",params![loan_id,reader,item.title_id,copy,Self::now()])?;
                copies.push(copy.into());
                loans.push(loan_id);
                quantity += 1
            } else {
                let qty = item.quantity;
                let changed=tx.execute("UPDATE legacy_title_stock SET available_quantity=available_quantity-?1,version=version+1,updated_at=?2 WHERE book_title_id=?3 AND available_quantity>=?1",params![qty,Self::now(),item.title_id])?;
                if changed == 0 {
                    return Err(AppError::new(
                        "INSUFFICIENT_STOCK",
                        "В старом фонде недостаточно книг.",
                    ));
                }
                let loan_id = item.loan_id.clone().ok_or_else(|| {
                    AppError::new("INVALID_INPUT", "Не задан идентификатор выдачи.")
                })?;
                tx.execute("INSERT INTO loans(id,reader_id,book_title_id,accounting_mode,quantity,issued_at,status,created_terminal_id,updated_at) VALUES (?1,?2,?3,'LEGACY_TITLE',?4,?5,'ACTIVE','terminal-local',?5)",params![loan_id,reader,item.title_id,qty,Self::now()])?;
                loans.push(loan_id);
                quantity += qty
            }
        }
        let result = OperationResult {
            operation_id: operation_id.into(),
            operation_type: "issue".into(),
            quantity,
            title_id: None,
            copy_ids: copies,
            loan_ids: loans,
            reservation_id: None,
            queue_position: None,
        };
        Self::persist_operation(
            &tx,
            &result,
            &payload,
            &hash,
            "loan",
            result
                .loan_ids
                .first()
                .map(String::as_str)
                .unwrap_or("batch"),
            Some(reader),
        )?;
        tx.commit()?;
        Ok(result)
    }
    pub fn accept(
        &mut self,
        reader: &str,
        items: &[BasketItem],
        operation_id: &str,
    ) -> Result<OperationResult, AppError> {
        validate_basket(items)?;
        let payload = serde_json::to_string(&(reader, items, operation_id))?;
        let hash = payload_hash(&payload);
        if let Some(result) = self.existing_operation(operation_id, &hash, "ACCEPT")? {
            return Ok(result);
        };
        if items.is_empty() {
            return Err(AppError::new(
                "INVALID_INPUT",
                "Отметьте хотя бы одну книгу к приёму.",
            ));
        }
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        ensure_reader(&tx, reader)?;
        let mut copies = vec![];
        let mut loans = vec![];
        let mut quantity = 0;
        for item in items {
            if item.mode == "COPY" {
                let loan_id = item.loan_id.as_deref().ok_or_else(|| {
                    AppError::new("NO_LOAN", "Не найдена активная выдача экземпляра.")
                })?;
                let copy:Option<String>=tx.query_row("SELECT book_copy_id FROM loans WHERE id=?1 AND reader_id=?2 AND status='ACTIVE' AND accounting_mode='COPY' AND book_title_id=?3 AND book_copy_id=?4",params![loan_id,reader,item.title_id,item.copy_id],|r|r.get(0)).optional()?;
                let copy = copy.ok_or_else(|| {
                    AppError::new(
                        "WRONG_READER",
                        "Экземпляр не числится за выбранным читателем.",
                    )
                })?;
                tx.execute("UPDATE loans SET status='RETURNED',returned_at=?1,updated_at=?1,version=version+1 WHERE id=?2",params![Self::now(),loan_id])?;
                tx.execute("UPDATE book_copies SET status='AVAILABLE',updated_at=?1,version=version+1 WHERE id=?2",params![Self::now(),copy])?;
                copies.push(copy);
                loans.push(loan_id.into());
                quantity += 1
            } else {
                let loan_id = item
                    .loan_id
                    .as_deref()
                    .ok_or_else(|| AppError::new("NO_LOAN", "Не найдена выдача старого фонда."))?;
                let available:i32=tx.query_row("SELECT quantity FROM loans WHERE id=?1 AND reader_id=?2 AND status='ACTIVE' AND accounting_mode='LEGACY_TITLE' AND book_title_id=?3",params![loan_id,reader,item.title_id],|r|r.get(0)).optional()?.ok_or_else(||AppError::new("WRONG_READER","Издание не числится за выбранным читателем."))?;
                let qty = item.quantity;
                if qty > available {
                    return Err(AppError::new(
                        "EXCESS_QUANTITY",
                        "Нельзя принять больше книг, чем числится за читателем.",
                    ));
                }
                if qty == available {
                    tx.execute("UPDATE loans SET status='RETURNED',returned_at=?1,updated_at=?1,version=version+1 WHERE id=?2",params![Self::now(),loan_id])?;
                } else {
                    tx.execute("UPDATE loans SET quantity=quantity-?1,updated_at=?2,version=version+1 WHERE id=?3",params![qty,Self::now(),loan_id])?;
                }
                tx.execute("UPDATE legacy_title_stock SET available_quantity=available_quantity+?1,updated_at=?2,version=version+1 WHERE book_title_id=?3",params![qty,Self::now(),item.title_id])?;
                loans.push(loan_id.into());
                quantity += qty
            }
        }
        let result = OperationResult {
            operation_id: operation_id.into(),
            operation_type: "accept".into(),
            quantity,
            title_id: None,
            copy_ids: copies,
            loan_ids: loans,
            reservation_id: None,
            queue_position: None,
        };
        Self::persist_operation(
            &tx,
            &result,
            &payload,
            &hash,
            "loan",
            result
                .loan_ids
                .first()
                .map(String::as_str)
                .unwrap_or("batch"),
            Some(reader),
        )?;
        tx.commit()?;
        Ok(result)
    }
    pub fn reserve(
        &mut self,
        reader: &str,
        title: &str,
        operation_id: &str,
    ) -> Result<OperationResult, AppError> {
        let id = crate::security::operation_entity_id(operation_id, "reservation", 0);
        let payload = serde_json::to_string(
            &serde_json::json!({"reader_id":reader,"title_id":title,"reservation_id":id}),
        )?;
        let hash = payload_hash(&payload);
        if let Some(result) = self.existing_operation(operation_id, &hash, "RESERVE")? {
            return Ok(result);
        };
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        ensure_reader(&tx, reader)?;
        if tx
            .query_row(
                "SELECT id FROM book_titles WHERE id=?1 AND deleted_at IS NULL",
                params![title],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .is_none()
        {
            return Err(AppError::new("NOT_FOUND", "Издание не найдено."));
        }
        if tx.query_row("SELECT id FROM reservations WHERE reader_id=?1 AND book_title_id=?2 AND status='WAITING'",params![reader,title],|r|r.get::<_,String>(0)).optional()?.is_some(){return Err(AppError::new("DUPLICATE_RESERVATION","Читатель уже находится в очереди на это издание."));}
        let now = Self::now();
        tx.execute("INSERT INTO reservations(id,reader_id,book_title_id,created_at,status,updated_at) VALUES (?1,?2,?3,?4,'WAITING',?4)",params![id,reader,title,now])?;
        let position: i32 = tx.query_row(
            "SELECT COUNT(*) FROM reservations WHERE book_title_id=?1 AND status='WAITING'",
            params![title],
            |r| r.get(0),
        )?;
        let result = OperationResult {
            operation_id: operation_id.into(),
            operation_type: "reserve".into(),
            quantity: 0,
            title_id: Some(title.into()),
            copy_ids: vec![],
            loan_ids: vec![],
            reservation_id: Some(id.clone()),
            queue_position: Some(position),
        };
        Self::persist_operation(
            &tx,
            &result,
            &payload,
            &hash,
            "reservation",
            &id,
            Some(reader),
        )?;
        tx.commit()?;
        Ok(result)
    }
    pub fn register(
        &mut self,
        title: &Title,
        mode: &str,
        codes: &[String],
        quantity: i32,
        operation_id: &str,
    ) -> Result<OperationResult, AppError> {
        self.require_test_workspace()?;
        if !["COPY", "LEGACY_TITLE"].contains(&mode)
            || quantity > 10000
            || title.name.trim().is_empty()
            || title.author.trim().is_empty()
            || codes
                .iter()
                .any(|code| code.trim().is_empty() || code.len() > 256)
        {
            return Err(AppError::new(
                "INVALID_INPUT",
                "Проверьте название, автора, учёт и коды экземпляров.",
            ));
        }
        let payload = serde_json::to_string(&(title, mode, codes, quantity, operation_id))?;
        let hash = payload_hash(&payload);
        if let Some(result) = self.existing_operation(operation_id, &hash, "REGISTER")? {
            return Ok(result);
        };
        if quantity < 1 {
            return Err(AppError::new(
                "INVALID_INPUT",
                "Укажите количество поступивших книг.",
            ));
        }
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        let title_id = if title.id.trim().is_empty() {
            let id = Uuid::new_v4().to_string();
            tx.execute("INSERT INTO book_titles(id,school_id,isbn,title,authors,publisher,publication_year,language,subject,grade,updated_at) VALUES (?1,'school-demo',?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![id,title.isbn,title.name,title.author,title.publisher,title.year,title.language,title.subject,title.grade,Self::now()])?;
            id
        } else {
            title.id.clone()
        };
        let mut copy_ids = vec![];
        if mode == "COPY" {
            if codes.len() != quantity as usize {
                return Err(AppError::new(
                    "INVALID_INPUT",
                    "Количество кодов должно совпадать с количеством экземпляров.",
                ));
            }
            for code in codes {
                let id = Uuid::new_v4().to_string();
                tx.execute("INSERT INTO book_copies(id,book_title_id,inventory_number,barcode,status,updated_at) VALUES (?1,?2,?3,?3,'AVAILABLE',?4)",params![id,title_id,code,Self::now()]).map_err(|_|AppError::new("DUPLICATE_CODE","Этот инвентарный номер уже зарегистрирован."))?;
                copy_ids.push(id)
            }
        } else {
            tx.execute("INSERT INTO legacy_title_stock(book_title_id,total_quantity,available_quantity,updated_at) VALUES (?1,?2,?2,?3) ON CONFLICT(book_title_id) DO UPDATE SET total_quantity=total_quantity+excluded.total_quantity,available_quantity=available_quantity+excluded.available_quantity,updated_at=excluded.updated_at",params![title_id,quantity,Self::now()])?;
        }
        let result = OperationResult {
            operation_id: operation_id.into(),
            operation_type: "register".into(),
            quantity,
            title_id: Some(title_id.clone()),
            copy_ids,
            loan_ids: vec![],
            reservation_id: None,
            queue_position: None,
        };
        Self::persist_operation(&tx, &result, &payload, &hash, "title", &title_id, None)?;
        tx.commit()?;
        Ok(result)
    }
    /// Availability is calculated from operational state, never stored as a stale count.
    pub fn book_availability(&self, title_id: &str) -> Result<BookAvailability, AppError> {
        let copies: i32 = self.conn.query_row("SELECT COUNT(*) FROM book_copies WHERE book_title_id=?1 AND status='AVAILABLE' AND deleted_at IS NULL", params![title_id], |r| r.get(0))?;
        let legacy: i32 = self.conn.query_row("SELECT coalesce(available_quantity,0) FROM legacy_title_stock WHERE book_title_id=?1", params![title_id], |r| r.get(0)).optional()?.unwrap_or(0);
        let waiting: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM reservations WHERE book_title_id=?1 AND status='WAITING'",
            params![title_id],
            |r| r.get(0),
        )?;
        let expected = self.nearest_expected_return(title_id)?;
        Ok(BookAvailability {
            book_title_id: title_id.into(),
            available_copies: copies,
            available_legacy_quantity: legacy,
            waiting_reservations: waiting,
            nearest_expected_return: expected.nearest_due_at,
            has_overdue: expected.has_overdue,
        })
    }
    /// Only ACTIVE loans are considered. Missing due dates never invent a return date.
    pub fn nearest_expected_return(&self, title_id: &str) -> Result<ExpectedReturn, AppError> {
        let now = Utc::now().to_rfc3339();
        let nearest: Option<String> = self.conn.query_row("SELECT MIN(due_at) FROM loans WHERE book_title_id=?1 AND status='ACTIVE' AND due_at IS NOT NULL AND due_at>=?2", params![title_id,now], |r| r.get(0))?;
        let overdue: i32 = self.conn.query_row("SELECT COUNT(*) FROM loans WHERE book_title_id=?1 AND status='ACTIVE' AND due_at IS NOT NULL AND due_at<?2", params![title_id,now], |r| r.get(0))?;
        Ok(ExpectedReturn {
            nearest_due_at: nearest,
            has_overdue: overdue > 0,
        })
    }
    pub fn cancel_reservation(
        &mut self,
        reader: &str,
        reservation_id: &str,
        operation_id: &str,
    ) -> Result<OperationResult, AppError> {
        let payload = serde_json::to_string(
            &serde_json::json!({"reader_id":reader,"reservation_id":reservation_id,"action":"CANCEL"}),
        )?;
        let hash = payload_hash(&payload);
        if let Some(result) = self.existing_operation(operation_id, &hash, "RESERVE")? {
            return Ok(result);
        }
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        let title_id:String=tx.query_row("SELECT book_title_id FROM reservations WHERE id=?1 AND reader_id=?2 AND status='WAITING'",params![reservation_id,reader],|r|r.get(0)).optional()?.ok_or_else(||AppError::new("NOT_FOUND","Активная бронь не найдена."))?;
        tx.execute("UPDATE reservations SET status='CANCELLED',updated_at=?1,version=version+1 WHERE id=?2",params![Self::now(),reservation_id])?;
        let result = OperationResult {
            operation_id: operation_id.into(),
            operation_type: "reserve".into(),
            quantity: 0,
            title_id: Some(title_id),
            copy_ids: vec![],
            loan_ids: vec![],
            reservation_id: Some(reservation_id.into()),
            queue_position: None,
        };
        Self::persist_operation(
            &tx,
            &result,
            &payload,
            &hash,
            "reservation",
            reservation_id,
            Some(reader),
        )?;
        tx.commit()?;
        Ok(result)
    }
    pub fn operation_result(&self, id: &str) -> Result<Option<OperationResult>, AppError> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT result_json FROM library_operations WHERE operation_id=?1",
                params![id],
                |r| r.get(0),
            )
            .optional()?;
        value
            .map(|raw| {
                serde_json::from_str(&raw).map_err(|_| {
                    AppError::new("LOCAL_DATABASE_ERROR", "Повреждён сохранённый результат.")
                })
            })
            .transpose()
    }
    pub fn outbox_counts(&self) -> Result<(i64, i64), AppError> {
        Ok((
            self.conn.query_row(
                "SELECT COUNT(*) FROM sync_outbox WHERE status IN ('PENDING','SENDING','FAILED')",
                [],
                |r| r.get(0),
            )?,
            self.conn.query_row(
                "SELECT COUNT(*) FROM sync_conflicts WHERE status='OPEN'",
                [],
                |r| r.get(0),
            )?,
        ))
    }
    pub fn pending_outbox(&self) -> Result<Vec<PendingOutboxRecord>, AppError> {
        let mut st=self.conn.prepare("SELECT o.operation_id,o.operation_type,o.payload_json,o.payload_hash,l.occurred_at FROM sync_outbox o JOIN library_operations l ON l.operation_id=o.operation_id WHERE o.status IN ('PENDING','SENDING','FAILED') ORDER BY o.rowid")?;
        let rows = st.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
    pub fn mark_outbox_synced(&mut self, ids: &[String]) -> Result<(), AppError> {
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        for id in ids {
            tx.execute("UPDATE sync_outbox SET status='SYNCED',synced_at=?1,last_error_code=NULL,last_error_message=NULL WHERE operation_id=?2",params![Self::now(),id])?;
            tx.execute(
                "UPDATE library_operations SET status='SYNCED' WHERE operation_id=?1",
                params![id],
            )?;
        }
        tx.execute("INSERT INTO sync_state(key,value,updated_at) VALUES ('last_sync_at',?1,?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=excluded.updated_at",params![Self::now()])?;
        tx.commit()?;
        Ok(())
    }
    pub fn record_conflict(
        &mut self,
        operation_id: &str,
        reason: &str,
        local: &str,
        remote: Option<&str>,
    ) -> Result<(), AppError> {
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        tx.execute("UPDATE sync_outbox SET status='CONFLICT',last_error_code='CONFLICT',last_error_message=?1 WHERE operation_id=?2",params![reason,operation_id])?;
        tx.execute(
            "UPDATE library_operations SET status='CONFLICT' WHERE operation_id=?1",
            params![operation_id],
        )?;
        tx.execute("INSERT INTO sync_conflicts(id,operation_id,entity_type,entity_id,reason,local_payload_json,remote_payload_json,status,created_at) VALUES (?1,?2,'operation',?2,?3,?4,?5,'OPEN',?6)",params![Uuid::new_v4().to_string(),operation_id,reason,local,remote,Self::now()])?;
        tx.commit()?;
        Ok(())
    }
    pub fn last_sync_at(&self) -> Result<Option<String>, AppError> {
        self.conn
            .query_row(
                "SELECT value FROM sync_state WHERE key='last_sync_at'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(Into::into)
    }
    /// The outbox retains the last actionable error; callers must never infer
    /// success from network availability alone.
    pub fn last_sync_error(&self) -> Result<Option<String>, AppError> {
        self.conn
            .query_row(
                "SELECT value FROM sync_state WHERE key='last_error' AND value<>''",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }
    pub fn set_sync_error(&self, code: Option<&str>) -> Result<(), AppError> {
        self.conn.execute("INSERT INTO sync_state(key,value,updated_at) VALUES('last_error',?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=excluded.updated_at",params![code.unwrap_or(""),Self::now()])?;
        Ok(())
    }
}
fn title_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Title> {
    Ok(Title {
        id: row.get(0)?,
        name: row.get(1)?,
        author: row.get(2)?,
        isbn: row.get(3)?,
        publisher: row.get(4)?,
        year: row.get(5)?,
        language: row.get(6)?,
        subject: row.get(7)?,
        grade: row.get(8)?,
    })
}
fn copy_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Copy> {
    Ok(Copy {
        id: row.get(0)?,
        title_id: row.get(1)?,
        code: row.get(2)?,
        status: row.get(3)?,
        location: row.get(4)?,
    })
}
fn loan_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Loan> {
    Ok(Loan {
        id: row.get(0)?,
        reader_id: row.get(1)?,
        title_id: row.get(2)?,
        copy_id: row.get(3)?,
        quantity: row.get(4)?,
        mode: row.get(5)?,
        due_date: row.get(6)?,
    })
}
fn reservation_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Reservation> {
    Ok(Reservation {
        id: row.get(0)?,
        reader_id: row.get(1)?,
        title_id: row.get(2)?,
        created_at: row.get(3)?,
        status: row.get(4)?,
    })
}
fn ensure_reader(tx: &Transaction<'_>, reader: &str) -> Result<(), AppError> {
    if tx
        .query_row(
            "SELECT id FROM persons WHERE id=?1 AND status='ACTIVE' AND deleted_at IS NULL",
            params![reader],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .is_none()
    {
        return Err(AppError::new(
            "NOT_FOUND",
            "Читатель не найден или неактивен.",
        ));
    }
    Ok(())
}

impl LocalDatabase {
    pub fn test_readers(&self) -> Result<Vec<TestReaderRecord>, AppError> {
        let mut statement = self.conn.prepare(
            "SELECT p.id,t.external_id,p.full_name,p.person_type,p.class_name,p.position_name,p.status,
                    EXISTS(SELECT 1 FROM cards c WHERE c.person_id=p.id AND c.status='ACTIVE')
             FROM terminal_test_readers t JOIN persons p ON p.id=t.person_id
             WHERE p.deleted_at IS NULL ORDER BY p.full_name, t.external_id",
        )?;
        let rows = statement
            .query_map([], test_reader_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn create_test_reader(
        &mut self,
        input: &TestReaderInput,
        update_existing: bool,
    ) -> Result<TestReaderUpsert, AppError> {
        self.require_test_workspace()?;
        validate_test_reader(input)?;
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        let existing = tx
            .query_row(
                "SELECT person_id FROM terminal_test_readers WHERE external_id=?1",
                params![input.external_id.trim()],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if existing.is_some() && !update_existing {
            return Err(AppError::new(
                "TEST_READER_EXISTS",
                "Читатель с этим external_id уже существует. Выберите обновление записи.",
            ));
        }
        let action = upsert_test_reader_tx(&tx, input, existing.as_deref())?;
        let id = match existing {
            Some(id) => id,
            None => tx.query_row(
                "SELECT person_id FROM terminal_test_readers WHERE external_id=?1",
                params![input.external_id.trim()],
                |row| row.get::<_, String>(0),
            )?,
        };
        let reader = test_reader_tx(&tx, &id)?;
        tx.commit()?;
        Ok(TestReaderUpsert { action, reader })
    }

    pub fn preview_test_readers_csv(&self, content: &str) -> Result<TestCsvPreview, AppError> {
        let rows = parse_test_csv(content)?;
        let mut preview_rows = Vec::with_capacity(rows.len());
        let mut create_count = 0;
        let mut update_count = 0;
        let mut skipped_count = 0;
        let mut error_count = 0;
        let mut seen = std::collections::HashSet::new();
        for (line, input) in rows {
            let duplicate_in_file = !seen.insert(input.external_id.trim().to_owned());
            let validation = validate_test_reader(&input);
            let existing = if validation.is_ok() && !duplicate_in_file {
                self.conn
                    .query_row(
                        "SELECT person_id FROM terminal_test_readers WHERE external_id=?1",
                        params![input.external_id.trim()],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()?
            } else {
                None
            };
            let (result, message): (&str, String) = if duplicate_in_file {
                error_count += 1;
                skipped_count += 1;
                ("ERROR", "Повторный external_id в этом CSV".into())
            } else if let Err(error) = validation {
                error_count += 1;
                skipped_count += 1;
                ("ERROR", error.message)
            } else if existing.is_some() {
                update_count += 1;
                (
                    "UPDATE",
                    "Будет обновлена существующая тестовая запись".into(),
                )
            } else {
                create_count += 1;
                ("CREATE", "Будет создан тестовый читатель".into())
            };
            preview_rows.push(TestCsvPreviewRow {
                line,
                external_id: input.external_id,
                full_name: input.full_name,
                status: input.status,
                result: result.into(),
                message,
            });
        }
        Ok(TestCsvPreview {
            valid_count: create_count + update_count,
            create_count,
            update_count,
            skipped_count,
            error_count,
            rows: preview_rows,
        })
    }

    pub fn apply_test_readers_csv(&mut self, content: &str) -> Result<TestCsvPreview, AppError> {
        self.require_test_workspace()?;
        let preview = self.preview_test_readers_csv(content)?;
        if preview.error_count > 0 {
            return Err(AppError::new(
                "INVALID_TEST_CSV",
                "CSV содержит ошибки. Импорт не выполнен; тестовые данные не изменены.",
            ));
        }
        let rows = parse_test_csv(content)?;
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        for (_, input) in rows {
            let existing = tx
                .query_row(
                    "SELECT person_id FROM terminal_test_readers WHERE external_id=?1",
                    params![input.external_id.trim()],
                    |r| r.get::<_, String>(0),
                )
                .optional()?;
            upsert_test_reader_tx(&tx, &input, existing.as_deref())?;
        }
        tx.commit()?;
        Ok(preview)
    }

    pub fn bind_test_reader_card(
        &mut self,
        reader_id: &str,
        raw_card: &str,
        secret: &str,
    ) -> Result<(), AppError> {
        self.require_test_workspace()?;
        let hash = card_lookup_hash(raw_card, secret)?;
        self.bind_test_reader_card_hash(reader_id, &hash)
    }

    pub(crate) fn bind_test_reader_card_hash(
        &mut self,
        reader_id: &str,
        hash: &str,
    ) -> Result<(), AppError> {
        self.require_test_workspace()?;
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        let is_test: Option<String> = tx
            .query_row(
                "SELECT t.person_id FROM terminal_test_readers t JOIN persons p ON p.id=t.person_id WHERE t.person_id=?1 AND p.status='ACTIVE' AND p.deleted_at IS NULL",
                params![reader_id],
                |row| row.get(0),
            )
            .optional()?;
        if is_test.is_none() {
            return Err(AppError::new(
                "NOT_FOUND",
                "Активный тестовый читатель не найден. Проверьте его статус.",
            ));
        }
        let owner: Option<String> = tx
            .query_row(
                "SELECT person_id FROM cards WHERE card_lookup_hash=?1",
                params![hash],
                |row| row.get(0),
            )
            .optional()?;
        if owner.as_deref().is_some_and(|id| id != reader_id) {
            return Err(AppError::new(
                "CARD_ALREADY_BOUND",
                "Эта карта уже привязана к другому тестовому читателю.",
            ));
        }
        tx.execute("UPDATE cards SET status='REVOKED',updated_at=?1 WHERE person_id=?2 AND status='ACTIVE'", params![Self::now(), reader_id])?;
        if owner.is_none() {
            tx.execute("INSERT INTO cards(id,person_id,card_lookup_hash,status,updated_at) VALUES(?1,?2,?3,'ACTIVE',?4)", params![Uuid::new_v4().to_string(),reader_id,hash,Self::now()])?;
        } else {
            tx.execute(
                "UPDATE cards SET status='ACTIVE',updated_at=?1 WHERE card_lookup_hash=?2",
                params![Self::now(), hash],
            )?;
        }
        let committed_owner: String = tx.query_row(
            "SELECT person_id FROM cards WHERE card_lookup_hash=?1 AND status='ACTIVE'",
            params![hash],
            |r| r.get(0),
        )?;
        if committed_owner != reader_id {
            return Err(AppError::new(
                "CARD_BIND_VERIFY_FAILED",
                "Не удалось проверить привязку карты.",
            ));
        }
        tx.commit()?;
        if self.reader_by_card_hash(hash)?.id != reader_id {
            return Err(AppError::new(
                "CARD_BIND_VERIFY_FAILED",
                "Проверка сохранённой карты не пройдена.",
            ));
        }
        Ok(())
    }

    pub(crate) fn card_hash_exists(&self, hash: &str) -> Result<bool, AppError> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM cards WHERE card_lookup_hash=?1 AND status='ACTIVE')",
            params![hash],
            |r| r.get(0),
        )?)
    }

    pub fn test_books(&self) -> Result<Vec<TestBookRecord>, AppError> {
        let mut statement = self.conn.prepare("SELECT c.id,c.book_title_id,t.title,c.inventory_number,c.barcode,t.isbn,c.status FROM book_copies c JOIN book_titles t ON t.id=c.book_title_id ORDER BY t.title,c.inventory_number")?;
        let rows = statement
            .query_map([], |row| {
                Ok(TestBookRecord {
                    id: row.get(0)?,
                    title_id: row.get(1)?,
                    title: row.get(2)?,
                    inventory_number: row.get(3)?,
                    barcode: row.get(4)?,
                    isbn: row.get(5)?,
                    status: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn bind_test_book_code(&mut self, copy_id: &str, code: &str) -> Result<(), AppError> {
        self.require_test_workspace()?;
        let code = code.trim();
        if code.is_empty() {
            return Err(AppError::new(
                "INVALID_INPUT",
                "Не получен сканерный код книги.",
            ));
        }
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM book_copies WHERE barcode=?1 AND id<>?2",
                params![code, copy_id],
                |r| r.get(0),
            )
            .optional()?;
        if existing.is_some() {
            return Err(AppError::new(
                "BOOK_CODE_ALREADY_BOUND",
                "Этот сканерный код уже привязан к другому экземпляру.",
            ));
        }
        if self.conn.execute(
            "UPDATE book_copies SET barcode=?1,updated_at=?2 WHERE id=?3",
            params![code, Self::now(), copy_id],
        )? == 0
        {
            return Err(AppError::new("NOT_FOUND", "Экземпляр не найден."));
        }
        Ok(())
    }

    pub fn delete_test_reader(&mut self, reader_id: &str) -> Result<(), AppError> {
        self.require_test_workspace()?;
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        let exists: Option<String> = tx
            .query_row(
                "SELECT person_id FROM terminal_test_readers WHERE person_id=?1",
                params![reader_id],
                |r| r.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(AppError::new("NOT_FOUND", "Тестовый читатель не найден."));
        }
        tx.execute(
            "DELETE FROM reservations WHERE reader_id=?1",
            params![reader_id],
        )?;
        tx.execute("DELETE FROM loans WHERE reader_id=?1", params![reader_id])?;
        tx.execute("DELETE FROM cards WHERE person_id=?1", params![reader_id])?;
        tx.execute(
            "DELETE FROM terminal_test_readers WHERE person_id=?1",
            params![reader_id],
        )?;
        tx.execute("DELETE FROM persons WHERE id=?1", params![reader_id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn seed_test_readers(&mut self) -> Result<Vec<TestReaderRecord>, AppError> {
        self.require_test_workspace()?;
        for input in [
            test_input(
                "UAT-STUDENT-001",
                "Тестовый ученик Айдана",
                "STUDENT",
                Some("6 «А» класс"),
                None,
                "ACTIVE",
            ),
            test_input(
                "UAT-STUDENT-002",
                "Тестовый ученик Бекзат",
                "STUDENT",
                Some("7 «Б» класс"),
                None,
                "ACTIVE",
            ),
            test_input(
                "UAT-STUDENT-003",
                "Тестовый ученик Дана",
                "STUDENT",
                Some("8 «В» класс"),
                None,
                "INACTIVE",
            ),
            test_input(
                "UAT-TEACHER-001",
                "Тестовый учитель Ерлан",
                "TEACHER",
                None,
                Some("Учитель литературы"),
                "ACTIVE",
            ),
        ] {
            self.create_test_reader(&input, true)?;
        }
        self.test_readers()
    }

    pub fn reset_test_data(&mut self) -> Result<(), AppError> {
        self.require_test_workspace()?;
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        // Workspace isolation is an object invariant, never a caller-provided SQL scope.
        // Delete dependants first. A failure restores the entire test workspace.
        for table in [
            "sync_outbox",
            "conflict_projections",
            "sync_conflicts",
            "library_operations",
            "reservations",
            "loans",
            "cards",
            "face_templates",
            "terminal_test_readers",
            "persons",
            "book_copies",
            "legacy_title_stock",
            "book_titles",
            "library_locations",
            "classes",
            "cloud_entity_versions",
            "sync_state",
        ] {
            tx.execute(&format!("DELETE FROM {table}"), [])?;
        }
        tx.commit()?;
        Ok(())
    }
}

fn test_input(
    external_id: &str,
    full_name: &str,
    person_type: &str,
    class_name: Option<&str>,
    position_name: Option<&str>,
    status: &str,
) -> TestReaderInput {
    TestReaderInput {
        external_id: external_id.into(),
        full_name: full_name.into(),
        person_type: person_type.into(),
        class_name: class_name.map(str::to_owned),
        position_name: position_name.map(str::to_owned),
        status: status.into(),
    }
}

fn validate_test_reader(input: &TestReaderInput) -> Result<(), AppError> {
    if input.external_id.trim().is_empty() || input.full_name.trim().is_empty() {
        return Err(AppError::new(
            "INVALID_INPUT",
            "Заполните external_id и ФИО.",
        ));
    }
    if !matches!(input.person_type.as_str(), "STUDENT" | "TEACHER" | "STAFF") {
        return Err(AppError::new(
            "INVALID_INPUT",
            "Выберите допустимый тип читателя.",
        ));
    }
    if !matches!(input.status.as_str(), "ACTIVE" | "INACTIVE") {
        return Err(AppError::new("INVALID_INPUT", "Выберите статус читателя."));
    }
    if input.person_type == "STUDENT"
        && input
            .class_name
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(AppError::new("INVALID_INPUT", "Для ученика укажите класс."));
    }
    Ok(())
}

fn upsert_test_reader_tx(
    tx: &Transaction<'_>,
    input: &TestReaderInput,
    existing: Option<&str>,
) -> Result<String, AppError> {
    let now = LocalDatabase::now();
    let class_name = input
        .class_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let class_id = if let Some(name) = class_name {
        let found = tx
            .query_row(
                "SELECT id FROM classes WHERE school_id='school-demo' AND name=?1",
                params![name],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        Some(match found {
            Some(id) => id,
            None => {
                let id = Uuid::new_v4().to_string();
                tx.execute("INSERT INTO classes(id,school_id,name,updated_at) VALUES(?1,'school-demo',?2,?3)",params![id,name,now])?;
                id
            }
        })
    } else {
        None
    };
    let id = existing
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    if existing.is_some() {
        tx.execute("UPDATE persons SET full_name=?1,person_type=?2,class_id=?3,class_name=?4,position_name=?5,status=?6,updated_at=?7 WHERE id=?8",params![input.full_name.trim(),input.person_type,class_id,class_name,input.position_name.as_deref().map(str::trim).filter(|v|!v.is_empty()),input.status,now,id])?;
        tx.execute(
            "UPDATE terminal_test_readers SET updated_at=?1 WHERE person_id=?2",
            params![now, id],
        )?;
        Ok("UPDATED".into())
    } else {
        tx.execute("INSERT INTO persons(id,school_id,full_name,person_type,class_id,class_name,position_name,status,updated_at) VALUES(?1,'school-demo',?2,?3,?4,?5,?6,?7,?8)",params![id,input.full_name.trim(),input.person_type,class_id,class_name,input.position_name.as_deref().map(str::trim).filter(|v|!v.is_empty()),input.status,now])?;
        tx.execute("INSERT INTO terminal_test_readers(person_id,external_id,created_at,updated_at) VALUES(?1,?2,?3,?3)",params![id,input.external_id.trim(),now])?;
        Ok("CREATED".into())
    }
}

fn test_reader_tx(tx: &Transaction<'_>, id: &str) -> Result<TestReaderRecord, AppError> {
    tx.query_row("SELECT p.id,t.external_id,p.full_name,p.person_type,p.class_name,p.position_name,p.status,EXISTS(SELECT 1 FROM cards c WHERE c.person_id=p.id AND c.status='ACTIVE') FROM terminal_test_readers t JOIN persons p ON p.id=t.person_id WHERE p.id=?1",params![id],test_reader_from_row).map_err(Into::into)
}

fn test_reader_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TestReaderRecord> {
    Ok(TestReaderRecord {
        id: row.get(0)?,
        external_id: row.get(1)?,
        full_name: row.get(2)?,
        person_type: row.get(3)?,
        class_name: row.get(4)?,
        position_name: row.get(5)?,
        status: row.get(6)?,
        card_bound: row.get(7)?,
    })
}

fn parse_test_csv(content: &str) -> Result<Vec<(usize, TestReaderInput)>, AppError> {
    let mut lines = content.trim_start_matches('\u{feff}').lines();
    let header = lines.next().unwrap_or("").trim();
    if header != "external_id,full_name,person_type,class_name,position_name,status" {
        return Err(AppError::new("INVALID_TEST_CSV", "CSV должен иметь заголовок external_id,full_name,person_type,class_name,position_name,status."));
    }
    let mut result = Vec::new();
    for (offset, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = csv_fields(line).ok_or_else(|| {
            AppError::new(
                "INVALID_TEST_CSV",
                format!("Строка {} содержит некорректные кавычки CSV.", offset + 2),
            )
        })?;
        if fields.len() != 6 {
            return Err(AppError::new(
                "INVALID_TEST_CSV",
                format!("В строке {} должно быть 6 полей.", offset + 2),
            ));
        }
        result.push((
            offset + 2,
            TestReaderInput {
                external_id: fields[0].to_owned(),
                full_name: fields[1].to_owned(),
                person_type: fields[2].to_owned(),
                class_name: nonempty(&fields[3]),
                position_name: nonempty(&fields[4]),
                status: fields[5].to_owned(),
            },
        ));
    }
    if result.is_empty() {
        return Err(AppError::new(
            "INVALID_TEST_CSV",
            "CSV не содержит строк для импорта.",
        ));
    }
    Ok(result)
}

fn nonempty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.trim().to_owned())
}

fn csv_fields(line: &str) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    let mut value = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                value.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                fields.push(value.trim().to_owned());
                value.clear();
            }
            _ => value.push(ch),
        }
    }
    (!quoted).then(|| {
        fields.push(value.trim().to_owned());
        fields
    })
}

#[cfg(test)]
mod test_reader_tests {
    use super::*;
    use crate::security::CredentialStore;

    fn database() -> (tempfile::TempDir, LocalDatabase) {
        let directory = tempfile::tempdir().expect("temporary UAT db");
        let database = LocalDatabase::open(
            directory.path().join("uat.db"),
            &CredentialStore::test_fixture(),
        )
        .expect("open encrypted UAT db");
        (directory, database)
    }

    fn student(external_id: &str, name: &str) -> TestReaderInput {
        test_input(
            external_id,
            name,
            "STUDENT",
            Some("9 «А» класс"),
            None,
            "ACTIVE",
        )
    }

    #[test]
    fn student_requires_class_but_teacher_can_have_a_position() {
        assert!(
            validate_test_reader(&test_input("S-1", "Тест", "STUDENT", None, None, "ACTIVE"))
                .is_err()
        );
        assert!(validate_test_reader(&test_input(
            "T-1",
            "Тест",
            "TEACHER",
            None,
            Some("Учитель"),
            "ACTIVE"
        ))
        .is_ok());
    }

    #[test]
    fn external_id_is_unique_but_equal_names_are_allowed() {
        let (_directory, mut db) = database();
        db.create_test_reader(&student("UAT-1", "Тестовый читатель"), false)
            .unwrap();
        assert!(
            matches!(db.create_test_reader(&student("UAT-1", "Другой"), false), Err(error) if error.code == "TEST_READER_EXISTS")
        );
        db.create_test_reader(&student("UAT-2", "Тестовый читатель"), false)
            .unwrap();
        assert_eq!(db.test_readers().unwrap().len(), 2);
    }

    #[test]
    fn csv_preview_reports_errors_and_invalid_import_is_atomic() {
        let (_directory, mut db) = database();
        let csv = "external_id,full_name,person_type,class_name,position_name,status\nUAT-CSV-1,Тест CSV,STUDENT,7 А,,ACTIVE\nUAT-CSV-2,Без класса,STUDENT,,,ACTIVE\n";
        let preview = db.preview_test_readers_csv(csv).unwrap();
        assert_eq!(preview.create_count, 1);
        assert_eq!(preview.error_count, 1);
        assert!(
            matches!(db.apply_test_readers_csv(csv), Err(error) if error.code == "INVALID_TEST_CSV")
        );
        assert!(db.test_readers().unwrap().is_empty());
    }

    #[test]
    fn card_binding_persists_a_hash_and_rejects_second_owner_without_outbox() {
        let (_directory, mut db) = database();
        let first = db
            .create_test_reader(&student("UAT-CARD-1", "Карта один"), false)
            .unwrap()
            .reader;
        let second = db
            .create_test_reader(&student("UAT-CARD-2", "Карта два"), false)
            .unwrap()
            .reader;
        db.bind_test_reader_card(&first.id, " raw-card-123 ", "test-secret")
            .unwrap();
        let stored: String = db
            .conn
            .query_row(
                "SELECT card_lookup_hash FROM cards WHERE person_id=?1",
                params![first.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            stored,
            card_lookup_hash("RAW-CARD-123", "test-secret").unwrap()
        );
        assert_ne!(stored, "RAW-CARD-123");
        assert!(
            matches!(db.bind_test_reader_card(&second.id, "RAW-CARD-123", "test-secret"), Err(error) if error.code == "CARD_ALREADY_BOUND")
        );
        assert_eq!(db.outbox_counts().unwrap().0, 0);
    }

    #[test]
    fn reset_removes_entire_test_dataset_without_dropping_schema() {
        let (_directory, mut db) = database();
        let record = db
            .create_test_reader(&student("UAT-RESET", "Сброс"), false)
            .unwrap()
            .reader;
        db.bind_test_reader_card(&record.id, "UAT-RESET-CARD", "test-secret")
            .unwrap();
        db.reset_test_data().unwrap();
        assert!(db.test_readers().unwrap().is_empty());
        assert!(db.readers("").unwrap().is_empty());
        assert!(db.titles("").unwrap().is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::LocalDatabase;
    use crate::security::CredentialStore;
    use rusqlite::Connection;
    use std::{fs, path::PathBuf};

    #[test]
    fn sqlcipher_database_rejects_unkeyed_access_and_reopens_with_credential_manager_key() {
        let supplied_path = std::env::var_os("EDUS_ENCRYPTION_PROOF_PATH").map(PathBuf::from);
        let temporary = tempfile::tempdir().expect("create temporary encrypted database directory");
        let path = supplied_path.unwrap_or_else(|| temporary.path().join("library-proof.db"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create proof database directory");
        }
        for suffix in ["", "-wal", "-shm"] {
            let candidate = PathBuf::from(format!("{}{}", path.display(), suffix));
            let _ = fs::remove_file(candidate);
        }

        let credentials = CredentialStore::test_fixture();
        let database = LocalDatabase::open(path.clone(), &credentials)
            .expect("open encrypted database with isolated test key");
        assert!(database
            .cipher_version()
            .expect("read cipher version")
            .starts_with("4.18.0"));
        assert_eq!(
            database
                .cipher_memory_hardening()
                .expect("read cipher memory security state"),
            "1"
        );
        drop(database);

        let unkeyed = Connection::open(&path).expect("open database file without a key");
        let unkeyed_read: rusqlite::Result<i64> =
            unkeyed.query_row("SELECT COUNT(*) FROM sqlite_schema", [], |row| row.get(0));
        assert!(
            unkeyed_read.is_err(),
            "an unkeyed connection must not read SQLCipher data"
        );
        drop(unkeyed);

        let reopened = LocalDatabase::open(path, &credentials)
            .expect("reopen encrypted database with the same isolated test key");
        assert!(
            reopened
                .snapshot(false)
                .expect("read reopened database")
                .readers
                .len()
                >= 3
        );
    }

    #[test]
    fn protected_admin_audit_is_persisted_without_credentials() {
        let temporary = tempfile::Builder::new()
            .prefix("edus-admin-audit-")
            .tempdir_in("E:\\Codex\\temp")
            .expect("temporary E: database");
        let database = LocalDatabase::open(
            temporary.path().join("admin-audit.db"),
            &CredentialStore::test_fixture(),
        )
        .expect("open encrypted database");

        database
            .record_admin_audit("ADMIN_ACCESS_DENIED", "WINDOWS_AUTH_DENIED")
            .expect("record non-secret audit event");
        database
            .checkpoint_for_maintenance()
            .expect("checkpoint encrypted WAL");
        let (event_type, reason_code): (String, String) = database
            .conn
            .query_row(
                "SELECT event_type, reason_code FROM admin_audit_events ORDER BY occurred_at DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read audit event");
        assert_eq!(event_type, "ADMIN_ACCESS_DENIED");
        assert_eq!(reason_code, "WINDOWS_AUTH_DENIED");
        let schema: String = database
            .conn
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE type='table' AND name='admin_audit_events'",
                [],
                |row| row.get(0),
            )
            .expect("read audit schema");
        assert!(!schema.to_ascii_lowercase().contains("password"));
        assert!(!schema.to_ascii_lowercase().contains("username"));
    }

    #[test]
    fn cloud_projection_bootstrap_delta_and_pending_guard_are_atomic() {
        use serde_json::json;
        let temporary = tempfile::Builder::new()
            .prefix("edus-projection-")
            .tempdir_in("E:\\Codex\\temp")
            .expect("temporary E: database");
        let mut database = LocalDatabase::open(
            temporary.path().join("projection.db"),
            &CredentialStore::test_fixture(),
        )
        .expect("open database");
        let mut snapshot = json!({"school":{"id":"cloud-school","name":"Cloud school"},"next_cursor":"cursor-0","classes":[{"id":"class-1","name":"7 A","version":1}],"persons":[{"id":"reader-1","full_name":"Тестовый читатель","person_type":"STUDENT","class_id":"class-1","class_name":"7 A","status":"ACTIVE","version":1,"updated_at":"2026-09-23T00:00:00Z","deleted_at":null}],"cards":[{"id":"card-1","person_id":"reader-1","card_lookup_hash":"test-card-hash","status":"ACTIVE","version":1,"updated_at":"2026-09-23T00:00:00Z"}],"face_templates":[],"locations":[{"id":"location-1","name":"Фонд","code":"MAIN","version":1,"updated_at":"2026-09-23T00:00:00Z"}],"book_titles":[{"id":"title-1","isbn":"9780000000001","title":"Тестовая книга","authors":"Автор","language":"ru","version":1,"updated_at":"2026-09-23T00:00:00Z","deleted_at":null}],"book_copies":[{"id":"copy-1","book_title_id":"title-1","inventory_number":"T-1","barcode":"B-1","status":"ON_LOAN","location_id":"location-1","version":1,"updated_at":"2026-09-23T00:00:00Z","deleted_at":null}],"legacy_title_stock":[{"book_title_id":"title-1","total_quantity":4,"available_quantity":3,"version":1,"updated_at":"2026-09-23T00:00:00Z"}],"active_loans":[{"id":"loan-1","reader_id":"reader-1","book_title_id":"title-1","book_copy_id":"copy-1","accounting_mode":"COPY","quantity":1,"returned_quantity":0,"issued_at":"2026-09-01T00:00:00Z","status":"ACTIVE","version":1,"updated_at":"2026-09-23T00:00:00Z"}],"active_reservations":[]});
        for key in [
            "classes",
            "persons",
            "cards",
            "locations",
            "book_titles",
            "book_copies",
            "legacy_title_stock",
            "active_loans",
        ] {
            for item in snapshot[key].as_array_mut().expect("fixture array") {
                item["school_id"] = json!("cloud-school");
                item["updated_at"] = json!("2026-09-23T00:00:00Z");
                if key == "legacy_title_stock" {
                    item["id"] = item["book_title_id"].clone();
                }
                if key == "active_loans" {
                    item["created_terminal_id"] = json!("cloud-terminal");
                }
            }
        }
        database
            .apply_cloud_bootstrap(&snapshot, "cloud-projection", "1")
            .expect("bootstrap");
        assert!(database
            .cloud_bootstrap_completed()
            .expect("bootstrap state"));
        database.apply_cloud_delta_batch("cloud-school",&[json!({"entity_type":"BOOK_COPY_UPSERT","entity_id":"copy-1","version":2,"operation":"UPSERT","payload_kind":"FULL","payload_schema_version":1,"payload":{"id":"copy-1","school_id":"cloud-school","book_title_id":"title-1","location_id":"location-1","inventory_number":"T-1","barcode":"B-1","status":"AVAILABLE","version":2,"updated_at":"2026-09-23T00:01:00Z","deleted_at":null}})],"cursor-1").expect("delta");
        assert_eq!(
            database
                .conn
                .query_row(
                    "SELECT status FROM book_copies WHERE id='copy-1'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .expect("copy"),
            "AVAILABLE"
        );
        database.conn.execute("INSERT INTO library_operations(id,operation_id,type,entity_type,entity_id,terminal_id,occurred_at,status,payload_hash,payload_json,result_json,created_at) VALUES('op-row','op-1','ISSUE','BOOK_COPY','copy-1','terminal-local','2026-09-23T00:00:00Z','COMMITTED','hash','{\"copyId\":\"copy-1\"}','{}','2026-09-23T00:00:00Z')",[]).expect("operation");
        database.conn.execute("INSERT INTO sync_outbox(id,operation_id,operation_type,payload_json,payload_hash,created_at,status) VALUES('outbox-1','op-1','ISSUE','{\"items\":[{\"copyId\":\"copy-1\"}]}','hash','2026-09-23T00:00:00Z','PENDING')",[]).expect("outbox");
        let blocked=database.apply_cloud_delta_batch("cloud-school",&[json!({"entity_type":"BOOK_COPY_UPSERT","entity_id":"copy-1","version":3,"operation":"UPSERT","payload_kind":"FULL","payload_schema_version":1,"payload":{"id":"copy-1","school_id":"cloud-school","book_title_id":"title-1","location_id":"location-1","inventory_number":"T-1","barcode":"B-1","status":"ON_LOAN","version":3,"updated_at":"2026-09-23T00:02:00Z","deleted_at":null}})],"cursor-2");
        assert!(
            matches!(blocked,Err(crate::domain::AppError{code,..}) if code=="PENDING_LOCAL_MUTATION")
        );
        assert_eq!(
            database.cloud_cursor().expect("cursor"),
            Some("cursor-1".into())
        );
    }
}

impl LocalDatabase {
    pub fn has_pending_cloud_impact(&self, entity_id: &str) -> Result<bool, AppError> {
        let mut statement = self.conn.prepare(
            "SELECT payload_json FROM sync_outbox WHERE status IN ('PENDING','SENDING','FAILED')",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        for payload in rows {
            if let Ok(value) = serde_json::from_str::<Value>(&payload?) {
                if projection_value_contains(&value, entity_id) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn reconcile_cloud_ack(&mut self, operation_id: &str) -> Result<(), AppError> {
        self.mark_outbox_synced(&[operation_id.to_owned()])
    }

    pub fn reconcile_cloud_conflict(
        &mut self,
        operation_id: &str,
        reason: &str,
        local: &str,
        remote: &str,
    ) -> Result<(), AppError> {
        let value: Value = serde_json::from_str(local).map_err(|_| {
            AppError::new(
                "CLOUD_PROTOCOL_ERROR",
                "Локальный payload конфликта повреждён.",
            )
        })?;
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        tx.execute("UPDATE sync_outbox SET status='CONFLICT',last_error_code=?1,last_error_message=?1 WHERE operation_id=?2",params![reason,operation_id])?;
        tx.execute(
            "UPDATE library_operations SET status='CONFLICT' WHERE operation_id=?1",
            params![operation_id],
        )?;
        tx.execute("INSERT INTO sync_conflicts(id,operation_id,entity_type,entity_id,reason,local_payload_json,remote_payload_json,status,created_at) SELECT ?1,?2,'operation',?2,?3,?4,?5,'OPEN',?6 WHERE NOT EXISTS(SELECT 1 FROM sync_conflicts WHERE operation_id=?2 AND status='OPEN')",params![Uuid::new_v4().to_string(),operation_id,reason,local,remote,Self::now()])?;
        let stored: Option<String> = tx
            .query_row(
                "SELECT result_json FROM library_operations WHERE operation_id=?1 AND type='ISSUE'",
                params![operation_id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(stored) = stored {
            let result: OperationResult = serde_json::from_str(&stored)?;
            for loan_id in result.loan_ids {
                tx.execute(
                    "UPDATE loans SET status='CONFLICT',updated_at=?1 WHERE id=?2",
                    params![Self::now(), loan_id],
                )?;
            }
        }
        let reservation: Option<String> = tx.query_row("SELECT result_json FROM library_operations WHERE operation_id=?1 AND type='RESERVE'", params![operation_id], |row| row.get(0)).optional()?;
        if let Some(stored) = reservation {
            let result: OperationResult = serde_json::from_str(&stored)?;
            if let Some(id) = result.reservation_id {
                if value.get("action").and_then(Value::as_str) != Some("CANCEL") {
                    tx.execute(
                        "UPDATE reservations SET status='CANCELLED',updated_at=?1 WHERE id=?2",
                        params![Self::now(), id],
                    )?;
                }
            }
        }
        let mut copy_ids = Vec::new();
        projection_collect_copy_ids(&value, &mut copy_ids);
        for copy_id in copy_ids {
            tx.execute(
                "UPDATE book_copies SET status='VERIFYING',updated_at=?1 WHERE id=?2",
                params![Self::now(), copy_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn apply_conflict_projections(&mut self) -> Result<(), AppError> {
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        let pending: i64 = tx.query_row(
            "SELECT COUNT(*) FROM sync_outbox WHERE status IN ('PENDING','SENDING','FAILED')",
            [],
            |r| r.get(0),
        )?;
        if pending > 0 {
            return Err(AppError::new(
                "PENDING_LOCAL_MUTATION",
                "Есть неподтверждённые локальные операции.",
            ));
        }
        let school = projection_state(&tx, "cloud_school_id")?
            .ok_or_else(|| AppError::new("SCHOOL_SCOPE_VIOLATION", "Не задана школа."))?;
        let rows = {
            let mut statement = tx.prepare("SELECT c.operation_id,c.remote_payload_json FROM sync_conflicts c LEFT JOIN conflict_projections p ON p.operation_id=c.operation_id WHERE p.operation_id IS NULL ORDER BY c.rowid")?;
            let rows = statement
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        for (operation, raw) in rows {
            let remote: Value = serde_json::from_str(&raw)?;
            let changes = remote
                .get("canonical")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    AppError::new(
                        "CLOUD_PROTOCOL_ERROR",
                        "Cloud не вернул данные для восстановления конфликта.",
                    )
                })?;
            let mut canonical = changes
                .iter()
                .map(|v| validate_change(v, &school))
                .collect::<Result<Vec<_>, _>>()?;
            canonical.sort_by_key(|v| {
                (
                    v.priority,
                    if v.entity_type == "LOAN_UPSERT"
                        && v.payload.get("status").and_then(Value::as_str) == Some("ACTIVE")
                    {
                        1
                    } else {
                        0
                    },
                )
            });
            for change in canonical {
                // A rejected local operation may have changed a row without changing its
                // last Cloud version. Reapply equal canonical versions, never an older one.
                tx.execute("DELETE FROM cloud_entity_versions WHERE entity_type=?1 AND entity_id=?2 AND version=?3", params![change.entity_type,change.entity_id,change.version])?;
                projection_apply(&tx, &change.entity_type, &change.payload, change.delete)?;
            }
            tx.execute(
                "INSERT INTO conflict_projections(operation_id,applied_at) VALUES(?1,?2)",
                params![operation, Self::now()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn apply_cloud_bootstrap(
        &mut self,
        snapshot: &Value,
        terminal_id: &str,
        protocol: &str,
    ) -> Result<(), AppError> {
        let school = snapshot
            .get("school")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                AppError::new("CLOUD_PROTOCOL_ERROR", "Bootstrap не содержит school.")
            })?;
        let school_id = projection_string(school.get("id"), "school.id")?;
        if protocol != "1" {
            return Err(AppError::new(
                "CLOUD_PROTOCOL_UNSUPPORTED",
                "Протокол Cloud не поддерживается.",
            ));
        }
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        let pending: i64 = tx.query_row(
            "SELECT COUNT(*) FROM sync_outbox WHERE status IN ('PENDING','SENDING','FAILED')",
            [],
            |r| r.get(0),
        )?;
        if pending > 0 {
            return Err(AppError::new(
                "PENDING_LOCAL_MUTATION",
                "Нельзя заменить данные до подтверждения локальных операций.",
            ));
        }
        if let Some(bound) = projection_state(&tx, "cloud_school_id")? {
            if bound != school_id {
                return Err(AppError::new(
                    "SCHOOL_SCOPE_VIOLATION",
                    "Bootstrap принадлежит другой школе.",
                ));
            }
        }
        tx.execute("INSERT INTO schools(id,cloud_id,name,version,updated_at) VALUES(?1,?1,?2,1,?3) ON CONFLICT(id) DO UPDATE SET name=excluded.name,updated_at=excluded.updated_at",params![school_id,projection_string(school.get("name"),"school.name")?,Self::now()])?;
        tx.execute("INSERT INTO terminals(id,school_id,cloud_id,name,updated_at) VALUES('terminal-local',?1,?2,'EDUS Library',?3) ON CONFLICT(id) DO UPDATE SET cloud_id=excluded.cloud_id,school_id=excluded.school_id",params![school_id,terminal_id,Self::now()])?;
        projection_set_state(&tx, "cloud_school_id", &school_id)?;
        projection_set_state(&tx, "cloud_terminal_id", terminal_id)?;
        projection_set_state(&tx, "sync_protocol_version", protocol)?;
        for (key, entity_type) in [
            ("classes", "CLASS_UPSERT"),
            ("persons", "PERSON_UPSERT"),
            ("locations", "LOCATION_UPSERT"),
            ("book_titles", "BOOK_TITLE_UPSERT"),
            ("book_copies", "BOOK_COPY_UPSERT"),
            ("legacy_title_stock", "LEGACY_STOCK_UPSERT"),
            ("cards", "CARD_UPSERT"),
            ("face_templates", "FACE_TEMPLATE_UPSERT"),
            ("active_loans", "LOAN_UPSERT"),
            ("active_reservations", "RESERVATION_UPSERT"),
        ] {
            let items = snapshot.get(key).and_then(Value::as_array).ok_or_else(|| {
                AppError::new(
                    "CLOUD_PROTOCOL_ERROR",
                    format!("Bootstrap не содержит {key}."),
                )
            })?;
            for item in items {
                let change = serde_json::json!({"entity_type":entity_type,"entity_id":item.get("id"),"version":item.get("version"),"operation":"UPSERT","payload_kind":"FULL","payload_schema_version":1,"payload":item});
                let valid = validate_change(&change, &school_id)?;
                projection_apply(&tx, &valid.entity_type, &valid.payload, false)?;
            }
        }
        projection_set_state(
            &tx,
            "last_cursor",
            &projection_string(snapshot.get("next_cursor"), "next_cursor")?,
        )?;
        projection_set_state(&tx, "bootstrap_completed", "true")?;
        projection_set_state(&tx, "last_sync_at", &Self::now())?;
        tx.commit()?;
        Ok(())
    }

    pub fn apply_cloud_delta_batch(
        &mut self,
        school_id: &str,
        changes: &[Value],
        next_cursor: &str,
    ) -> Result<(), AppError> {
        let mut canonical = changes
            .iter()
            .map(|change| validate_change(change, school_id))
            .collect::<Result<Vec<_>, _>>()?;
        canonical.sort_by_key(|change| change.priority);
        self.ensure_write_space()?;
        let tx = self.conn.transaction()?;
        let bound = projection_state(&tx, "cloud_school_id")?.ok_or_else(|| {
            AppError::new(
                "SCHOOL_SCOPE_VIOLATION",
                "Терминал ещё не bootstrap-привязан к школе.",
            )
        })?;
        if bound != school_id {
            return Err(AppError::new(
                "SCHOOL_SCOPE_VIOLATION",
                "Delta принадлежит другой школе.",
            ));
        }
        let pending: i64 = tx.query_row(
            "SELECT COUNT(*) FROM sync_outbox WHERE status IN ('PENDING','SENDING','FAILED')",
            [],
            |row| row.get(0),
        )?;
        if pending > 0 {
            return Err(AppError::new(
                "PENDING_LOCAL_MUTATION",
                "Сначала требуется подтвердить локальные операции Cloud.",
            ));
        }
        for change in canonical {
            if projection_has_pending(&tx, &change.entity_id)? {
                return Err(AppError::new(
                    "PENDING_LOCAL_MUTATION",
                    "Cloud change затрагивает локальную неподтверждённую операцию.",
                ));
            }
            projection_apply(&tx, &change.entity_type, &change.payload, change.delete)?;
        }
        projection_set_state(&tx, "last_cursor", next_cursor)?;
        projection_set_state(&tx, "last_sync_at", &Self::now())?;
        tx.commit()?;
        Ok(())
    }
}

fn projection_string(value: Option<&Value>, field: &str) -> Result<String, AppError> {
    value
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            AppError::new(
                "CLOUD_PROTOCOL_ERROR",
                format!("Cloud DTO не содержит {field}."),
            )
        })
}
fn projection_state(tx: &Transaction<'_>, key: &str) -> Result<Option<String>, AppError> {
    tx.query_row(
        "SELECT value FROM sync_state WHERE key=?1",
        params![key],
        |r| r.get(0),
    )
    .optional()
    .map_err(Into::into)
}
fn projection_set_state(tx: &Transaction<'_>, key: &str, value: &str) -> Result<(), AppError> {
    tx.execute("INSERT INTO sync_state(key,value,updated_at) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=excluded.updated_at",params![key,value,LocalDatabase::now()])?;
    Ok(())
}
fn projection_has_pending(tx: &Transaction<'_>, entity_id: &str) -> Result<bool, AppError> {
    let mut statement = tx.prepare(
        "SELECT payload_json FROM sync_outbox WHERE status IN ('PENDING','SENDING','FAILED')",
    )?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    for payload in rows {
        if let Ok(value) = serde_json::from_str::<Value>(&payload?) {
            if projection_value_contains(&value, entity_id) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
fn projection_value_contains(value: &Value, id: &str) -> bool {
    match value {
        Value::String(s) => s == id,
        Value::Array(a) => a.iter().any(|v| projection_value_contains(v, id)),
        Value::Object(o) => o.values().any(|v| projection_value_contains(v, id)),
        _ => false,
    }
}
fn projection_collect_copy_ids(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(o) => {
            for key in ["copyId", "copy_id"] {
                if let Some(Value::String(id)) = o.get(key) {
                    out.push(id.clone());
                }
            }
            for v in o.values() {
                projection_collect_copy_ids(v, out)
            }
        }
        Value::Array(a) => {
            for v in a {
                projection_collect_copy_ids(v, out)
            }
        }
        _ => {}
    }
}
fn projection_optional_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Option<String> {
    object.get(field).and_then(Value::as_str).map(str::to_owned)
}
fn projection_number(object: &serde_json::Map<String, Value>, field: &str, default: i64) -> i64 {
    object.get(field).and_then(Value::as_i64).unwrap_or(default)
}
fn projection_apply(
    tx: &Transaction<'_>,
    entity_type: &str,
    payload: &Value,
    delete: bool,
) -> Result<(), AppError> {
    let object = payload.as_object().ok_or_else(|| {
        AppError::new(
            "CLOUD_PROTOCOL_ERROR",
            "Cloud change не содержит object payload.",
        )
    })?;
    let id = projection_string(
        object.get("id").or_else(|| object.get("book_title_id")),
        "entity id",
    )?;
    let version = projection_number(object, "version", 1);
    let existing: Option<i64> = tx
        .query_row(
            "SELECT version FROM cloud_entity_versions WHERE entity_type=?1 AND entity_id=?2",
            params![entity_type, id],
            |row| row.get(0),
        )
        .optional()?;
    if existing.map(|stored| stored >= version).unwrap_or(false) {
        return Ok(());
    }
    let now = LocalDatabase::now();
    if delete {
        match entity_type {
            "PERSON_UPSERT" => {
                tx.execute("UPDATE persons SET status='INACTIVE',deleted_at=?2,version=?3,updated_at=?2 WHERE id=?1",params![id,now,version])?;
            }
            "BOOK_TITLE_UPSERT" => {
                tx.execute(
                    "UPDATE book_titles SET deleted_at=?2,version=?3,updated_at=?2 WHERE id=?1",
                    params![id, now, version],
                )?;
            }
            "BOOK_COPY_UPSERT" => {
                tx.execute("UPDATE book_copies SET status='WRITTEN_OFF',deleted_at=?2,version=?3,updated_at=?2 WHERE id=?1",params![id,now,version])?;
            }
            "CARD_UPSERT" => {
                tx.execute(
                    "UPDATE cards SET status='REVOKED',version=?2,updated_at=?3 WHERE id=?1",
                    params![id, version, now],
                )?;
            }
            "FACE_TEMPLATE_UPSERT" => {
                tx.execute("UPDATE face_templates SET consent_status='REVOKED',revoked_at=?2,updated_at=?2 WHERE id=?1",params![id,now])?;
            }
            "LEGACY_STOCK_UPSERT" => {
                tx.execute("UPDATE legacy_title_stock SET available_quantity=0,version=?2,updated_at=?3 WHERE book_title_id=?1",params![id,version,now])?;
            }
            "CLASS_UPSERT" | "LOCATION_UPSERT" => {}
            "LOAN_UPSERT" => {
                tx.execute("UPDATE loans SET status='CANCELLED',version=?2,updated_at=?3 WHERE id=?1 AND status='ACTIVE'",params![id,version,now])?;
            }
            "RESERVATION_UPSERT" => {
                tx.execute("UPDATE reservations SET status='CANCELLED',version=?2,updated_at=?3 WHERE id=?1 AND status='WAITING'",params![id,version,now])?;
            }
            _ => {}
        }
        tx.execute("INSERT INTO cloud_entity_versions(entity_type,entity_id,version,deleted_at,updated_at) VALUES(?1,?2,?3,?4,?4) ON CONFLICT(entity_type,entity_id) DO UPDATE SET version=excluded.version,deleted_at=excluded.deleted_at,updated_at=excluded.updated_at",params![entity_type,id,version,now])?;
        return Ok(());
    }
    let incomplete = |name: &str| {
        AppError::new(
            "CLOUD_DELTA_INCOMPLETE",
            format!("{name} delta без полного DTO не может создать локальную сущность."),
        )
    };
    match entity_type {
      "CLASS_UPSERT"=>{tx.execute("INSERT INTO classes(id,school_id,name,version,updated_at) VALUES(?1,(SELECT value FROM sync_state WHERE key='cloud_school_id'),?2,?3,?4) ON CONFLICT(id) DO UPDATE SET name=excluded.name,version=excluded.version,updated_at=excluded.updated_at",params![id,projection_string(object.get("name"),"class.name")?,version,now])?;}
      "PERSON_UPSERT"=>if let Some(name)=projection_optional_string(object,"full_name") {tx.execute("INSERT INTO persons(id,cloud_id,school_id,full_name,person_type,class_id,class_name,position_name,status,version,updated_at,deleted_at) VALUES(?1,?1,(SELECT value FROM sync_state WHERE key='cloud_school_id'),?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(id) DO UPDATE SET full_name=excluded.full_name,person_type=excluded.person_type,class_id=excluded.class_id,class_name=excluded.class_name,position_name=excluded.position_name,status=excluded.status,version=excluded.version,updated_at=excluded.updated_at,deleted_at=excluded.deleted_at",params![id,name,projection_string(object.get("person_type"),"person_type")?,projection_optional_string(object,"class_id"),projection_optional_string(object,"class_name"),projection_optional_string(object,"position_name"),projection_optional_string(object,"status").unwrap_or_else(||"ACTIVE".into()),version,projection_optional_string(object,"updated_at").unwrap_or(now.clone()),projection_optional_string(object,"deleted_at")])?;} else if tx.execute("UPDATE persons SET status=coalesce(?2,status),version=?3,updated_at=?4 WHERE id=?1",params![id,projection_optional_string(object,"status"),version,now])?==0{return Err(incomplete("PERSON"));},
      "CARD_UPSERT"=>if let Some(person_id)=projection_optional_string(object,"person_id") {tx.execute("INSERT INTO cards(id,person_id,card_lookup_hash,status,version,updated_at) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET person_id=excluded.person_id,card_lookup_hash=excluded.card_lookup_hash,status=excluded.status,version=excluded.version,updated_at=excluded.updated_at",params![id,person_id,projection_string(object.get("card_lookup_hash"),"card_lookup_hash")?,projection_optional_string(object,"status").unwrap_or_else(||"ACTIVE".into()),version,projection_optional_string(object,"updated_at").unwrap_or(now.clone())])?;} else if tx.execute("UPDATE cards SET status=coalesce(?2,status),version=?3,updated_at=?4 WHERE id=?1",params![id,projection_optional_string(object,"status"),version,now])?==0{return Err(incomplete("CARD"));},
      "FACE_TEMPLATE_UPSERT"=>if let Some(person_id)=projection_optional_string(object,"person_id") {tx.execute("INSERT INTO face_templates(id,person_id,template_ciphertext,model_id,model_version,template_version,consent_reference,consent_status,enrolled_at,updated_at,revoked_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(id) DO UPDATE SET template_ciphertext=excluded.template_ciphertext,model_id=excluded.model_id,model_version=excluded.model_version,template_version=excluded.template_version,consent_reference=excluded.consent_reference,consent_status=excluded.consent_status,updated_at=excluded.updated_at,revoked_at=excluded.revoked_at",params![id,person_id,projection_string(object.get("template_ciphertext"),"template_ciphertext")?,projection_string(object.get("model_id"),"model_id")?,projection_string(object.get("model_version"),"model_version")?,projection_number(object,"template_version",1),projection_optional_string(object,"consent_reference"),projection_optional_string(object,"consent_status").unwrap_or_else(||"ACTIVE".into()),projection_optional_string(object,"enrolled_at").unwrap_or(now.clone()),projection_optional_string(object,"updated_at").unwrap_or(now.clone()),projection_optional_string(object,"revoked_at")])?;}else if tx.execute("UPDATE face_templates SET consent_status=coalesce(?2,consent_status),updated_at=?3 WHERE id=?1",params![id,projection_optional_string(object,"consent_status"),now])?==0{return Err(incomplete("FACE_TEMPLATE"));},
      "LOCATION_UPSERT"=>if let Some(name)=projection_optional_string(object,"name") {tx.execute("INSERT INTO library_locations(id,school_id,name,code,version,updated_at) VALUES(?1,(SELECT value FROM sync_state WHERE key='cloud_school_id'),?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET name=excluded.name,code=excluded.code,version=excluded.version,updated_at=excluded.updated_at",params![id,name,projection_string(object.get("code"),"location.code")?,version,projection_optional_string(object,"updated_at").unwrap_or(now.clone())])?;} else if tx.execute("UPDATE library_locations SET version=?2,updated_at=?3 WHERE id=?1",params![id,version,now])?==0{return Err(incomplete("LOCATION"));},
      "BOOK_TITLE_UPSERT"=>if let Some(title)=projection_optional_string(object,"title") {tx.execute("INSERT INTO book_titles(id,cloud_id,school_id,isbn,title,authors,language,publisher,publication_year,subject,grade,version,updated_at,deleted_at) VALUES(?1,?1,(SELECT value FROM sync_state WHERE key='cloud_school_id'),?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12) ON CONFLICT(id) DO UPDATE SET isbn=excluded.isbn,title=excluded.title,authors=excluded.authors,language=excluded.language,publisher=excluded.publisher,publication_year=excluded.publication_year,subject=excluded.subject,grade=excluded.grade,version=excluded.version,updated_at=excluded.updated_at,deleted_at=excluded.deleted_at",params![id,projection_optional_string(object,"isbn"),title,projection_string(object.get("authors"),"authors")?,projection_string(object.get("language"),"language")?,projection_optional_string(object,"publisher"),object.get("publication_year").and_then(Value::as_i64),projection_optional_string(object,"subject"),projection_optional_string(object,"grade"),version,projection_optional_string(object,"updated_at").unwrap_or(now.clone()),projection_optional_string(object,"deleted_at")])?;} else if tx.execute("UPDATE book_titles SET version=?2,updated_at=?3 WHERE id=?1",params![id,version,now])?==0{return Err(incomplete("BOOK_TITLE"));},
      "BOOK_COPY_UPSERT"=>if let Some(title_id)=projection_optional_string(object,"book_title_id") {tx.execute("INSERT INTO book_copies(id,cloud_id,book_title_id,inventory_number,barcode,status,location_id,version,updated_at,deleted_at) VALUES(?1,?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET book_title_id=excluded.book_title_id,inventory_number=excluded.inventory_number,barcode=excluded.barcode,status=excluded.status,location_id=excluded.location_id,version=excluded.version,updated_at=excluded.updated_at,deleted_at=excluded.deleted_at",params![id,title_id,projection_optional_string(object,"inventory_number"),projection_optional_string(object,"barcode"),projection_optional_string(object,"status").unwrap_or_else(||"AVAILABLE".into()),projection_optional_string(object,"location_id"),version,projection_optional_string(object,"updated_at").unwrap_or(now.clone()),projection_optional_string(object,"deleted_at")])?;} else if tx.execute("UPDATE book_copies SET status=coalesce(?2,status),version=?3,updated_at=?4 WHERE id=?1",params![id,projection_optional_string(object,"status"),version,now])?==0{return Err(incomplete("BOOK_COPY"));},
      "LEGACY_STOCK_UPSERT"=>{let title_id=projection_optional_string(object,"book_title_id").unwrap_or(id.clone());if object.get("total_quantity").is_some(){tx.execute("INSERT INTO legacy_title_stock(book_title_id,total_quantity,available_quantity,version,updated_at) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(book_title_id) DO UPDATE SET total_quantity=excluded.total_quantity,available_quantity=excluded.available_quantity,version=excluded.version,updated_at=excluded.updated_at",params![title_id,projection_number(object,"total_quantity",0),projection_number(object,"available_quantity",0),version,projection_optional_string(object,"updated_at").unwrap_or(now.clone())])?;}else if tx.execute("UPDATE legacy_title_stock SET available_quantity=?2,version=?3,updated_at=?4 WHERE book_title_id=?1",params![title_id,projection_number(object,"available_quantity",0),version,now])?==0{return Err(incomplete("LEGACY_STOCK"));}},
      "LOAN_UPSERT"=>if let Some(reader_id)=projection_optional_string(object,"reader_id") {tx.execute("INSERT INTO terminals(id,school_id,cloud_id,name,status,version,updated_at) VALUES('cloud-projection',(SELECT value FROM sync_state WHERE key='cloud_school_id'),'cloud-projection','Cloud projection','ACTIVE',1,?1) ON CONFLICT(id) DO NOTHING",params![now.clone()])?;tx.execute("INSERT INTO loans(id,cloud_id,reader_id,book_title_id,book_copy_id,accounting_mode,quantity,issued_at,due_at,returned_at,status,created_terminal_id,version,updated_at) VALUES(?1,?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'cloud-projection',?11,?12) ON CONFLICT(id) DO UPDATE SET reader_id=excluded.reader_id,book_title_id=excluded.book_title_id,book_copy_id=excluded.book_copy_id,accounting_mode=excluded.accounting_mode,quantity=excluded.quantity,due_at=excluded.due_at,returned_at=excluded.returned_at,status=excluded.status,version=excluded.version,updated_at=excluded.updated_at",params![id,reader_id,projection_string(object.get("book_title_id"),"loan.book_title_id")?,projection_optional_string(object,"book_copy_id"),projection_string(object.get("accounting_mode"),"loan.accounting_mode")?,projection_remaining_quantity(object)?,projection_optional_string(object,"issued_at").unwrap_or(now.clone()),projection_optional_string(object,"due_at"),projection_optional_string(object,"returned_at"),projection_optional_string(object,"status").unwrap_or_else(||"ACTIVE".into()),version,projection_optional_string(object,"updated_at").unwrap_or(now.clone())])?;}else if tx.execute("UPDATE loans SET status=coalesce(?2,status),version=?3,updated_at=?4 WHERE id=?1",params![id,projection_optional_string(object,"status"),version,now])?==0{return Err(incomplete("LOAN"));},
      "RESERVATION_UPSERT"=>if let Some(reader_id)=projection_optional_string(object,"reader_id") {tx.execute("INSERT INTO reservations(id,cloud_id,reader_id,book_title_id,created_at,status,version,updated_at) VALUES(?1,?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET reader_id=excluded.reader_id,book_title_id=excluded.book_title_id,status=excluded.status,version=excluded.version,updated_at=excluded.updated_at",params![id,reader_id,projection_string(object.get("book_title_id"),"reservation.book_title_id")?,projection_optional_string(object,"created_at").unwrap_or(now.clone()),projection_optional_string(object,"status").unwrap_or_else(||"WAITING".into()),version,projection_optional_string(object,"updated_at").unwrap_or(now.clone())])?;}else if tx.execute("UPDATE reservations SET status=coalesce(?2,status),version=?3,updated_at=?4 WHERE id=?1",params![id,projection_optional_string(object,"status"),version,now])?==0{return Err(incomplete("RESERVATION"));},
      _=>return Err(AppError::new("CLOUD_PROTOCOL_ERROR",format!("Неподдерживаемый тип Cloud change: {entity_type}."))),
    }
    tx.execute("INSERT INTO cloud_entity_versions(entity_type,entity_id,version,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(entity_type,entity_id) DO UPDATE SET version=excluded.version,deleted_at=NULL,updated_at=excluded.updated_at",params![entity_type,id,version,now])?;
    Ok(())
}
impl LocalDatabase {
    pub fn cloud_bootstrap_completed(&self) -> Result<bool, AppError> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM sync_state WHERE key='bootstrap_completed'",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .as_deref()
            == Some("true"))
    }
    pub fn cloud_school_id(&self) -> Result<Option<String>, AppError> {
        self.conn
            .query_row(
                "SELECT value FROM sync_state WHERE key='cloud_school_id'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }
    pub fn cloud_cursor(&self) -> Result<Option<String>, AppError> {
        self.conn
            .query_row(
                "SELECT value FROM sync_state WHERE key='last_cursor'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(Into::into)
    }
}

fn validate_basket(items: &[BasketItem]) -> Result<(), AppError> {
    let mut keys = std::collections::HashSet::new();
    if items.is_empty() || items.len() > 200 {
        return Err(AppError::new(
            "INVALID_INPUT",
            "Выберите от 1 до 200 позиций.",
        ));
    }
    for item in items {
        let valid = item.quantity > 0
            && item.quantity <= 10_000
            && !item.title_id.is_empty()
            && match item.mode.as_str() {
                "COPY" => {
                    item.quantity == 1 && item.copy_id.as_ref().is_some_and(|id| !id.is_empty())
                }
                "LEGACY_TITLE" => item.copy_id.is_none(),
                _ => false,
            };
        let key = item
            .copy_id
            .as_ref()
            .or(item.loan_id.as_ref())
            .unwrap_or(&item.title_id);
        if !valid || !keys.insert(key.clone()) {
            return Err(AppError::new(
                "INVALID_INPUT",
                "Проверьте способ учёта, количество и повторные позиции.",
            ));
        }
    }
    Ok(())
}

fn projection_remaining_quantity(object: &serde_json::Map<String, Value>) -> Result<i64, AppError> {
    let total = object.get("quantity").and_then(Value::as_i64).unwrap_or(0);
    let returned = object
        .get("returned_quantity")
        .and_then(Value::as_i64)
        .unwrap_or(-1);
    let active = object.get("status").and_then(Value::as_str) == Some("ACTIVE");
    if total < 1 || returned < 0 || returned > total || (active && returned == total) {
        return Err(AppError::new(
            "CLOUD_PROTOCOL_ERROR",
            "Некорректное количество в выдаче Cloud.",
        ));
    }
    Ok(if active { total - returned } else { total })
}

#[cfg(test)]
mod regression_tests;

#[cfg(test)]
mod card_tests;
