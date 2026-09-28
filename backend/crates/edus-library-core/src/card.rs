//! Reader-card diagnostics and short-lived UAT capture state.
//!
//! Raw reader input is normalised then HMACed. It never becomes persisted
//! diagnostic data, a database field, or a log value.
use crate::{
    db::LocalDatabase,
    domain::{AppError, Reader},
    security::{card_lookup_hash, normalize_card_code, CredentialStore},
};
use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct Capture {
    token: String,
    hash: String,
    expires: Instant,
}
pub struct CardDiagnostics {
    credentials: CredentialStore,
    capture: Mutex<Option<Capture>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardReport {
    pub input_received: bool,
    pub character_count: usize,
    pub suffix: String,
    pub normalization_success: bool,
    pub hash_found: bool,
    pub reader_resolved: bool,
    pub reader: Option<Reader>,
    pub fingerprint: Option<String>,
    pub runtime_mode: String,
    pub db_namespace: String,
    pub capture_token: Option<String>,
}

impl CardDiagnostics {
    pub fn new(credentials: CredentialStore) -> Self {
        Self {
            credentials,
            capture: Mutex::new(None),
        }
    }
    pub fn clear(&self) {
        if let Ok(mut capture) = self.capture.lock() {
            *capture = None;
        }
    }
    pub fn diagnose(&self, db: &LocalDatabase, raw: &str) -> Result<CardReport, AppError> {
        self.clear();
        let suffix = if raw.ends_with("\r\n") {
            "CR+LF"
        } else if raw.ends_with('\r') {
            "CR / Enter"
        } else if raw.ends_with('\n') {
            "LF / Enter"
        } else if raw.ends_with('\t') {
            "Tab"
        } else {
            "Нет"
        };
        let normalized = normalize_card_code(raw);
        let mut report = CardReport {
            input_received: !raw.is_empty(),
            character_count: raw.trim_end_matches(['\r', '\n', '\t']).chars().count(),
            suffix: suffix.into(),
            normalization_success: normalized.is_ok(),
            hash_found: false,
            reader_resolved: false,
            reader: None,
            fingerprint: None,
            runtime_mode: if self.credentials.is_test_mode() {
                "UAT"
            } else {
                "PRODUCTION"
            }
            .into(),
            db_namespace: self.credentials.namespace().into(),
            capture_token: None,
        };
        if normalized.is_err() {
            return Ok(report);
        }
        let hash = card_lookup_hash(raw, &self.credentials.card_hmac_secret()?)?;
        report.hash_found = db.card_hash_exists(&hash)?;
        report.reader = match db.reader_by_card_hash(&hash) {
            Ok(reader) => Some(reader),
            Err(error) if error.code == "NOT_FOUND" => None,
            Err(error) => return Err(error),
        };
        report.reader_resolved = report.reader.is_some();
        report.fingerprint = Some(format!("HMAC •••• {}", &hash[..8]));
        if self.credentials.is_test_mode() {
            let token = uuid::Uuid::new_v4().to_string();
            *self
                .capture
                .lock()
                .map_err(|_| AppError::new("CARD_CAPTURE_BUSY", "Диагностика занята."))? =
                Some(Capture {
                    token: token.clone(),
                    hash,
                    expires: Instant::now() + Duration::from_secs(120),
                });
            report.capture_token = Some(token);
        }
        Ok(report)
    }
    pub fn bind(
        &self,
        db: &mut LocalDatabase,
        reader_id: &str,
        token: &str,
    ) -> Result<Reader, AppError> {
        if !self.credentials.is_test_mode() {
            return Err(AppError::new(
                "TEST_MODE_REQUIRED",
                "Привязка доступна только в UAT.",
            ));
        }
        let mut capture = self
            .capture
            .lock()
            .map_err(|_| AppError::new("CARD_CAPTURE_BUSY", "Диагностика занята."))?;
        let item = capture
            .as_ref()
            .filter(|item| item.token == token && item.expires > Instant::now())
            .ok_or_else(|| {
                AppError::new(
                    "CARD_CAPTURE_EXPIRED",
                    "Повторите считывание карты: время подтверждения истекло.",
                )
            })?;
        db.bind_test_reader_card_hash(reader_id, &item.hash)?;
        let reader = db.reader_by_card_hash(&item.hash)?;
        *capture = None;
        Ok(reader)
    }
}
