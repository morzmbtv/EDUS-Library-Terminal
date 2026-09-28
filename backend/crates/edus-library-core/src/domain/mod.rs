use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reader {
    pub id: String,
    pub name: String,
    pub group: String,
    pub card: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Title {
    pub id: String,
    pub name: String,
    pub author: String,
    pub isbn: String,
    pub publisher: String,
    pub year: i32,
    pub language: String,
    pub subject: String,
    pub grade: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Copy {
    pub id: String,
    pub title_id: String,
    pub code: String,
    pub status: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Loan {
    pub id: String,
    pub reader_id: String,
    pub title_id: String,
    pub copy_id: Option<String>,
    pub quantity: i32,
    pub mode: String,
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reservation {
    pub id: String,
    pub reader_id: String,
    pub title_id: String,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BasketItem {
    pub id: String,
    pub title_id: String,
    pub copy_id: Option<String>,
    pub quantity: i32,
    pub mode: String,
    pub loan_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult {
    pub operation_id: String,
    #[serde(rename = "type")]
    pub operation_type: String,
    pub quantity: i32,
    pub title_id: Option<String>,
    pub copy_ids: Vec<String>,
    pub loan_ids: Vec<String>,
    pub reservation_id: Option<String>,
    pub queue_position: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookAvailability {
    pub book_title_id: String,
    pub available_copies: i32,
    pub available_legacy_quantity: i32,
    pub waiting_reservations: i32,
    pub nearest_expected_return: Option<String>,
    pub has_overdue: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedReturn {
    pub nearest_due_at: Option<String>,
    pub has_overdue: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub server: bool,
    pub internet: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub readers: Vec<Reader>,
    pub titles: Vec<Title>,
    pub copies: Vec<Copy>,
    pub loans: Vec<Loan>,
    pub reservations: Vec<Reservation>,
    pub legacy_stock: std::collections::HashMap<String, i32>,
    pub connection: Connection,
    pub persistence: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeResult {
    pub kind: String,
    pub copy: Option<Copy>,
    pub title: Option<Title>,
    pub loan: Option<Loan>,
    pub titles: Vec<Title>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityResult {
    pub person_id: String,
    pub method: String,
    pub confidence_class: Option<String>,
    pub timestamp: String,
    pub reader: Reader,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    /// READY, OFFLINE_QUEUE, SYNCING or CONFLICT. The UI may surface this in
    /// diagnostics; it is deliberately not a permanent workflow status bar.
    pub state: String,
    pub local_ready: bool,
    pub online: bool,
    pub pending_count: i64,
    pub conflict_count: i64,
    pub last_sync_at: Option<String>,
    pub last_error: Option<String>,
    pub test_cloud: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub audit_write_failed: bool,
    pub local_database: String,
    pub cipher_version: String,
    pub memory_hardening: String,
    pub database_health: String,
    pub disk_available_bytes: u64,
    pub low_disk: bool,
    pub update_policy: String,
    pub version: String,
    pub cloud_url: Option<String>,
    pub sqlcipher: bool,
    pub sync: SyncStatus,
    pub card_reader: String,
    pub camera: String,
    pub face_engine: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilities {
    pub terminal_test: bool,
    pub uat: bool,
    pub test_namespace: Option<String>,
    pub build_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestReaderInput {
    pub external_id: String,
    pub full_name: String,
    pub person_type: String,
    pub class_name: Option<String>,
    pub position_name: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestReaderRecord {
    pub id: String,
    pub external_id: String,
    pub full_name: String,
    pub person_type: String,
    pub class_name: Option<String>,
    pub position_name: Option<String>,
    pub status: String,
    pub card_bound: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestBookRecord {
    pub id: String,
    pub title_id: String,
    pub title: String,
    pub inventory_number: String,
    pub barcode: Option<String>,
    pub isbn: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestReaderUpsert {
    pub action: String,
    pub reader: TestReaderRecord,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCsvPreviewRow {
    pub line: usize,
    pub external_id: String,
    pub full_name: String,
    pub status: String,
    pub result: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCsvPreview {
    pub rows: Vec<TestCsvPreviewRow>,
    pub valid_count: usize,
    pub create_count: usize,
    pub update_count: usize,
    pub skipped_count: usize,
    pub error_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            operation_id: None,
        }
    }
}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for AppError {}
impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        match error {
            rusqlite::Error::SqliteFailure(code, _)
                if code.code == rusqlite::ErrorCode::DiskFull =>
            {
                Self::new(
                    "DISK_FULL",
                    "Запись не завершена: диск заполнен. Данные операции не удалены.",
                )
            }
            rusqlite::Error::SqliteFailure(code, _)
                if matches!(
                    code.code,
                    rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                ) =>
            {
                Self::new(
                    "DATABASE_BUSY",
                    "База занята. Дождитесь завершения операции.",
                )
            }
            _ => Self::new(
                "LOCAL_DATABASE_ERROR",
                "Не удалось прочитать или записать локальную базу. Обратитесь к администратору.",
            ),
        }
    }
}
impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::new("LOCAL_STORAGE_ERROR", error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(_: serde_json::Error) -> Self {
        Self::new("INVALID_INPUT", "Не удалось обработать данные операции.")
    }
}
