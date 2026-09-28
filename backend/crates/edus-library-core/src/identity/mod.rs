use crate::db::LocalDatabase;
use crate::{
    domain::{AppError, IdentityResult, Reader},
    security::{card_lookup_hash, CredentialStore},
};

#[derive(Clone)]
pub struct IdentityService {
    credentials: CredentialStore,
}
impl IdentityService {
    pub fn new(credentials: CredentialStore) -> Self {
        Self { credentials }
    }
    pub fn card(&self, database: &LocalDatabase, raw: &str) -> Result<IdentityResult, AppError> {
        if raw.len() > 256 || !raw.chars().any(|c| c.is_ascii_alphanumeric()) {
            return Err(AppError::new("INVALID_INPUT", "Код карты пуст."));
        }
        let hash = card_lookup_hash(raw, &self.credentials.card_hmac_secret()?)?;
        let reader: Reader = database.reader_by_card_hash(&hash)?;
        Ok(IdentityResult {
            person_id: reader.id.clone(),
            method: "CARD".into(),
            confidence_class: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            reader,
        })
    }
}

#[derive(Default)]
pub struct ReaderSession {
    selected: Option<(String, std::time::Instant)>,
}
impl ReaderSession {
    pub fn remember(&mut self, id: &str) {
        self.selected = Some((id.into(), std::time::Instant::now()));
    }
    pub fn clear(&mut self) {
        self.selected = None;
    }
    pub fn selected(&self) -> Option<&str> {
        self.selected
            .as_ref()
            .filter(|(_, at)| at.elapsed() < std::time::Duration::from_secs(5 * 60))
            .map(|(id, _)| id.as_str())
    }
    pub fn require(&self, id: &str) -> Result<(), AppError> {
        if self.selected() != Some(id) {
            return Err(AppError::new(
                "READER_IDENTIFICATION_REQUIRED",
                "Сначала определите читателя по карте или по лицу.",
            ));
        }
        Ok(())
    }
}
#[cfg(test)]
mod session_tests {
    use super::*;
    #[test]
    fn reader_context_does_not_authorize_another_person_and_expires() {
        let mut session = ReaderSession::default();
        assert!(session.require("one").is_err());
        session.remember("one");
        assert!(session.require("one").is_ok());
        assert!(session.require("two").is_err());
        session.selected.as_mut().unwrap().1 =
            std::time::Instant::now() - std::time::Duration::from_secs(301);
        assert!(session.require("one").is_err());
        session.remember("one");
        session.clear();
        assert!(session.require("one").is_err());
    }
}
