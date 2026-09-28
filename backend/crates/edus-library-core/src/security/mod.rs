use crate::domain::AppError;
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Service-owned DPAPI credential store. The Windows user scope is the service identity.
#[derive(Clone)]
pub struct CredentialStore {
    service: String,
    test_mode: bool,
    backend: CredentialBackend,
}

#[derive(Clone)]
enum CredentialBackend {
    #[cfg(test)]
    Memory,
    ServiceDpapi {
        path: PathBuf,
    },
}

#[derive(Default, Serialize, Deserialize)]
struct ServiceSecrets {
    values: BTreeMap<String, String>,
}

impl CredentialStore {
    /// Creates a credential store for `EDUSLibraryService`. `root` is the
    /// service-owned ProgramData root; secrets are DPAPI-protected in the
    /// current service identity and stored separately for Production and UAT.
    pub fn for_service_workspace(root: &Path, test_mode: bool) -> Self {
        let namespace = if test_mode {
            "edus.library.service.uat"
        } else {
            "edus.library.service.production"
        };
        let workspace = if test_mode { "UAT" } else { "Production" };
        Self {
            service: namespace.into(),
            test_mode,
            backend: CredentialBackend::ServiceDpapi {
                path: root.join(workspace).join("secrets.dpapi"),
            },
        }
    }

    pub fn existing_database_key(&self) -> Result<String, AppError> {
        let value = self.read_required("database-key-v1")?;
        if value.is_empty() {
            return Err(AppError::new(
                "CREDENTIAL_STORE_ERROR",
                "Ключ существующей базы недоступен. Требуется восстановление.",
            ));
        }
        Ok(value)
    }

    pub fn save_card_hmac_secret(&self, secret: &str) -> Result<(), AppError> {
        if secret.len() < 32 {
            return Err(AppError::new(
                "CLOUD_PROTOCOL_ERROR",
                "Cloud не вернул ключ идентификации карт.",
            ));
        }
        self.write_secret("card-hmac-key-v1", secret)?;
        Ok(())
    }

    #[cfg(test)]
    pub fn test_fixture() -> Self {
        Self {
            service: format!(
                "kz.edus.library.automated-tests.{}",
                std::thread::current().name().unwrap_or("integration")
            ),
            test_mode: true,
            backend: CredentialBackend::Memory,
        }
    }

    #[cfg(test)]
    pub fn named_test_production_fixture(name: &str) -> Self {
        Self {
            service: format!("kz.edus.library.automated-tests.{}", name),
            test_mode: false,
            backend: CredentialBackend::Memory,
        }
    }
    #[cfg(test)]
    pub fn test_production_fixture() -> Self {
        let mut store = Self::test_fixture();
        store.test_mode = false;
        store
    }
    fn password(&self, account: &str) -> Result<String, AppError> {
        static CREATION: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _creation = CREATION
            .lock()
            .map_err(|_| AppError::new("CREDENTIAL_STORE_ERROR", "Хранилище ключей занято."))?;
        match self.read_optional(account) {
            Ok(Some(value)) if !value.is_empty() => Ok(value),
            Ok(None) | Ok(Some(_)) => {
                let mut bytes = [0_u8; 32];
                rand::rng().fill_bytes(&mut bytes);
                let value = bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                self.write_secret(account, &value)?;
                Ok(value)
            }
            Err(error) => Err(error),
        }
    }

    pub fn database_key(&self) -> Result<String, AppError> {
        self.password("database-key-v1")
    }
    /// Creation is allowed only when initializing a new UAT database.
    pub fn initialize_uat_card_secret(&self) -> Result<(), AppError> {
        if self.test_mode {
            self.password("card-hmac-key-v1")?;
        }
        Ok(())
    }
    pub fn card_hmac_secret(&self) -> Result<String, AppError> {
        let secret=self.read_required("card-hmac-key-v1")
            .map_err(|_| AppError::new("CARD_SECRET_UNAVAILABLE", "Ключ считывателя недоступен в Windows. Восстановите защищённое хранилище службы. Новый ключ автоматически не создаётся."))?;
        if secret.len() < 32 {
            return Err(AppError::new(
                "CARD_SECRET_UNAVAILABLE",
                "Ключ считывателя не соответствует требованиям.",
            ));
        }
        Ok(secret)
    }
    pub fn namespace(&self) -> &str {
        &self.service
    }
    pub fn is_test_mode(&self) -> bool {
        self.test_mode
    }
    pub fn cloud_device_credential(&self) -> Result<Option<String>, AppError> {
        Ok(self
            .read_optional("cloud-device-credential-v1")?
            .filter(|value| !value.is_empty()))
    }
    pub fn save_cloud_device_credential(&self, credential: &str) -> Result<(), AppError> {
        if credential.is_empty() {
            return Err(AppError::new(
                "DEVICE_UNAUTHORIZED",
                "Сервер не вернул credential устройства.",
            ));
        }
        self.write_secret("cloud-device-credential-v1", credential)?;
        Ok(())
    }

