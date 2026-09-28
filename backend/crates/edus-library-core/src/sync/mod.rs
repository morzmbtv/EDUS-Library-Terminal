pub(crate) mod dto;
mod projection;
use crate::{
    db::LocalDatabase,
    domain::{AppError, SyncStatus},
    security::CredentialStore,
};
pub use projection::CloudProjectionApplier;
use serde_json::{json, Value};
#[cfg(test)]
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum PushResult {
    Ack,
    Conflict {
        reason: String,
        remote_payload: String,
    },
}

/// In-memory fixture for unit tests; never compiled into the service.
#[cfg(test)]
struct LocalTestCloud {
    operations: HashMap<String, (String, String)>,
}
#[cfg(test)]
impl LocalTestCloud {
    pub fn push(&mut self, operation_id: &str, payload: &str, hash: &str) -> PushResult {
        match self.operations.get(operation_id) {
            Some((stored, _)) if stored == hash => PushResult::Ack,
            Some((_, remote)) => PushResult::Conflict {
                reason: "IDEMPOTENCY_CONFLICT".into(),
                remote_payload: remote.clone(),
            },
            None => {
                self.operations
                    .insert(operation_id.into(), (hash.into(), payload.into()));
                PushResult::Ack
            }
        }
    }
}

/// Real network transport. It exists only with an explicit `EDUS_CLOUD_BASE_URL`.
/// HTTPS is mandatory except an explicit loopback UAT opt-in.
pub struct ProductionCloudHttpApi {
    base_url: String,
    credential: String,
    terminal_id: String,
    expected_school_id: String,
}
impl ProductionCloudHttpApi {
    fn agent() -> ureq::Agent {
        ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(15)))
            .max_redirects(0)
            .build()
            .into()
    }
    pub fn validate_base(base: &str) -> Result<String, AppError> {
        let url = url::Url::parse(base.trim())
            .map_err(|_| AppError::new("CLOUD_CONFIG_INVALID", "Некорректный адрес Cloud."))?;
        let local = url.host_str().is_some_and(|host| {
            host == "localhost"
                || host
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        });
        let http_test = cfg!(debug_assertions)
            && local
            && std::env::var("EDUS_CLOUD_ALLOW_HTTP_LOCAL").ok().as_deref() == Some("1");
        if (url.scheme() != "https" && !(url.scheme() == "http" && http_test))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.host_str().is_none()
        {
            return Err(AppError::new(
                "CLOUD_CONFIG_INVALID",
                "Требуется HTTPS URL без пароля, query и fragment.",
            ));
        }
        Ok(url.as_str().trim_end_matches('/').to_owned())
    }
    pub fn configured(
        base: &str,
        credentials: &CredentialStore,
        terminal_id: String,
        expected_school_id: String,
    ) -> Result<Self, AppError> {
        let base_url = Self::validate_base(base)?;
        let credential = credentials
            .cloud_device_credential()?
            .ok_or_else(|| AppError::new("DEVICE_UNAUTHORIZED", "Подключите терминал к Cloud."))?;
        Ok(Self {
            base_url,
            credential,
            terminal_id,
            expected_school_id,
        })
    }
    pub fn enroll_with_code(
        base: &str,
        code: &str,
        name: &str,
        credentials: &CredentialStore,
    ) -> Result<(String, String), AppError> {
        let base = Self::validate_base(base)?;
        if code.len() < 12 || code.len() > 256 {
            return Err(AppError::new("INVALID_INPUT", "Проверьте код подключения."));
        }
        let response = Self::agent()
            .post(format!("{base}/api/terminal/v1/enroll"))
            .send_json(
                json!({"enrollment_code":code,"terminal_name":name,"sync_protocol_version":"1","terminal_version":env!("CARGO_PKG_VERSION")}),
            )
            .map_err(Self::http_error)?;
        let value: Value = response.into_body().read_json().map_err(|_| {
            AppError::new(
                "CLOUD_PROTOCOL_ERROR",
                "Некорректный ответ регистрации Cloud.",
            )
        })?;
        let field = |path| {
            value
                .pointer(path)
                .and_then(Value::as_str)
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| {
                    AppError::new("CLOUD_PROTOCOL_ERROR", "Ответ регистрации Cloud неполный.")
                })
        };
        let token = field("/details/device_credential")?;
        let terminal = field("/details/terminal_id")?;
        let school = field("/details/school_id")?;
        let secret = field("/details/card_hmac_secret")?;
        if field("/details/protocol/sync_protocol_version")? != "1" {
            return Err(AppError::new(
                "CLOUD_PROTOCOL_UNSUPPORTED",
                "Версия Cloud не поддерживается.",
            ));
        }
        credentials.save_card_hmac_secret(&secret)?;
        credentials.save_cloud_device_credential(&token)?;
        Ok((terminal, school))
    }
    #[cfg(test)]
    pub fn from_environment(credentials: &CredentialStore) -> Result<Option<Self>, AppError> {
        let Some(base) = std::env::var("EDUS_CLOUD_BASE_URL").ok() else {
            return Ok(None);
        };
        let (terminal, school) = Self::enroll_with_code(
            &base,
            &std::env::var("EDUS_CLOUD_ENROLLMENT_CODE").unwrap_or_default(),
            "Automated integration terminal",
            credentials,
        )?;
        Self::configured(&base, credentials, terminal, school).map(Some)
    }
    fn http_error(error: ureq::Error) -> AppError {
        match error {
            ureq::Error::StatusCode(401) => AppError::new(
                "AUTH_REQUIRED",
                "Cloud credential недействителен или отозван.",
            ),
            ureq::Error::StatusCode(403) => AppError::new(
                "SCHOOL_SCOPE_VIOLATION",
                "Cloud отклонил scope школы для устройства.",
            ),
            ureq::Error::StatusCode(409) => AppError::new(
                "CLOUD_PROTOCOL_UNSUPPORTED",
                "Требуется совместимая версия терминала/Cloud. Локальные операции сохранены.",
            ),
            _ => AppError::new(
                "CLOUD_UNAVAILABLE",
                "Cloud недоступен. Локальные операции сохранены для повторной синхронизации.",
            ),
        }
    }
    pub fn bootstrap(&self) -> Result<Value, AppError> {
        let response = Self::agent()
            .post(format!("{}/api/terminal/v1/bootstrap", self.base_url))
            .header("Authorization", &format!("Bearer {}", self.credential))
            .header("X-EDUS-Sync-Protocol", "1")
            .header("X-EDUS-Terminal-Version", env!("CARGO_PKG_VERSION"))
            .send_empty()
            .map_err(Self::http_error)?;
        let value: Value = response
            .into_body()
            .read_json()
            .map_err(|error| AppError::new("CLOUD_PROTOCOL_ERROR", error.to_string()))?;
        let details = value.get("details").cloned().ok_or_else(|| {
            AppError::new("CLOUD_PROTOCOL_ERROR", "Cloud bootstrap не вернул details.")
        })?;
        if details.pointer("/school/id").and_then(Value::as_str)
            != Some(self.expected_school_id.as_str())
        {
            return Err(AppError::new(
                "SCHOOL_SCOPE_VIOLATION",
                "Cloud вернул другую школу. Локальная база не изменена.",
            ));
        }
        Ok(details)
    }

    pub fn changes(&self, cursor: Option<&str>) -> Result<Value, AppError> {
        let url = match cursor {
            Some(cursor) => format!("{}/api/terminal/v1/changes?cursor={cursor}", self.base_url),
            None => format!("{}/api/terminal/v1/changes", self.base_url),
        };
        let response = Self::agent()
            .get(url)
            .header("Authorization", &format!("Bearer {}", self.credential))
            .header("X-EDUS-Sync-Protocol", "1")
            .header("X-EDUS-Terminal-Version", env!("CARGO_PKG_VERSION"))
            .call()
            .map_err(Self::http_error)?;
        let value: Value = response
            .into_body()
            .read_json()
            .map_err(|error| AppError::new("CLOUD_PROTOCOL_ERROR", error.to_string()))?;
        value.get("details").cloned().ok_or_else(|| {
            AppError::new("CLOUD_PROTOCOL_ERROR", "Cloud changes не вернул details.")
        })
    }
    pub fn push(
        &self,
        operation_id: &str,
        operation_type: &str,
        payload: &str,
        hash: &str,
        occurred_at: &str,
    ) -> Result<PushResult, AppError> {
        let payload_json = payload.to_owned();
        let payload: Value = serde_json::from_str(payload)
            .map_err(|e| AppError::new("CLOUD_PROTOCOL_ERROR", e.to_string()))?;
        let response=Self::agent().post(format!("{}/api/terminal/v1/operations",self.base_url)).header("Authorization",&format!("Bearer {}",self.credential)).header("X-EDUS-Sync-Protocol","1").header("X-EDUS-Terminal-Version",env!("CARGO_PKG_VERSION")).header("Content-Type","application/json").send_json(json!({"sync_protocol_version":"1","operations":[{"operation_id":operation_id,"operation_type":operation_type,"occurred_at":occurred_at,"payload_hash":hash,"payload_json":payload_json,"payload":payload}]})).map_err(Self::http_error)?;
        let value: Value = response
            .into_body()
            .read_json()
            .map_err(|e| AppError::new("CLOUD_PROTOCOL_ERROR", e.to_string()))?;
        let item = value
            .pointer("/details/results/0")
            .ok_or_else(|| AppError::new("CLOUD_PROTOCOL_ERROR", "Cloud push не вернул result."))?;
        if item.get("operation_id").and_then(Value::as_str) != Some(operation_id) {
            return Err(AppError::new(
                "CLOUD_PROTOCOL_ERROR",
                "Cloud вернул результат другой операции.",
            ));
        }
        if item.get("status").and_then(Value::as_str) == Some("ACK") {
            Ok(PushResult::Ack)
        } else if item.get("status").and_then(Value::as_str) == Some("CONFLICT") {
            Ok(PushResult::Conflict {
                reason: item
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("CLOUD_CONFLICT")
                    .to_owned(),
                remote_payload: item.to_string(),
            })
        } else {
            Err(AppError::new(
                "CLOUD_PROTOCOL_ERROR",
                "Cloud не подтвердил результат операции.",
            ))
        }
    }
}

