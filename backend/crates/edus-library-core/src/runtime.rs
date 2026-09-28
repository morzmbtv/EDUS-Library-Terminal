//! Runtime state owned exclusively by the local Windows service.
use crate::{
    card::CardDiagnostics,
    db::LocalDatabase,
    device::DeviceService,
    domain::AppError,
    face::FaceService,
    identity::{IdentityService, ReaderSession},
    security::CredentialStore,
    sync::{ProductionCloudHttpApi, SyncEngine},
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkspaceMode {
    Production,
    Uat,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceConfig {
    pub mode: WorkspaceMode,
    pub cloud_url: Option<String>,
    pub terminal_id: Option<String>,
    pub school_id: Option<String>,
    pub device_name: String,
}

pub struct Runtime {
    pub database: Mutex<LocalDatabase>,
    pub sync: Mutex<SyncEngine>,
    pub identity: IdentityService,
    pub card_diagnostics: CardDiagnostics,
    pub reader_session: Mutex<ReaderSession>,
    pub face: FaceService,
    pub device: DeviceService,
    pub credentials: CredentialStore,
    pub config: WorkspaceConfig,
}

impl Runtime {
    /// Opens a service-owned workspace. It never shares a kiosk user's Windows
    /// Credential Manager namespace with the service process.
    pub fn open_service(root: &std::path::Path, config: WorkspaceConfig) -> Result<Self, AppError> {
        let test = config.mode == WorkspaceMode::Uat;
        Self::open_with_credentials(
            root,
            config,
            CredentialStore::for_service_workspace(root, test),
        )
    }
    fn open_with_credentials(
        root: &std::path::Path,
        config: WorkspaceConfig,
        credentials: CredentialStore,
    ) -> Result<Self, AppError> {
        let test = config.mode == WorkspaceMode::Uat;
        let database = LocalDatabase::open(
            root.join(if test { "uat" } else { "production" })
                .join("library.db"),
            &credentials,
        )?;
        let transport = if test {
            None
        } else {
            Some(ProductionCloudHttpApi::configured(
                config
                    .cloud_url
                    .as_deref()
                    .ok_or_else(|| AppError::new("CLOUD_CONFIG_INVALID", "Укажите адрес Cloud."))?,
                &credentials,
                config.terminal_id.clone().ok_or_else(|| {
                    AppError::new("DEVICE_UNAUTHORIZED", "Подключите терминал к Cloud.")
                })?,
                config.school_id.clone().ok_or_else(|| {
                    AppError::new(
                        "SCHOOL_SCOPE_VIOLATION",
                        "Нет подтверждённой школы устройства.",
                    )
                })?,
            )?)
        };
        Ok(Self {
            database: Mutex::new(database),
            sync: Mutex::new(SyncEngine::with_production(transport)),
            identity: IdentityService::new(credentials.clone()),
            card_diagnostics: CardDiagnostics::new(credentials.clone()),
            reader_session: Mutex::new(ReaderSession::default()),
            face: FaceService::new(test),
            device: DeviceService::new(),
            credentials,
            config,
        })
    }
}