    fn read_required(&self, account: &str) -> Result<String, AppError> {
        self.read_optional(account)?
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::new(
                    "CREDENTIAL_STORE_ERROR",
                    "Защищённый ключ недоступен в Windows.",
                )
            })
    }
    fn read_optional(&self, account: &str) -> Result<Option<String>, AppError> {
        match &self.backend {
            #[cfg(test)]
            CredentialBackend::Memory => Ok(test_secrets()
                .lock()
                .unwrap()
                .get(&(self.service.clone(), account.into()))
                .cloned()),
            CredentialBackend::ServiceDpapi { path } => {
                Ok(read_service_secrets(path)?.values.get(account).cloned())
            }
        }
    }
    fn write_secret(&self, account: &str, value: &str) -> Result<(), AppError> {
        static WRITES: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _write = WRITES
            .lock()
            .map_err(|_| AppError::new("CREDENTIAL_STORE_ERROR", "Хранилище ключей занято."))?;
        match &self.backend {
            #[cfg(test)]
            CredentialBackend::Memory => {
                test_secrets()
                    .lock()
                    .unwrap()
                    .insert((self.service.clone(), account.into()), value.into());
                Ok(())
            }
            CredentialBackend::ServiceDpapi { path } => {
                let mut secrets = read_service_secrets(path)?;
                secrets.values.insert(account.into(), value.into());
                write_service_secrets(path, &secrets)
            }
        }
    }
}

#[cfg(windows)]
fn protect(data: &[u8]) -> Result<Vec<u8>, AppError> {
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::{LocalFree, HLOCAL},
            Security::Cryptography::{
                CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
            },
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len().try_into().map_err(|_| {
            AppError::new(
                "CREDENTIAL_STORE_ERROR",
                "Размер защищённого ключа недопустим.",
            )
        })?,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(
            &input,
            PCWSTR::null(),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|_| {
            AppError::new(
                "CREDENTIAL_STORE_ERROR",
                "Windows не смогла защитить ключ службы.",
            )
        })?;
    }
    let result =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
    }
    Ok(result)
}
#[cfg(windows)]
fn unprotect(data: &[u8]) -> Result<Vec<u8>, AppError> {
    use windows::Win32::{
        Foundation::{LocalFree, HLOCAL},
        Security::Cryptography::{
            CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len().try_into().map_err(|_| {
            AppError::new(
                "CREDENTIAL_STORE_ERROR",
                "Размер защищённого ключа недопустим.",
            )
        })?,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|_| {
            AppError::new(
                "CREDENTIAL_STORE_ERROR",
                "Windows не смогла открыть ключ службы.",
            )
        })?;
    }
    let result =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
    }
    Ok(result)
}
#[cfg(not(windows))]
fn protect(_: &[u8]) -> Result<Vec<u8>, AppError> {
    Err(AppError::new(
        "CREDENTIAL_STORE_ERROR",
        "Service storage поддерживается только в Windows.",
    ))
}
#[cfg(not(windows))]
fn unprotect(_: &[u8]) -> Result<Vec<u8>, AppError> {
    Err(AppError::new(
        "CREDENTIAL_STORE_ERROR",
        "Service storage поддерживается только в Windows.",
    ))
}
fn read_service_secrets(path: &Path) -> Result<ServiceSecrets, AppError> {
    if !path.exists() {
        return Ok(ServiceSecrets::default());
    }
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > 64 * 1024 || !metadata.is_file() {
        return Err(AppError::new(
            "CREDENTIAL_STORE_ERROR",
            "Размер защищённого хранилища недопустим.",
        ));
    }
    let encrypted = std::fs::read(path).map_err(|_| {
        AppError::new(
            "CREDENTIAL_STORE_ERROR",
            "Не удалось открыть защищённые ключи службы.",
        )
    })?;
    let plaintext = unprotect(&encrypted)?;
    serde_json::from_slice(&plaintext).map_err(|_| {
        AppError::new(
            "CREDENTIAL_STORE_ERROR",
            "Защищённые ключи службы повреждены.",
        )
    })
}
fn write_service_secrets(path: &Path, secrets: &ServiceSecrets) -> Result<(), AppError> {
    let plaintext = serde_json::to_vec(secrets).map_err(|_| {
        AppError::new(
            "CREDENTIAL_STORE_ERROR",
            "Не удалось подготовить защищённые ключи службы.",
        )
    })?;
    let encrypted = protect(&plaintext)?;
    let parent = path.parent().ok_or_else(|| {
        AppError::new(
            "CREDENTIAL_STORE_ERROR",
            "Путь защищённых ключей недопустим.",
        )
    })?;
    std::fs::create_dir_all(parent).map_err(|_| {
        AppError::new(
            "CREDENTIAL_STORE_ERROR",
            "Не удалось создать каталог ключей службы.",
        )
    })?;
    let temporary = path.with_extension("new");
    use std::io::Write;
    let write_result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(&encrypted)?;
        file.sync_all()
    })();
    write_result.map_err(|_| {
        AppError::new(
            "CREDENTIAL_STORE_ERROR",
            "Не удалось сохранить защищённые ключи службы.",
        )
    })?;
    std::fs::rename(temporary, path).map_err(|_| {
        AppError::new(
            "CREDENTIAL_STORE_ERROR",
            "Не удалось завершить сохранение ключей службы.",
        )
    })
}