pub struct SyncEngine {
    transport: Option<ProductionCloudHttpApi>,
}
impl SyncEngine {
    pub fn with_production(production: Option<ProductionCloudHttpApi>) -> Self {
        Self {
            transport: production,
        }
    }
    pub fn online(&self) -> bool {
        self.transport.is_some()
    }
    pub fn test_cloud(&self) -> bool {
        false
    }
    pub fn status(&self, db: &LocalDatabase) -> Result<SyncStatus, AppError> {
        Self::status_for(db, self.online(), self.test_cloud())
    }
    // Read-only status must remain available while the transport is waiting on HTTP.
    pub fn status_for(
        db: &LocalDatabase,
        configured: bool,
        test_cloud: bool,
    ) -> Result<SyncStatus, AppError> {
        let (pending, conflicts) = db.outbox_counts()?;
        let last_sync_at = db.last_sync_at()?;
        let last_error = db.last_sync_error()?;
        let online = configured && last_sync_at.is_some() && last_error.is_none();
        let state = if conflicts > 0 {
            "CONFLICT"
        } else if pending > 0 {
            if online {
                "SYNCING"
            } else {
                "OFFLINE_QUEUE"
            }
        } else {
            "READY"
        };
        Ok(SyncStatus {
            state: state.into(),
            local_ready: true,
            online,
            pending_count: pending,
            conflict_count: conflicts,
            last_sync_at,
            last_error,
            test_cloud,
        })
    }
    pub fn sync_shared(
        &mut self,
        database: &std::sync::Mutex<LocalDatabase>,
    ) -> Result<SyncStatus, AppError> {
        self.run_sync(&mut DatabaseAccess::Shared(database))
    }
    pub fn sync(&mut self, db: &mut LocalDatabase) -> Result<SyncStatus, AppError> {
        self.run_sync(&mut DatabaseAccess::Direct(db))
    }
    fn run_sync(&mut self, access: &mut DatabaseAccess<'_>) -> Result<SyncStatus, AppError> {
        let result = self.sync_with(access);
        access.with(|db| db.set_sync_error(result.as_ref().err().map(|e| e.code.as_str())))?;
        result
    }
    fn sync_with(&mut self, access: &mut DatabaseAccess<'_>) -> Result<SyncStatus, AppError> {
        let Some(transport) = self.transport.as_mut() else {
            return access.with(|db| self.status(db));
        };
        {
            let cloud = &mut *transport;
            if !access.with(|db| db.cloud_bootstrap_completed())? {
                let snapshot = cloud.bootstrap()?;
                access.with(|db| {
                    CloudProjectionApplier::apply_bootstrap(db, &snapshot, &cloud.terminal_id, "1")
                })?;
            }
        }
        let pending = access.with(|db| db.pending_outbox())?;
        for (id, kind, payload, hash, time) in pending {
            let response = transport.push(&id, &kind, &payload, &hash, &time)?;
            access.with(|db| match &response {
                PushResult::Ack => CloudProjectionApplier::reconcile_ack(db, &id),
                PushResult::Conflict {
                    reason,
                    remote_payload,
                } => CloudProjectionApplier::reconcile_conflict(
                    db,
                    &id,
                    reason,
                    &payload,
                    remote_payload,
                ),
            })?;
        }
        {
            let cloud = &mut *transport;
            access.with(|db| db.apply_conflict_projections())?;
            // Bound one cycle. More pages are pulled by the next worker tick.
            for _ in 0..10 {
                let (cursor, school) =
                    access.with(|db| Ok((db.cloud_cursor()?, db.cloud_school_id()?)))?;
                let school = school.ok_or_else(|| {
                    AppError::new("SCHOOL_SCOPE_VIOLATION", "Не задана школа Cloud.")
                })?;
                let details = cloud.changes(cursor.as_deref())?;
                let changes = details
                    .get("changes")
                    .and_then(Value::as_array)
                    .ok_or_else(|| {
                        AppError::new("CLOUD_PROTOCOL_ERROR", "Некорректная delta Cloud.")
                    })?;
                let next = details
                    .get("next_cursor")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::new("CLOUD_PROTOCOL_ERROR", "Cloud не вернул курсор.")
                    })?;
                access.with(|db| {
                    CloudProjectionApplier::apply_delta_batch(db, &school, changes, next)
                })?;
                if changes.len() < 500 {
                    break;
                }
            }
        }
        access.with(|db| self.status(db))
    }
}

