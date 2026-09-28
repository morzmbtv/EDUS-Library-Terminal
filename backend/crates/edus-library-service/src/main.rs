//! EDUS Library loopback service used by Microsoft Edge Assigned Access.
//!
//! This binary is intentionally not a desktop application. It binds exclusively
//! to 127.0.0.1, owns local operational state, and exposes only explicit
//! library endpoints. It has no generic command, SQL, or filesystem API.
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use edus_library_core::{
    domain::{AppError, BasketItem, OperationResult, Snapshot},
    runtime::{Runtime, WorkspaceConfig, WorkspaceMode},
    services::LibraryService,
    sync::SyncEngine,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ffi::OsString,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Component, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{net::TcpListener, sync::oneshot};
use tokio_stream::{wrappers::IntervalStream, StreamExt};
use uuid::Uuid;

mod admin_ipc;
mod catalog;
#[cfg(test)]
mod contract_tests;
mod frontend;

const SERVICE_NAME: &str = "EDUSLibraryService";
const LISTEN: &str = "127.0.0.1:43180";
const ORIGIN: &str = "http://127.0.0.1:43180";
const COOKIE_NAME: &str = "edus_local_session";
const SESSION_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_BODY_BYTES: usize = 256 * 1024;
const CSP: &str = "default-src 'self'; connect-src 'self'; img-src 'self' blob:; media-src 'self' blob:; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'self'";

#[derive(Clone)]
struct ServiceState {
    admin_lock: Arc<Mutex<()>>,
    runtime: Arc<Mutex<Option<Arc<Runtime>>>>,
    startup_error: Arc<Mutex<Option<AppError>>>,
    sessions: Arc<Mutex<HashMap<String, BrowserSession>>>,
    web_root: Arc<PathBuf>,
    data_root: Arc<PathBuf>,
    stopping: Arc<AtomicBool>,
}

struct BrowserSession {
    csrf: String,
    expires: Instant,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Health {
    application: &'static str,
    pid: u32,
    /// `SETUP_REQUIRED` is deliberately distinct from a failed service.  A
    /// fresh installation has no active workspace until an administrator uses
    /// Configurator; Edge can keep retrying instead of showing a browser error.
    state: &'static str,
    ready: bool,
    mode: Option<WorkspaceMode>,
    version: &'static str,
    error: Option<AppError>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionInfo {
    csrf_token: String,
    expires_in_seconds: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeCapabilities {
    terminal_test: bool,
    uat: bool,
    test_namespace: Option<String>,
    build_label: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UiSettings {
    locale: String,
}

#[derive(Debug)]
struct ApiError(AppError, StatusCode);
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.1,
            [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
            Json(self.0),
        )
            .into_response()
    }
}
type ApiResult<T> = Result<Json<T>, ApiError>;
fn failure(code: &str, message: &str, status: StatusCode) -> ApiError {
    ApiError(AppError::new(code, message), status)
}
fn app_error(error: AppError) -> ApiError {
    ApiError(error, StatusCode::BAD_REQUEST)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodeRequest {
    code: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReaderRequest {
    reader_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueRequest {
    reader_id: String,
    items: Vec<BasketItem>,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReservationRequest {
    reader_id: String,
    title_id: String,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UiSettingsRequest {
    locale: String,
}

fn data_root() -> PathBuf {
    std::env::var_os("EDUS_SERVICE_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("ProgramData")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
                .join("EDUS Library")
        })
}
fn web_root() -> Result<PathBuf, &'static str> {
    if let Some(value) = std::env::var_os("EDUS_SERVICE_WEB_ROOT") {
        return Ok(PathBuf::from(value));
    }
    let executable = std::env::current_exe().map_err(|_| "DEPLOYMENT_CONFIG_INVALID")?;
    if let Some(root) = frontend::deployment_root(&executable)? {
        return Ok(root);
    }
    Ok(std::env::var_os("ProgramW6432")
        .or_else(|| std::env::var_os("ProgramFiles"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
        .join("EDUS Library Frontend"))
}
fn lifecycle_log(stage: &'static str, code: &'static str) {
    // Lifecycle only: no request bodies, card identifiers, SQL or credentials.
    let directory = data_root().join("logs");
    let result = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(&directory)?;
        let path = directory.join("service.jsonl");
        if std::fs::metadata(&path)
            .map(|m| m.len() > 1024 * 1024)
            .unwrap_or(false)
        {
            for index in (1..=3).rev() {
                let old = directory.join(format!("service.{index}.jsonl"));
                let next = directory.join(format!("service.{}.jsonl", index + 1));
                if old.exists() {
                    std::fs::rename(old, next)?;
                }
            }
            std::fs::rename(&path, directory.join("service.1.jsonl"))?;
        }
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?;
        writeln!(
            file,
            "{}",
            serde_json::json!({"time":format!("{:?}",std::time::SystemTime::now()),"pid":std::process::id(),"stage":stage,"code":code})
        )
    })();
    if result.is_err() {
        eprintln!("EDUS_LOG_WRITE_FAILED");
    }
}
fn read_runtime(root: &std::path::Path) -> Result<Option<Arc<Runtime>>, AppError> {
    let config_path = root.join("workspace.json");
    if !config_path.exists() {
        return Ok(None);
    }
    let config: WorkspaceConfig =
        serde_json::from_slice(&std::fs::read(config_path)?).map_err(|_| {
            AppError::new(
                "WORKSPACE_CONFIG_INVALID",
                "Не удалось прочитать настройки рабочего пространства.",
            )
        })?;
    Runtime::open_service(root, config).map(|runtime| Some(Arc::new(runtime)))
}
fn state() -> ServiceState {
    let root = data_root();
    // The browser never receives filesystem paths or service implementation
    // details. Detailed diagnostics belong in the service log/configurator.
    let open = std::fs::create_dir_all(&root)
        .map_err(|_| {
            AppError::new(
                "LOCAL_STORAGE_ERROR",
                "Не удалось открыть каталог данных службы.",
            )
        })
        .and_then(|_| read_runtime(&root));
    let (runtime, mut startup_error) = match open {
        Ok(runtime) => (runtime, None),
        Err(error) => (None, Some(error)),
    };
    let web_root = web_root().unwrap_or_else(|code| {
        startup_error = Some(AppError::new(
            code,
            "Не удалось прочитать настройки поставки EDUS.",
        ));
        // This is the running executable file, never a directory of static assets.
        std::env::current_exe().unwrap_or_else(|_| root.join("deployment.invalid"))
    });
    ServiceState {
        admin_lock: Arc::new(Mutex::new(())),
        runtime: Arc::new(Mutex::new(runtime)),
        startup_error: Arc::new(Mutex::new(startup_error)),
        sessions: Arc::new(Mutex::new(HashMap::new())),
        web_root: Arc::new(web_root),
        data_root: Arc::new(root),
        stopping: Arc::new(AtomicBool::new(false)),
    }
}
fn exact_host(headers: &HeaderMap) -> Result<(), ApiError> {
    if headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        != Some("127.0.0.1:43180")
    {
        return Err(failure(
            "LOCAL_ORIGIN_REJECTED",
            "Локальный запрос отклонён.",
            StatusCode::FORBIDDEN,
        ));
    }
    Ok(())
}
fn cookie(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(&format!("{COOKIE_NAME}=")))
}
fn session(state: &ServiceState, headers: &HeaderMap, mutation: bool) -> Result<(), ApiError> {
    exact_host(headers)?;
    if mutation
        && headers
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok())
            != Some(ORIGIN)
    {
        return Err(failure(
            "LOCAL_ORIGIN_REJECTED",
            "Локальный запрос отклонён.",
            StatusCode::FORBIDDEN,
        ));
    }
    if mutation
        && !headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| {
                value.eq_ignore_ascii_case("application/json")
                    || value.to_ascii_lowercase().starts_with("application/json;")
            })
    {
        return Err(failure(
            "CONTENT_TYPE_REJECTED",
            "Локальный запрос отклонён.",
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ));
    }
    let id = cookie(headers).ok_or_else(|| {
        failure(
            "LOCAL_SESSION_REQUIRED",
            "Сеанс локального терминала истёк. Откройте страницу снова.",
            StatusCode::UNAUTHORIZED,
        )
    })?;
    let mut sessions = state.sessions.lock().map_err(|_| {
        failure(
            "LOCAL_SERVICE_ERROR",
            "Локальная служба занята.",
            StatusCode::SERVICE_UNAVAILABLE,
        )
    })?;
    sessions.retain(|_, item| item.expires > Instant::now());
    let item = sessions.get(id).ok_or_else(|| {
        failure(
            "LOCAL_SESSION_REQUIRED",
            "Сеанс локального терминала истёк. Откройте страницу снова.",
            StatusCode::UNAUTHORIZED,
        )
    })?;
    if mutation
        && headers
            .get("x-edus-csrf")
            .and_then(|value| value.to_str().ok())
            != Some(item.csrf.as_str())
    {
        return Err(failure(
            "CSRF_REJECTED",
            "Локальный запрос отклонён.",
            StatusCode::FORBIDDEN,
        ));
    }
    Ok(())
}
fn with_runtime<T>(
    state: &ServiceState,
    action: impl FnOnce(&Runtime) -> Result<T, AppError>,
) -> Result<T, ApiError> {
    let guard = state.runtime.lock().map_err(|_| {
        failure(
            "LOCAL_SERVICE_ERROR",
            "Локальная служба занята.",
            StatusCode::SERVICE_UNAVAILABLE,
        )
    })?;
    let runtime = guard.as_ref().ok_or_else(|| {
        failure(
            "SETUP_REQUIRED",
            "Терминал ещё не настроен. Откройте EDUS Terminal Configurator.",
            StatusCode::SERVICE_UNAVAILABLE,
        )
    })?;
    action(runtime).map_err(app_error)
}
fn selected(runtime: &Runtime, reader_id: &str) -> Result<(), AppError> {
    runtime
        .reader_session
        .lock()
        .map_err(|_| AppError::new("READER_CONTEXT_ERROR", "Сеанс читателя недоступен."))?
        .require(reader_id)
}
fn snapshot(runtime: &Runtime) -> Result<Snapshot, AppError> {
    let online = runtime
        .sync
        .try_lock()
        .map(|sync| sync.online())
        .unwrap_or(false);
    let mut snapshot = runtime
        .database
        .lock()
        .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "Локальная база занята."))?
        .snapshot(online)?;
    let reader = runtime
        .reader_session
        .lock()
        .map_err(|_| AppError::new("READER_CONTEXT_ERROR", "Сеанс читателя недоступен."))?;
    snapshot
        .readers
        .retain(|item| Some(item.id.as_str()) == reader.selected());
    Ok(snapshot)
}

async fn version(headers: HeaderMap) -> ApiResult<serde_json::Value> {
    exact_host(&headers)?;
    Ok(Json(
        serde_json::json!({"backend_version":env!("CARGO_PKG_VERSION"),"local_api_version":frontend::API_VERSION,"minimum_frontend_version":frontend::FRONTEND_VERSION,"maximum_frontend_version":frontend::FRONTEND_VERSION}),
    ))
}
async fn health(
    State(state): State<ServiceState>,
    headers: HeaderMap,
) -> Result<Json<Health>, ApiError> {
    exact_host(&headers)?;
    let runtime = state.runtime.lock().map_err(|_| {
        failure(
            "LOCAL_SERVICE_ERROR",
            "Локальная служба занята.",
            StatusCode::SERVICE_UNAVAILABLE,
        )
    })?;
    let error = state
        .startup_error
        .lock()
        .ok()
        .and_then(|item| item.clone());
    let state_name = if error.is_some() {
        "ERROR"
    } else if frontend::load(&state.web_root)
        .and_then(|r| r.read("index.html"))
        .is_err()
    {
        "FRONTEND_MISSING"
    } else if runtime.is_some() {
        "READY"
    } else {
        "SETUP_REQUIRED"
    };
    Ok(Json(Health {
        application: SERVICE_NAME,
        pid: std::process::id(),
        state: state_name,
        ready: state_name == "READY",
        mode: runtime.as_ref().map(|item| item.config.mode),
        version: env!("CARGO_PKG_VERSION"),
        error,
    }))
}

async fn renew_session(
    State(state): State<ServiceState>,
    headers: HeaderMap,
) -> Result<axum::response::Response, ApiError> {
    exact_host(&headers)?;
    if headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) != Some(ORIGIN)
        || headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            != Some("application/json")
    {
        return Err(failure(
            "LOCAL_ORIGIN_REJECTED",
            "Локальный запрос отклонён.",
            StatusCode::FORBIDDEN,
        ));
    }
    let value = new_session(&state)?;
    let mut internal = headers.clone();
    internal.insert(
        header::COOKIE,
        HeaderValue::from_str(value.split(';').next().unwrap_or_default()).map_err(|_| {
            failure(
                "LOCAL_SERVICE_ERROR",
                "Сеанс недоступен.",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?,
    );
    let Json(info) = get_session(State(state), internal).await?;
    let mut response = Json(info).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&value).map_err(|_| {
            failure(
                "LOCAL_SERVICE_ERROR",
                "Сеанс недоступен.",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?,
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}
async fn get_session(
    State(state): State<ServiceState>,
    headers: HeaderMap,
) -> ApiResult<SessionInfo> {
    session(&state, &headers, false)?;
    let id = cookie(&headers).unwrap_or_default();
    let sessions = state.sessions.lock().map_err(|_| {
        failure(
            "LOCAL_SERVICE_ERROR",
            "Локальная служба занята.",
            StatusCode::SERVICE_UNAVAILABLE,
        )
    })?;
    let item = sessions.get(id).ok_or_else(|| {
        failure(
            "LOCAL_SESSION_REQUIRED",
            "Сеанс локального терминала истёк.",
            StatusCode::UNAUTHORIZED,
        )
    })?;
    Ok(Json(SessionInfo {
        csrf_token: item.csrf.clone(),
        expires_in_seconds: SESSION_TTL.as_secs(),
    }))
}
async fn get_snapshot(
    State(state): State<ServiceState>,
    headers: HeaderMap,
) -> ApiResult<Snapshot> {
    session(&state, &headers, false)?;
    with_runtime(&state, snapshot).map(Json)
}
async fn resolve_code(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(input): Json<CodeRequest>,
) -> ApiResult<edus_library_core::domain::CodeResult> {
    session(&state, &headers, true)?;
    with_runtime(&state, |runtime| {
        runtime
            .database
            .lock()
            .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "Локальная база занята."))?
            .resolve_code(&input.code)
    })
    .map(Json)
}
async fn identify_card(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(input): Json<CodeRequest>,
) -> ApiResult<edus_library_core::domain::IdentityResult> {
    session(&state, &headers, true)?;
    with_runtime(&state, |runtime| {
        let db = runtime
            .database
            .lock()
            .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "Локальная база занята."))?;
        let result = runtime.identity.card(&db, &input.code)?;
        runtime
            .reader_session
            .lock()
            .map_err(|_| AppError::new("READER_CONTEXT_ERROR", "Сеанс читателя недоступен."))?
            .remember(&result.reader.id);
        Ok(result)
    })
    .map(Json)
}
async fn reader_loans(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(input): Json<ReaderRequest>,
) -> ApiResult<Vec<edus_library_core::domain::Loan>> {
    session(&state, &headers, false)?;
    with_runtime(&state, |runtime| {
        selected(runtime, &input.reader_id)?;
        runtime
            .database
            .lock()
            .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "Локальная база занята."))?
            .reader_loans(&input.reader_id)
    })
    .map(Json)
}
async fn issue(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(input): Json<IssueRequest>,
) -> ApiResult<OperationResult> {
    session(&state, &headers, true)?;
    with_runtime(&state, |runtime| {
        selected(runtime, &input.reader_id)?;
        LibraryService::issue(
            &mut *runtime
                .database
                .lock()
                .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "Локальная база занята."))?,
            &input.reader_id,
            &input.items,
            &input.operation_id,
        )
    })
    .map(Json)
}
async fn accept(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(input): Json<IssueRequest>,
) -> ApiResult<OperationResult> {
    session(&state, &headers, true)?;
    with_runtime(&state, |runtime| {
        selected(runtime, &input.reader_id)?;
        LibraryService::accept(
            &mut *runtime
                .database
                .lock()
                .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "Локальная база занята."))?,
            &input.reader_id,
            &input.items,
            &input.operation_id,
        )
    })
    .map(Json)
}
async fn reserve(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(input): Json<ReservationRequest>,
) -> ApiResult<OperationResult> {
    session(&state, &headers, true)?;
    with_runtime(&state, |runtime| {
        selected(runtime, &input.reader_id)?;
        LibraryService::reserve(
            &mut *runtime
                .database
                .lock()
                .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "Локальная база занята."))?,
            &input.reader_id,
            &input.title_id,
            &input.operation_id,
        )
    })
    .map(Json)
}
async fn operation_result(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Path(operation_id): Path<String>,
) -> ApiResult<Option<OperationResult>> {
    session(&state, &headers, false)?;
    with_runtime(&state, |runtime| {
        runtime
            .database
            .lock()
            .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "Локальная база занята."))?
            .operation_result(&operation_id)
    })
    .map(Json)
}
async fn sync_status(
    State(state): State<ServiceState>,
    headers: HeaderMap,
) -> ApiResult<edus_library_core::domain::SyncStatus> {
    session(&state, &headers, false)?;
    with_runtime(&state, |runtime| {
        let db = runtime
            .database
            .lock()
            .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "Локальная база занята."))?;
        SyncEngine::status_for(&db, runtime.config.mode == WorkspaceMode::Production, false)
    })
    .map(Json)
}
async fn runtime_capabilities(
    State(state): State<ServiceState>,
    headers: HeaderMap,
) -> ApiResult<RuntimeCapabilities> {
    session(&state, &headers, false)?;
    with_runtime(&state, |runtime| {
        let uat = runtime.config.mode == WorkspaceMode::Uat;
        Ok(RuntimeCapabilities {
            terminal_test: uat,
            uat,
            test_namespace: uat.then(|| "edus.library.service.uat".into()),
            build_label: format!(
                "EDUS Library {} · Edge local service · UNSIGNED RC BUILD",
                env!("CARGO_PKG_VERSION")
            ),
        })
    })
    .map(Json)
}
fn ui_settings_path(state: &ServiceState) -> PathBuf {
    state.data_root.join("ui-settings.json")
}
fn read_ui_settings(state: &ServiceState) -> UiSettings {
    std::fs::read(ui_settings_path(state))
        .ok()
        .and_then(|raw| serde_json::from_slice::<UiSettings>(&raw).ok())
        .filter(|value| value.locale == "ru" || value.locale == "kk")
        .unwrap_or(UiSettings {
            locale: "ru".into(),
        })
}
async fn get_ui_settings(
    State(state): State<ServiceState>,
    headers: HeaderMap,
) -> ApiResult<UiSettings> {
    session(&state, &headers, false)?;
    Ok(Json(read_ui_settings(&state)))
}
async fn set_ui_settings(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(input): Json<UiSettingsRequest>,
) -> ApiResult<UiSettings> {
    session(&state, &headers, true)?;
    if input.locale != "ru" && input.locale != "kk" {
        return Err(failure(
            "INVALID_INPUT",
            "Выберите поддерживаемый язык интерфейса.",
            StatusCode::BAD_REQUEST,
        ));
    }
    let settings = UiSettings {
        locale: input.locale,
    };
    let path = ui_settings_path(&state);
    let temporary = path.with_extension("new.json");
    std::fs::write(
        &temporary,
        serde_json::to_vec(&settings).map_err(|_| {
            failure(
                "LOCAL_SERVICE_ERROR",
                "Не удалось сохранить настройку.",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?,
    )
    .map_err(|_| {
        failure(
            "LOCAL_STORAGE_ERROR",
            "Не удалось сохранить настройку.",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    std::fs::rename(temporary, path).map_err(|_| {
        failure(
            "LOCAL_STORAGE_ERROR",
            "Не удалось сохранить настройку.",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    Ok(Json(settings))
}
async fn events(
    State(state): State<ServiceState>,
    headers: HeaderMap,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>>, ApiError>
{
    session(&state, &headers, false)?;
    let interval = IntervalStream::new(tokio::time::interval(Duration::from_secs(5)));
    let end = state.stopping.clone();
    let stream = interval
        .take_while(move |_| !end.load(Ordering::Acquire))
        .map(move |_| {
            let data = match with_runtime(&state, |runtime| {
                let db = runtime
                    .database
                    .lock()
                    .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "Локальная база занята."))?;
                SyncEngine::status_for(&db, runtime.config.mode == WorkspaceMode::Production, false)
            }) {
                Ok(status) => serde_json::to_string(&status)
                    .unwrap_or_else(|_| "{\"state\":\"ERROR\"}".into()),
                Err(_) => "{\"state\":\"UNAVAILABLE\"}".into(),
            };
            Ok(Event::default().event("status").data(data))
        });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

fn content_type(path: &str) -> &'static str {
    if path.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    }
}
fn safe_file(root: &std::path::Path, request: &str) -> Option<PathBuf> {
    let relative = std::path::Path::new(request);
    if relative.components().any(|part| {
        matches!(
            part,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return None;
    }
    Some(root.join(relative))
}
fn new_session(state: &ServiceState) -> Result<String, ApiError> {
    let id = Uuid::new_v4().to_string();
    let csrf = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let mut sessions = state.sessions.lock().map_err(|_| {
        failure(
            "LOCAL_SERVICE_ERROR",
            "Локальная служба занята.",
            StatusCode::SERVICE_UNAVAILABLE,
        )
    })?;
    sessions.retain(|_, session| session.expires > Instant::now());
    if sessions.len() >= 128 {
        return Err(failure(
            "LOCAL_SESSION_LIMIT",
            "Повторите запрос позже.",
            StatusCode::TOO_MANY_REQUESTS,
        ));
    }
    sessions.insert(
        id.clone(),
        BrowserSession {
            csrf,
            expires: Instant::now() + SESSION_TTL,
        },
    );
    Ok(format!(
        "{COOKIE_NAME}={id}; Path=/; HttpOnly; SameSite=Strict"
    ))
}
async fn index(State(state): State<ServiceState>, headers: HeaderMap) -> axum::response::Response {
    if exact_host(&headers).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match frontend::load(&state.web_root).and_then(|r|r.read("index.html")) {
        Ok(body) => {
            let Ok(cookie) = new_session(&state) else {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            };
            let mut response = (
                [
                    (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                    (header::CACHE_CONTROL, "no-store"),
                    (header::CONTENT_SECURITY_POLICY, CSP),
                ],
                body,
            )
                .into_response();
            if let Ok(value) = HeaderValue::from_str(&cookie) {
                response.headers_mut().insert(header::SET_COOKIE, value);
            }
            response
        }
        Err(code) => (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::CONTENT_TYPE,"text/html; charset=utf-8"),(header::CONTENT_SECURITY_POLICY,CSP)],
            if code=="FRONTEND_INCOMPATIBLE" { "<!doctype html><html lang=ru><meta charset=utf-8><title>EDUS</title><h1>Версии компонентов EDUS несовместимы. Требуется обновление.</h1></html>" }
            else { "<!doctype html><html lang=ru><meta charset=utf-8><meta http-equiv=refresh content=5><title>EDUS</title><h1>Интерфейс EDUS не установлен</h1><p>Установите пакет Frontend. Служба и Configurator доступны.</p></html>" }
        ).into_response(),
    }
}
async fn asset(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Path(path): Path<String>,
) -> axum::response::Response {
    if exact_host(&headers).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    if safe_file(&state.web_root, &path).is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    match frontend::read_asset(&state.web_root, &path) {
        Ok(body) => (
            [
                (header::CONTENT_TYPE, content_type(&path)),
                (header::CACHE_CONTROL, "no-store"),
            ],
            body,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

fn router(state: ServiceState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/api/local/v1/version", get(version))
        .route("/api/local/v1/catalog/search", post(catalog::search))
        .route(
            "/api/local/v1/catalog/availability",
            post(catalog::availability),
        )
        .route("/api/local/v1/identify/face", post(catalog::face))
        .route("/api/local/v1/reservation/cancel", post(catalog::cancel))
        .route("/api/local/v1/session", get(get_session))
        .route("/api/local/v1/session/renew", post(renew_session))
        .route("/api/local/v1/snapshot", get(get_snapshot))
        .route("/api/local/v1/resolve-code", post(resolve_code))
        .route("/api/local/v1/identify/card", post(identify_card))
        .route("/api/local/v1/reader-loans", post(reader_loans))
        .route("/api/local/v1/issue", post(issue))
        .route("/api/local/v1/return", post(accept))
        .route("/api/local/v1/reservation", post(reserve))
        .route(
            "/api/local/v1/settings/ui",
            get(get_ui_settings).post(set_ui_settings),
        )
        .route(
            "/api/local/v1/runtime/capabilities",
            get(runtime_capabilities),
        )
        .route(
            "/api/local/v1/operations/{operation_id}",
            get(operation_result),
        )
        .route("/api/local/v1/sync/status", get(sync_status))
        .route("/api/local/v1/events", get(events))
        .route("/{*path}", get(asset))
        .with_state(state)
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(axum::middleware::from_fn(api_errors))
}

async fn api_errors(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let api = request.uri().path().starts_with("/api/local/v1/");
    let response = next.run(request).await;
    let status = response.status();
    if api
        && (status.is_client_error() || status.is_server_error())
        && !response
            .headers()
            .get(header::CONTENT_TYPE)
            .is_some_and(|value| value.as_bytes().starts_with(b"application/json"))
    {
        let code = match status {
            StatusCode::PAYLOAD_TOO_LARGE => "REQUEST_TOO_LARGE",
            StatusCode::UNSUPPORTED_MEDIA_TYPE => "UNSUPPORTED_CONTENT_TYPE",
            StatusCode::NOT_FOUND => "ENDPOINT_NOT_FOUND",
            _ => "INVALID_REQUEST",
        };
        return (
            status,
            Json(AppError::new(code, "Запрос не принят локальной службой.")),
        )
            .into_response();
    }
    response
}

fn start_sync_worker(state: ServiceState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while !state.stopping.load(Ordering::Acquire) {
            let current = state.clone();
            // Only one worker exists. All workspace leases finish before a switch.
            let _ = tokio::task::spawn_blocking(move || {
                if current.stopping.load(Ordering::Acquire) {
                    return;
                }
                if let Ok(slot) = current.runtime.lock() {
                    if let Some(runtime) = slot.as_ref() {
                        if runtime.config.mode == WorkspaceMode::Production {
                            if let Ok(mut sync) = runtime.sync.try_lock() {
                                let _ = sync.sync_shared(&runtime.database);
                            }
                        }
                    }
                }
            })
            .await;
            for _ in 0..20 {
                if state.stopping.load(Ordering::Acquire) {
                    return;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    })
}
async fn run_server(
    shutdown: Option<oneshot::Receiver<()>>,
    scm: Option<windows_service::service_control_handler::ServiceStatusHandle>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(LISTEN).await.inspect_err(|_| {
        lifecycle_log("bind", "LISTENER_BIND_FAILED");
    })?;
    let service_state = state();
    let pipe = admin_ipc::start(service_state.clone()).inspect_err(|_| {
        lifecycle_log("admin_ipc", "ADMIN_PIPE_START_FAILED");
    })?;
    lifecycle_log(
        "startup",
        if service_state
            .startup_error
            .lock()
            .map(|v| v.is_some())
            .unwrap_or(true)
        {
            "WORKSPACE_OPEN_FAILED"
        } else {
            "LISTENING"
        },
    );
    let worker = start_sync_worker(service_state.clone());
    if let Some(handle) = scm {
        report_status(handle, windows_service::service::ServiceState::Running, 0)?;
    }
    let stopping = service_state.clone();
    let app = router(service_state.clone());
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            match shutdown {
                Some(signal) => {
                    let _ = signal.await;
                }
                None => {
                    let _ = tokio::signal::ctrl_c().await;
                }
            }
            stopping.stopping.store(true, Ordering::Release);
            if let Some(handle) = scm {
                let _ = report_status(
                    handle,
                    windows_service::service::ServiceState::StopPending,
                    0,
                );
            }
        })
        .await;
    service_state.stopping.store(true, Ordering::Release);
    pipe.abort();
    let _ = worker.await;
    if let Ok(slot) = service_state.runtime.lock() {
        if let Some(runtime) = slot.as_ref() {
            let database = runtime
                .database
                .lock()
                .map_err(|_| "Database lock poisoned")?;
            database
                .checkpoint_for_maintenance()
                .map_err(|_| "WAL checkpoint failed")?;
        }
    }
    result?;
    lifecycle_log("shutdown", "CHECKPOINT_COMPLETE");
    Ok(())
}

/// Installer-only, bounded loopback health probe. It never opens a listener,
/// accepts no input beyond a fixed local request, and treats READY and
/// SETUP_REQUIRED as successful post-install service states.
fn installer_health_probe() -> Result<&'static str, String> {
    let address = LISTEN
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid loopback address: {error}"))?;
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))
        .map_err(|error| format!("loopback connection failed: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| format!("loopback read timeout setup failed: {error}"))?;
    stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1:43180\r\nConnection: close\r\n\r\n")
        .map_err(|error| format!("health request failed: {error}"))?;
    let mut response = String::new();
    stream
        .take(16385)
        .read_to_string(&mut response)
        .map_err(|error| format!("health response failed: {error}"))?;
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "health response has no HTTP body".to_owned())?;
    if !headers.starts_with("HTTP/1.1 200") {
        return Err(format!("health HTTP response was not 200: {headers}"));
    }
    let payload: serde_json::Value =
        serde_json::from_str(body).map_err(|error| format!("invalid health JSON: {error}"))?;
    if response.len() > 16384 || payload["application"] != SERVICE_NAME {
        return Err("Invalid EDUS health identity".into());
    }
    match payload.get("state").and_then(serde_json::Value::as_str) {
        Some("READY") => Ok("READY"),
        Some("SETUP_REQUIRED") => Ok("SETUP_REQUIRED"),
        Some(state) => Err(format!("health state is {state}")),
        None => Err("health response has no state".to_owned()),
    }
}
#[cfg(windows)]
windows_service::define_windows_service!(ffi_service_main, service_main);

#[cfg(windows)]
fn report_status(
    handle: windows_service::service_control_handler::ServiceStatusHandle,
    state: windows_service::service::ServiceState,
    code: u32,
) -> windows_service::Result<()> {
    use windows_service::service::*;
    let pending = matches!(
        state,
        ServiceState::StartPending | ServiceState::StopPending
    );
    handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: if state == ServiceState::Running {
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN
        } else {
            ServiceControlAccept::empty()
        },
        exit_code: ServiceExitCode::Win32(code),
        checkpoint: if pending { 1 } else { 0 },
        wait_hint: if pending {
            Duration::from_secs(60)
        } else {
            Duration::ZERO
        },
        process_id: None,
    })
}
#[cfg(windows)]
fn service_main(_: Vec<OsString>) {
    use windows_service::{
        service::{ServiceControl, ServiceState as ScmState},
        service_control_handler::{self, ServiceControlHandlerResult},
    };
    let (tx, rx) = oneshot::channel();
    let sender = Mutex::new(Some(tx));
    let handler =
        match service_control_handler::register(SERVICE_NAME, move |control| match control {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                if let Some(tx) = sender.lock().ok().and_then(|mut item| item.take()) {
                    let _ = tx.send(());
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }) {
            Ok(handle) => handle,
            Err(_) => return,
        };
    if report_status(handler, ScmState::StartPending, 0).is_err() {
        return;
    }
    // ServiceMain must remain alive until workers drain and WAL is checkpointed.
    let result = tokio::runtime::Runtime::new()
        .map_err(|_| ())
        .and_then(|runtime| {
            runtime
                .block_on(run_server(Some(rx), Some(handler)))
                .map_err(|_| ())
        });
    let _ = report_status(
        handler,
        ScmState::Stopped,
        if result.is_ok() { 0 } else { 1064 },
    );
}
fn main() {
    let args: Vec<OsString> = std::env::args_os().collect();
    if args.iter().any(|value| value == "--installer-health") {
        match installer_health_probe() {
            Ok(state) => {
                println!("{state}");
                return;
            }
            Err(error) => {
                eprintln!("EDUS installer health probe failed: {error}");
                std::process::exit(1);
            }
        }
    }
    let console = args.iter().any(|value| value == "--console");
    #[cfg(windows)]
    if !console {
        if windows_service::service_dispatcher::start(SERVICE_NAME, ffi_service_main).is_ok() {
            return;
        }
        eprintln!("EDUSLibraryService must be started by Windows Service Control Manager or with --console for an explicit local test.");
        std::process::exit(1);
    }
    let runtime = tokio::runtime::Runtime::new().expect("EDUS service runtime");
    if let Err(error) = runtime.block_on(run_server(None, None)) {
        eprintln!("EDUS local service failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use edus_library_core::{domain::TestReaderInput, security::CredentialStore};
    use tower::ServiceExt;

    pub(super) fn fixture() -> (tempfile::TempDir, ServiceState) {
        let dir = tempfile::tempdir().unwrap();
        let state = ServiceState {
            admin_lock: Arc::new(Mutex::new(())),
            runtime: Arc::new(Mutex::new(None)),
            startup_error: Arc::new(Mutex::new(None)),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            web_root: Arc::new(dir.path().join("www")),
            data_root: Arc::new(dir.path().to_path_buf()),
            stopping: Arc::new(AtomicBool::new(false)),
        };
        (dir, state)
    }
    pub(super) async fn call(
        state: &ServiceState,
        path: &str,
        body: serde_json::Value,
        cookie: &str,
        csrf: &str,
    ) -> (StatusCode, serde_json::Value) {
        let response = router(state.clone())
            .oneshot(
                Request::post(path)
                    .header("host", "127.0.0.1:43180")
                    .header("origin", ORIGIN)
                    .header("content-type", "application/json")
                    .header("cookie", cookie)
                    .header("x-edus-csrf", csrf)
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    pub(super) async fn auth(state: &ServiceState) -> (String, String) {
        let response = router(state.clone())
            .oneshot(
                Request::post("/api/local/v1/session/renew")
                    .header("host", "127.0.0.1:43180")
                    .header("origin", ORIGIN)
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"));
        let data: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        (cookie, data["csrfToken"].as_str().unwrap().into())
    }
    #[tokio::test]
    async fn session_boundary_and_renewal() {
        let (_dir, state) = fixture();
        for (host, origin) in [
            ("evil.test", ORIGIN),
            ("127.0.0.1:43180", "https://evil.test"),
        ] {
            let response = router(state.clone())
                .oneshot(
                    Request::post("/api/local/v1/session/renew")
                        .header("host", host)
                        .header("origin", origin)
                        .header("content-type", "application/json")
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
            assert!(!response
                .headers()
                .contains_key("access-control-allow-origin"));
        }
        let (cookie, csrf) = auth(&state).await;
        let (status, _) = call(
            &state,
            "/api/local/v1/settings/ui",
            serde_json::json!({"locale":"kk"}),
            &cookie,
            "wrong",
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(!state.data_root.join("ui-settings.json").exists());
        for session in state.sessions.lock().unwrap().values_mut() {
            session.expires = Instant::now() - Duration::from_secs(1);
        }
        assert_eq!(
            call(
                &state,
                "/api/local/v1/settings/ui",
                serde_json::json!({"locale":"kk"}),
                &cookie,
                &csrf
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
        let (cookie, csrf) = auth(&state).await;
        assert_eq!(
            call(
                &state,
                "/api/local/v1/settings/ui",
                serde_json::json!({"locale":"kk"}),
                &cookie,
                &csrf
            )
            .await
            .0,
            StatusCode::OK
        );
        assert_eq!(read_ui_settings(&state).locale, "kk");
    }
    #[tokio::test]
    async fn body_limits_and_content_type() {
        let (_dir, state) = fixture();
        let (cookie, csrf) = auth(&state).await;
        let big = "x".repeat(MAX_BODY_BYTES + 1);
        let response = router(state.clone())
            .oneshot(
                Request::post("/api/local/v1/identify/card")
                    .header("host", "127.0.0.1:43180")
                    .header("origin", ORIGIN)
                    .header("cookie", &cookie)
                    .header("x-edus-csrf", &csrf)
                    .header("content-type", "application/json")
                    .body(Body::from(big))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let response = router(state)
            .oneshot(
                Request::post("/api/local/v1/identify/card")
                    .header("host", "127.0.0.1:43180")
                    .header("content-type", "text/plain")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    #[test]
    fn traversal_rejected() {
        for path in [
            r"..\secrets.dpapi",
            r"C:\secrets.dpapi",
            "../library.db",
            r"\server\share",
        ] {
            assert!(safe_file(std::path::Path::new(r"E:\www"), path).is_none());
        }
    }
    #[test]
    #[ignore = "Explicit development browser fixture on E; not compiled into runtime"]
    fn prepare_browser_fixture() {
        let root = PathBuf::from(r"E:\Codex\temp\edus-split-2.0-browser-test");
        assert!(!root.exists(), "Do not overwrite an existing fixture");
        std::fs::create_dir_all(&root).unwrap();
        let config = WorkspaceConfig {
            mode: WorkspaceMode::Uat,
            cloud_url: None,
            terminal_id: None,
            school_id: None,
            device_name: "EDUS Audit UAT".into(),
        };
        let runtime = Runtime::open_service(&root, config.clone()).unwrap();
        let reader = runtime
            .database
            .lock()
            .unwrap()
            .create_test_reader(
                &TestReaderInput {
                    external_id: "AUDIT-CARD".into(),
                    full_name: "Тестовый читатель аудита".into(),
                    person_type: "STUDENT".into(),
                    class_name: Some("7 Т".into()),
                    position_name: None,
                    status: "ACTIVE".into(),
                },
                false,
            )
            .unwrap()
            .reader;
        runtime
            .database
            .lock()
            .unwrap()
            .bind_test_reader_card(
                &reader.id,
                "00001009",
                &runtime.credentials.card_hmac_secret().unwrap(),
            )
            .unwrap();
        runtime
            .database
            .lock()
            .unwrap()
            .checkpoint_for_maintenance()
            .unwrap();
        std::fs::write(
            root.join("workspace.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
    }
    #[tokio::test]
    async fn uat_dpapi_card_offline_operations_and_reopen() {
        let (_dir, state) = fixture();
        let config = WorkspaceConfig {
            mode: WorkspaceMode::Uat,
            cloud_url: None,
            terminal_id: None,
            school_id: None,
            device_name: "Test".into(),
        };
        let runtime = Runtime::open_service(&state.data_root, config.clone()).unwrap();
        let secret = runtime.credentials.card_hmac_secret().unwrap();
        let reader = runtime
            .database
            .lock()
            .unwrap()
            .create_test_reader(
                &TestReaderInput {
                    external_id: "audit-001".into(),
                    full_name: "Тест аудита".into(),
                    person_type: "STUDENT".into(),
                    class_name: Some("7 Т".into()),
                    position_name: None,
                    status: "ACTIVE".into(),
                },
                false,
            )
            .unwrap()
            .reader;
        runtime
            .database
            .lock()
            .unwrap()
            .bind_test_reader_card(&reader.id, "00001009\r\n", &secret)
            .unwrap();
        let copy = runtime
            .database
            .lock()
            .unwrap()
            .test_books()
            .unwrap()
            .into_iter()
            .find(|b| b.status == "AVAILABLE")
            .unwrap();
        *state.runtime.lock().unwrap() = Some(Arc::new(runtime));
        let (cookie, csrf) = auth(&state).await;
        let identity = call(
            &state,
            "/api/local/v1/identify/card",
            serde_json::json!({"code":"00001009\t"}),
            &cookie,
            &csrf,
        )
        .await;
        assert_eq!(identity.0, StatusCode::OK, "{:?}", identity.1);
        assert_eq!(identity.1["reader"]["id"], reader.id);
        let item = serde_json::json!({"id":"scan","titleId":copy.title_id,"copyId":copy.id,"quantity":1,"mode":"COPY","loanId":null});
        let op = Uuid::new_v4().to_string();
        let body = serde_json::json!({"readerId":reader.id,"items":[item],"operationId":op});
        let issued = call(&state, "/api/local/v1/issue", body.clone(), &cookie, &csrf).await;
        assert_eq!(issued.0, StatusCode::OK, "{:?}", issued.1);
        assert_eq!(
            call(&state, "/api/local/v1/issue", body, &cookie, &csrf)
                .await
                .1,
            issued.1
        );
        let old = state.runtime.lock().unwrap().take().unwrap();
        old.database
            .lock()
            .unwrap()
            .checkpoint_for_maintenance()
            .unwrap();
        drop(old);
        let reopened = Runtime::open_service(&state.data_root, config).unwrap();
        assert_eq!(reopened.credentials.card_hmac_secret().unwrap(), secret);
        assert_eq!(
            reopened
                .identity
                .card(&reopened.database.lock().unwrap(), "00001009")
                .unwrap()
                .reader
                .id,
            reader.id
        );
        let loans = reopened
            .database
            .lock()
            .unwrap()
            .reader_loans(&reader.id)
            .unwrap();
        assert_eq!(loans.len(), 1);
        let loan_id = loans[0].id.clone();
        assert!(
            !CredentialStore::for_service_workspace(&state.data_root, false)
                .existing_database_key()
                .is_ok()
        );
        *state.runtime.lock().unwrap() = Some(Arc::new(reopened));
        call(
            &state,
            "/api/local/v1/identify/card",
            serde_json::json!({"code":"00001009"}),
            &cookie,
            &csrf,
        )
        .await;
        let returned=call(&state,"/api/local/v1/return",serde_json::json!({"readerId":reader.id,"items":[{"id":"return","titleId":copy.title_id,"copyId":copy.id,"quantity":1,"mode":"COPY","loanId":loan_id}],"operationId":Uuid::new_v4().to_string()}),&cookie,&csrf).await;
        assert_eq!(returned.0, StatusCode::OK, "{:?}", returned.1);
        assert!(state
            .runtime
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .database
            .lock()
            .unwrap()
            .reader_loans(&reader.id)
            .unwrap()
            .is_empty());
    }
}