/// Preserve leading zeros and byte order. Only documented formatting separators
/// and HID terminators are ignored; no HEX/DEC conversion or device-specific guessing.
pub fn normalize_card_code(raw: &str) -> Result<String, AppError> {
    if raw.len() > 256
        || raw.chars().any(|c| {
            !c.is_ascii_alphanumeric() && !matches!(c, ' ' | ':' | '-' | '\r' | '\n' | '\t')
        })
    {
        return Err(AppError::new(
            "INVALID_CARD_CODE",
            "Формат карты не поддерживается. Проверьте настройки считывателя.",
        ));
    }
    let value = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect::<String>();
    if value.is_empty() {
        return Err(AppError::new("INVALID_CARD_CODE", "Не получен код карты."));
    }
    Ok(value)
}
pub fn card_lookup_hash(raw: &str, secret: &str) -> Result<String, AppError> {
    let normalized = normalize_card_code(raw)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| AppError::new("CARD_SECRET_UNAVAILABLE", "Ключ карты недоступен."))?;
    mac.update(normalized.as_bytes());
    Ok(mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub fn payload_hash(payload: &str) -> String {
    use sha2::Digest;
    let digest = Sha256::digest(payload.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Stable entity IDs let an offline return reference the same loan after push.
pub fn operation_entity_id(operation: &str, kind: &str, index: usize) -> String {
    use sha2::Digest;
    let digest = Sha256::digest(format!("edus:v1:{operation}:{kind}:{index}").as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes).to_string()
}

#[cfg(test)]
fn test_secrets() -> &'static std::sync::Mutex<BTreeMap<(String, String), String>> {
    static STORE: std::sync::OnceLock<std::sync::Mutex<BTreeMap<(String, String), String>>> =
        std::sync::OnceLock::new();
    STORE.get_or_init(Default::default)
}

#[cfg(test)]
mod tests {
    use super::{card_lookup_hash, payload_hash};

    #[test]
    fn card_hash_normalizes_non_secret_formatting() {
        let secret = "fixture-secret";
        assert_eq!(
            card_lookup_hash(" edus- 001  ", secret).unwrap(),
            card_lookup_hash("EDUS001", secret).unwrap()
        );
        assert_ne!(
            card_lookup_hash("EDUS001", secret).unwrap(),
            card_lookup_hash("EDUS002", secret).unwrap()
        );
    }

    #[test]
    fn normalization_keeps_zeros_hex_order_and_rejects_unknown_device_format() {
        for raw in [
            "00:ab-CD 01",
            "00abcd01\r",
            "00ABCD01\n",
            "00ABCD01\t",
            "00ABCD01\r\n",
        ] {
            assert_eq!(super::normalize_card_code(raw).unwrap(), "00ABCD01");
        }
        assert!(super::normalize_card_code("\r\n\t").is_err());
        assert!(super::normalize_card_code("AB/12").is_err());
        assert!(super::normalize_card_code("AB12\u{1b}").is_err());
        assert_ne!(super::normalize_card_code("000123").unwrap(), "123");
    }
    #[test]
    fn missing_card_secret_fails_closed_without_creating_a_replacement() {
        let store = super::CredentialStore::named_test_production_fixture(
            &uuid::Uuid::new_v4().to_string(),
        );
        assert!(matches!(store.card_hmac_secret(), Err(e) if e.code=="CARD_SECRET_UNAVAILABLE"));
        assert!(store.read_optional("card-hmac-key-v1").unwrap().is_none());
    }

    #[test]
    fn payload_hash_is_deterministic_and_not_plaintext() {
        assert_eq!(payload_hash("operation"), payload_hash("operation"));
        assert_ne!(payload_hash("operation"), payload_hash("other-operation"));
        assert_ne!(payload_hash("operation"), "operation");
    }

    #[cfg(windows)]
    #[test]
    fn service_workspace_uses_dpapi_file_and_reopens_without_user_keyring() {
        let temp = tempfile::tempdir().unwrap();
        let store = super::CredentialStore::for_service_workspace(temp.path(), true);
        let database_key = store.database_key().unwrap();
        store
            .save_card_hmac_secret("service-card-secret-with-at-least-32-bytes")
            .unwrap();
        let protected_path = temp.path().join("UAT").join("secrets.dpapi");
        let stored = std::fs::read(&protected_path).unwrap();
        let readable = String::from_utf8_lossy(&stored);
        assert!(!readable.contains(&database_key));
        assert!(!readable.contains("database-key-v1"));
        let reopened = super::CredentialStore::for_service_workspace(temp.path(), true);
        assert_eq!(reopened.database_key().unwrap(), database_key);
        assert_eq!(
            reopened.card_hmac_secret().unwrap(),
            "service-card-secret-with-at-least-32-bytes"
        );
    }
}