/// A lock is held only while executing a local database action, never during HTTP.
enum DatabaseAccess<'a> {
    Shared(&'a std::sync::Mutex<LocalDatabase>),
    Direct(&'a mut LocalDatabase),
}
impl DatabaseAccess<'_> {
    fn with<R>(
        &mut self,
        action: impl FnOnce(&mut LocalDatabase) -> Result<R, AppError>,
    ) -> Result<R, AppError> {
        match self {
            Self::Shared(database) => action(&mut *database.lock().map_err(|_| {
                AppError::new("LOCAL_DATABASE_ERROR", "Локальная база недоступна.")
            })?),
            Self::Direct(database) => action(database),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::LocalDatabase;
    use uuid::Uuid;

    #[test]
    fn test_cloud_is_idempotent_and_rejects_payload_replay() {
        let mut cloud = LocalTestCloud {
            operations: HashMap::new(),
        };
        assert!(matches!(
            cloud.push("op-1", "one", "hash-a"),
            PushResult::Ack
        ));
        assert!(matches!(
            cloud.push("op-1", "one", "hash-a"),
            PushResult::Ack
        ));
        assert!(matches!(
            cloud.push("op-1", "two", "hash-b"),
            PushResult::Conflict { .. }
        ));
    }

    #[test]
    fn cloud_http_requires_explicit_loopback_override() {
        std::env::set_var("EDUS_CLOUD_BASE_URL", "http://127.0.0.1:8088");
        std::env::remove_var("EDUS_CLOUD_ALLOW_HTTP_LOCAL");
        let result = ProductionCloudHttpApi::from_environment(&CredentialStore::test_fixture());
        assert!(matches!(result,Err(AppError{code,..})if code=="CLOUD_CONFIG_INVALID"));
        std::env::remove_var("EDUS_CLOUD_BASE_URL");
    }

    #[test]
    #[ignore = "requires dedicated local Laravel integration fixture; run explicitly"]
    fn cloud_http_end_to_end_two_encrypted_terminals() {
        use crate::{domain::BasketItem, identity::IdentityService};
        assert_eq!(std::env::var("EDUS_CLOUD_UAT").as_deref(), Ok("1"));
        let base = std::env::var("EDUS_CLOUD_BASE_URL").expect("local HTTP fixture URL");
        let root = tempfile::Builder::new()
            .prefix("edus-real-http-")
            .tempdir_in("E:\\Codex\\temp")
            .unwrap();
        let ca = CredentialStore::named_test_production_fixture("real-http-a");
        let cb = CredentialStore::named_test_production_fixture("real-http-b");
        let (ta, school) =
            ProductionCloudHttpApi::enroll_with_code(&base, "EDUS-A1-ENROLL", "Integration A", &ca)
                .unwrap();
        let (tb, sb) =
            ProductionCloudHttpApi::enroll_with_code(&base, "EDUS-A2-ENROLL", "Integration B", &cb)
                .unwrap();
        assert_eq!(school, sb);
        let wrong_scope = ProductionCloudHttpApi::configured(
            &base,
            &ca,
            ta.clone(),
            "10000000-0000-4000-8000-000000000099".into(),
        )
        .unwrap();
        assert_eq!(
            wrong_scope.bootstrap().unwrap_err().code,
            "SCHOOL_SCOPE_VIOLATION"
        );
        let mut sa = SyncEngine::with_production(Some(
            ProductionCloudHttpApi::configured(&base, &ca, ta, school.clone()).unwrap(),
        ));
        let mut sb = SyncEngine::with_production(Some(
            ProductionCloudHttpApi::configured(&base, &cb, tb, school.clone()).unwrap(),
        ));
        let pa = root.path().join("a.db");
        let mut a = LocalDatabase::open(pa.clone(), &ca).unwrap();
        let mut b = LocalDatabase::open(root.path().join("b.db"), &cb).unwrap();
        assert!(
            a.snapshot(false).unwrap().readers.is_empty(),
            "production never seeds demo"
        );
        sa.sync(&mut a).expect("A real HTTP bootstrap");
        sb.sync(&mut b).expect("B bootstrap");
        let reader = IdentityService::new(ca.clone())
            .card(&a, "edus-a-001")
            .expect("enrolled shared HMAC key")
            .reader
            .id;
        let title = "40000000-0000-4000-8000-000000000001";
        let copy = "50000000-0000-4000-8000-000000000001";
        let item = BasketItem {
            id: "scan-1".into(),
            title_id: title.into(),
            copy_id: Some(copy.into()),
            quantity: 1,
            mode: "COPY".into(),
            loan_id: None,
        };
        let op = Uuid::new_v4().to_string();
        let issued = a
            .issue(&reader, std::slice::from_ref(&item), &op)
            .expect("offline issue");
        assert_eq!(
            a.issue(&reader, std::slice::from_ref(&item), &op)
                .unwrap()
                .loan_ids,
            issued.loan_ids,
            "local idempotency"
        );
        assert_eq!(a.book_availability(title).unwrap().available_copies, 0);
        assert_eq!(a.outbox_counts().unwrap().0, 1);
        drop(a);
        let mut a = LocalDatabase::open(pa, &ca).expect("restart persistence");
        assert_eq!(a.reader_loans(&reader).unwrap().len(), 1);
        // Both terminals act offline on the same initial projection.
        b.issue(
            &reader,
            std::slice::from_ref(&item),
            &Uuid::new_v4().to_string(),
        )
        .unwrap();
        sa.sync(&mut a).expect("push A");
        sb.sync(&mut b)
            .expect("conflict B, canonical reconciliation");
        assert_eq!(b.outbox_counts().unwrap(), (0, 1));
        assert_eq!(b.reader_loans(&reader).unwrap()[0].id, issued.loan_ids[0]);
        assert_eq!(b.reader_loans(&reader).unwrap().len(), 1);
        let returned = BasketItem {
            loan_id: Some(issued.loan_ids[0].clone()),
            ..item.clone()
        };
        a.accept(
            &reader,
            std::slice::from_ref(&returned),
            &Uuid::new_v4().to_string(),
        )
        .unwrap();
        assert_eq!(a.book_availability(title).unwrap().available_copies, 1);
        sa.sync(&mut a).unwrap();
        sb.sync(&mut b).unwrap();
        assert!(b.reader_loans(&reader).unwrap().is_empty());
        // Partial legacy return is one physical book, not one title.
        let legacy = BasketItem {
            id: "legacy".into(),
            title_id: title.into(),
            copy_id: None,
            quantity: 2,
            mode: "LEGACY_TITLE".into(),
            loan_id: None,
        };
        let loan = a
            .issue(
                &reader,
                std::slice::from_ref(&legacy),
                &Uuid::new_v4().to_string(),
            )
            .unwrap();
        a.accept(
            &reader,
            &[BasketItem {
                quantity: 1,
                loan_id: Some(loan.loan_ids[0].clone()),
                ..legacy
            }],
            &Uuid::new_v4().to_string(),
        )
        .unwrap();
        sa.sync(&mut a)
            .expect("ordered offline issue/partial-return outbox");
        sb.sync(&mut b).unwrap();
        assert_eq!(a.reader_loans(&reader).unwrap()[0].quantity, 1);
        assert_eq!(b.reader_loans(&reader).unwrap()[0].quantity, 1);
        let reservation = a
            .reserve(&reader, title, &Uuid::new_v4().to_string())
            .unwrap();
        assert!(a
            .reserve(&reader, title, &Uuid::new_v4().to_string())
            .is_err());
        sa.sync(&mut a).unwrap();
        sb.sync(&mut b).unwrap();
        assert_eq!(b.book_availability(title).unwrap().waiting_reservations, 1);
        a.cancel_reservation(
            &reader,
            reservation.reservation_id.as_deref().unwrap(),
            &Uuid::new_v4().to_string(),
        )
        .unwrap();
        sa.sync(&mut a).unwrap();
        sb.sync(&mut b).unwrap();
        assert_eq!(b.book_availability(title).unwrap().waiting_reservations, 0);
        // New title/person/card published after both terminals bootstrapped.
        let output = std::process::Command::new("E:\\Codex\\tools\\php-8.4\\php.exe")
            .args(["tests/integration-fixture.php", "publish"])
            .current_dir("E:\\Codex\\projects\\edus-library-cloud")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "fixture publish failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        sa.sync(&mut a).unwrap();
        sb.sync(&mut b).unwrap();
        assert_eq!(b.titles("Remote new").unwrap().len(), 1);
        assert!(IdentityService::new(cb).card(&b, "remote-99").is_ok());
        let verification = std::process::Command::new("E:\\Codex\\tools\\php-8.4\\php.exe")
            .args(["tests/integration-fixture.php", "verify"])
            .current_dir("E:\\Codex\\projects\\edus-library-cloud")
            .output()
            .unwrap();
        assert!(
            verification.status.success(),
            "PostgreSQL verification failed: {}",
            String::from_utf8_lossy(&verification.stderr)
        );
        assert_eq!(a.outbox_counts().unwrap().0, 0);
        assert_eq!(a.cipher_memory_hardening().unwrap(), "1");
        assert!(a.cipher_version().unwrap().starts_with("4.18.0"));
        assert!(
            rusqlite::Connection::open(&a.path)
                .unwrap()
                .query_row("SELECT count(*) FROM persons", [], |r| r.get::<_, i64>(0))
                .is_err(),
            "independent no-key read must fail"
        );
    }
}
