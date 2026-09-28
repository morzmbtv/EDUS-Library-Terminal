//! Narrow administrator-only IPC for EDUS Terminal Configurator.
//!
//! This pipe is deliberately separate from the Edge HTTP listener.  It accepts
//! a bounded JSON request, verifies the connecting Windows token belongs to the
//! well-known BUILTIN\\Administrators SID, and dispatches only named domain
//! operations.  There is no SQL, filesystem, registry, or process command.

use crate::ServiceState;
use edus_library_core::{
    domain::{AppError, TestReaderInput},
    runtime::{Runtime, WorkspaceConfig, WorkspaceMode},
    security::CredentialStore,
    sync::ProductionCloudHttpApi,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{io, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
};

use edus_library_admin_ipc::{Request as AdminRequest, MAX_MESSAGE_BYTES, PIPE_NAME};
type AdminResponse = edus_library_admin_ipc::Response<AppError>;

fn create_pipe(first: bool) -> io::Result<NamedPipeServer> {
    use windows::{
        core::w,
        Win32::{
            Foundation::{LocalFree, HLOCAL},
            Security::{
                Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW,
                PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
            },
        },
    };
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            w!("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;LS)"),
            1,
            &mut descriptor,
            None,
        )
        .map_err(io::Error::other)?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: false.into(),
        };
        let result = ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .create_with_security_attributes_raw(PIPE_NAME, &attributes as *const _ as *mut _);
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
        result
    }
}
pub(crate) fn start(state: ServiceState) -> io::Result<tokio::task::JoinHandle<()>> {
    let mut server = create_pipe(true)?;
    Ok(tokio::spawn(async move {
        loop {
            if server.connect().await.is_err() {
                return;
            }
            // Keep the name owned while publishing the next listening instance.
            let next = match create_pipe(false) {
                Ok(next) => next,
                Err(_) => return,
            };
            let connected = server;
            server = next;
            let current = state.clone();
            tokio::spawn(async move {
                let _ = tokio::time::timeout(
                    std::time::Duration::from_secs(180),
                    handle(connected, current),
                )
                .await;
            });
        }
    }))
}

async fn handle(mut pipe: NamedPipeServer, state: ServiceState) -> io::Result<()> {
    let mut size = [0u8; 4];
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        pipe.read_exact(&mut size),
    )
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "admin request header timeout"))??;
    if !caller_is_windows_administrator(&pipe) {
        return write_response(
            &mut pipe,
            AdminResponse {
                request_id: String::new(),
                ok: false,
                result: None,
                error: Some(AppError::new(
                    "ADMIN_ACCESS_DENIED",
                    "Не удалось подтвердить права администратора Windows.",
                )),
            },
        )
        .await;
    }

    let length = u32::from_le_bytes(size) as usize;
    if length == 0 || length > MAX_MESSAGE_BYTES {
        return write_response(
            &mut pipe,
            AdminResponse {
                request_id: String::new(),
                ok: false,
                result: None,
                error: Some(AppError::new(
                    "ADMIN_IPC_INVALID_SIZE",
                    "Недопустимый размер административного запроса.",
                )),
            },
        )
        .await;
    }
    let mut raw = vec![0u8; length];
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        pipe.read_exact(&mut raw),
    )
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "admin request body timeout"))??;
    let request: AdminRequest = match serde_json::from_slice(&raw) {
        Ok(request) => request,
        Err(_) => {
            return write_response(
                &mut pipe,
                AdminResponse {
                    request_id: String::new(),
                    ok: false,
                    result: None,
                    error: Some(AppError::new(
                        "ADMIN_IPC_INVALID_REQUEST",
                        "Недопустимый административный запрос.",
                    )),
                },
            )
            .await
        }
    };
    let request_id = request.request_id.clone();
    let result = if request.protocol_version != edus_library_admin_ipc::PROTOCOL_VERSION
        || request.request_id.is_empty()
        || !edus_library_admin_ipc::allowed(&request.command)
    {
        Err(AppError::new(
            "ADMIN_IPC_PROTOCOL",
            "Несовместимая версия Configurator.",
        ))
    } else {
        tokio::task::spawn_blocking(move || dispatch(state, request))
            .await
            .unwrap_or_else(|_| {
                Err(AppError::new(
                    "ADMIN_OPERATION_FAILED",
                    "Административная операция прервана.",
                ))
            })
    };
    write_response(
        &mut pipe,
        match result {
            Ok(value) => AdminResponse {
                request_id,
                ok: true,
                result: Some(value),
                error: None,
            },
            Err(error) => AdminResponse {
                request_id,
                ok: false,
                result: None,
                error: Some(error),
            },
        },
    )
    .await
}

async fn write_response(pipe: &mut NamedPipeServer, response: AdminResponse) -> io::Result<()> {
    let raw = serde_json::to_vec(&response).map_err(io::Error::other)?;
    pipe.write_all(&(raw.len() as u32).to_le_bytes()).await?;
    pipe.write_all(&raw).await?;
    pipe.flush().await
}

fn dispatch(state: ServiceState, request: AdminRequest) -> Result<Value, AppError> {
    let _admin = state.admin_lock.lock().map_err(|_| {
        AppError::new(
            "ADMIN_OPERATION_FAILED",
            "Административная операция прервана.",
        )
    })?;
    match request.command.as_str() {
        "GetServiceStatus" => Ok(
            json!({ "service": "EDUSLibraryService", "listen": "127.0.0.1:43180", "ready": state.runtime.lock().map_err(|_| AppError::new("LOCAL_SERVICE_ERROR", "Служба занята."))?.is_some() }),
        ),
        "GetWorkspaceStatus" => {
            let runtime = state
                .runtime
                .lock()
                .map_err(|_| AppError::new("LOCAL_SERVICE_ERROR", "Служба занята."))?;
            Ok(
                json!({ "configured": runtime.is_some(), "mode": runtime.as_ref().map(|item| item.config.mode), "version": env!("CARGO_PKG_VERSION") }),
            )
        }
        "SetActiveWorkspace" => {
            let mut config: WorkspaceConfig =
                serde_json::from_value(request.payload).map_err(|_| {
                    AppError::new("INVALID_INPUT", "Недопустимые параметры рабочей области.")
                })?;
            let saved = state.data_root.join(if config.mode == WorkspaceMode::Uat { "uat" } else { "production" }).join("workspace.json");
            if saved.exists() { config = serde_json::from_slice(&std::fs::read(saved)?).map_err(|_|AppError::new("WORKSPACE_CONFIG_INVALID","Сохранённая конфигурация повреждена."))?; }
            else if config.mode == WorkspaceMode::Production { return Err(AppError::new("ENROLLMENT_REQUIRED","Сначала подключите Production через enrollment.")); }
            replace_workspace(&state, config)?;
            Ok(json!({ "restarted": true }))
        }
        "SetCloudConfiguration" => Err(AppError::new("CONFIRMATION_REQUIRED", "Смена Cloud действующего устройства требует отдельного подтверждённого переноса. Enrollment и данные сохранены.")),
        "ListUatData" => with_uat(&state, |runtime| {
            let db = runtime
                .database
                .lock()
                .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "База занята."))?;
            Ok(json!({"readers":db.test_readers()?, "books":db.test_books()?}))
        }),
        "PreviewUatReaders" => with_uat(&state, |runtime| {
            let csv = request.payload["csv"]
                .as_str()
                .ok_or_else(|| AppError::new("INVALID_INPUT", "Не получен CSV."))?;
            let result = runtime
                .database
                .lock()
                .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "База занята."))?
                .preview_test_readers_csv(csv)?;
            serde_json::to_value(result)
                .map_err(|_| AppError::new("LOCAL_SERVICE_ERROR", "Ответ недоступен."))
        }),
        "CreateUatBook" => with_uat(&state, |runtime| {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Input {
                title: edus_library_core::domain::Title,
                mode: String,
                codes: Vec<String>,
                quantity: i32,
                operation_id: String,
            }
            let input: Input = serde_json::from_value(request.payload)
                .map_err(|_| AppError::new("INVALID_INPUT", "Проверьте данные книги."))?;
            let result = runtime
                .database
                .lock()
                .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "База занята."))?
                .register(
                    &input.title,
                    &input.mode,
                    &input.codes,
                    input.quantity,
                    &input.operation_id,
                )?;
            serde_json::to_value(result)
                .map_err(|_| AppError::new("LOCAL_SERVICE_ERROR", "Ответ недоступен."))
        }),
        "CreateUatReader" => with_uat(&state, |runtime| {
            let input: TestReaderInput = serde_json::from_value(request.payload).map_err(|_| {
                AppError::new("INVALID_INPUT", "Недопустимые данные тестового читателя.")
            })?;
            let item = runtime
                .database
                .lock()
                .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "База занята."))?
                .create_test_reader(&input, false)?;
            serde_json::to_value(item)
                .map_err(|_| AppError::new("LOCAL_SERVICE_ERROR", "Не удалось подготовить ответ."))
        }),
        "ImportUatReaders" => with_uat(&state, |runtime| {
            let csv = request
                .payload
                .get("csv")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::new("INVALID_INPUT", "Не получен CSV."))?;
            let preview = runtime
                .database
                .lock()
                .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "База занята."))?
                .apply_test_readers_csv(csv)?;
            serde_json::to_value(preview)
                .map_err(|_| AppError::new("LOCAL_SERVICE_ERROR", "Не удалось подготовить ответ."))
        }),
        "BindUatCard" => with_uat(&state, |runtime| {
            let reader_id = request
                .payload
                .get("readerId")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::new("INVALID_INPUT", "Не выбран читатель."))?;
            let card = request
                .payload
                .get("card")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::new("INVALID_INPUT", "Не получен код карты."))?;
            let mut database = runtime
                .database
                .lock()
                .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "База занята."))?;
            let secret = runtime.credentials.card_hmac_secret()?;
            database.bind_test_reader_card(reader_id, card, &secret)?;
            Ok(json!({ "bound": true }))
        }),
        "BindUatScannerCode" => with_uat(&state, |runtime| {
            let copy_id = request
                .payload
                .get("copyId")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::new("INVALID_INPUT", "Не выбран экземпляр."))?;
            let code = request
                .payload
                .get("code")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::new("INVALID_INPUT", "Не получен код сканера."))?;
            runtime
                .database
                .lock()
                .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "База занята."))?
                .bind_test_book_code(copy_id, code)?;
            Ok(json!({ "bound": true }))
        }),
        "ResetUatData" => with_uat(&state, |runtime| {
            if request.payload["confirmation"] != "RESET UAT" {
                return Err(AppError::new(
                    "CONFIRMATION_REQUIRED",
                    "Подтвердите сброс UAT.",
                ));
            }
            runtime
                .database
                .lock()
                .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "База занята."))?
                .reset_test_data()?;
            Ok(json!({ "reset": true }))
        }),
        "CreateBackup" => with_runtime(&state, |runtime| {
            let path = runtime
                .database
                .lock()
                .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "База занята."))?
                .encrypted_backup(&runtime.credentials)?;
            Ok(
                json!({ "created": true, "fileName": path.file_name().and_then(|item| item.to_str()).unwrap_or("backup.db") }),
            )
        }),
        "ListBackups" => with_runtime(&state,|runtime|{
            serde_json::to_value(runtime.database.lock().map_err(|_|AppError::new("LOCAL_DATABASE_ERROR","База занята."))?.list_backups()?).map_err(|_|AppError::new("LOCAL_SERVICE_ERROR","Ответ недоступен."))
        }),
        "RestoreBackup" => {
            if request.payload["confirmation"]!="RESTORE UAT" {return Err(AppError::new("CONFIRMATION_REQUIRED","Подтвердите замену UAT из резервной копии."));}
            let file=request.payload["fileName"].as_str().ok_or_else(||AppError::new("INVALID_INPUT","Выберите резервную копию."))?;
            let result=with_uat(&state,|runtime|{
                let result=runtime.database.lock().map_err(|_|AppError::new("LOCAL_DATABASE_ERROR","База занята."))?.restore_backup(file,&runtime.credentials)?;
                runtime.reader_session.lock().map_err(|_|AppError::new("LOCAL_SERVICE_ERROR","Сеанс занят."))?.clear();
                serde_json::to_value(result).map_err(|_|AppError::new("LOCAL_SERVICE_ERROR","Ответ недоступен."))
            })?;
            state.sessions.lock().map_err(|_|AppError::new("LOCAL_SERVICE_ERROR","Сеанс занят."))?.clear();
            Ok(result)
        }
        "GetHardwareStatus" => with_runtime(&state, |runtime| {
            Ok(
                json!({ "camera": runtime.face.camera_status(), "card": "KEYBOARD_WEDGE_OR_SERVICE_HID", "scanner": "KEYBOARD_WEDGE" }),
            )
        }),
        "CollectDiagnostics" => with_runtime(&state, |runtime| {
            Ok(
                json!({ "database": runtime.database.lock().map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "База занята."))?.health_check()?, "mode": runtime.config.mode, "version": env!("CARGO_PKG_VERSION") }),
            )
        }),
        "EnrollDevice" => {
            let cloud_url = request
                .payload
                .get("cloudUrl")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| AppError::new("INVALID_INPUT", "Укажите Cloud API URL."))?;
            let enrollment_code = request
                .payload
                .get("enrollmentCode")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| AppError::new("INVALID_INPUT", "Укажите код подключения."))?;
            let device_name = request
                .payload
                .get("deviceName")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| AppError::new("INVALID_INPUT", "Укажите название терминала."))?;
            let credentials = CredentialStore::for_service_workspace(&state.data_root, false);
            if state.data_root.join("production").join("library.db").exists() || credentials.cloud_device_credential()?.is_some() {
                return Err(AppError::new("ALREADY_ENROLLED", "Production уже содержит данные или credential. Повторное подключение не выполнялось."));
            }
            let (terminal_id, school_id) = ProductionCloudHttpApi::enroll_with_code(
                cloud_url,
                enrollment_code,
                device_name,
                &credentials,
            )?;
            replace_workspace(
                &state,
                WorkspaceConfig {
                    mode: WorkspaceMode::Production,
                    cloud_url: Some(cloud_url.into()),
                    terminal_id: Some(terminal_id),
                    school_id: Some(school_id),
                    device_name: device_name.into(),
                },
            )?;
            Ok(json!({ "enrolled": true }))
        }
        "RevokeDeviceCredential" => Err(AppError::new(
            "ADMIN_COMMAND_REQUIRES_CONFIGURATOR_UPDATE",
            "Эта версия Configurator ещё не поддерживает данную операцию безопасно.",
        )),
        _ => Err(AppError::new(
            "ADMIN_COMMAND_REJECTED",
            "Недопустимая административная команда.",
        )),
    }
}

fn with_runtime<T>(
    state: &ServiceState,
    operation: impl FnOnce(&Runtime) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let slot = state
        .runtime
        .lock()
        .map_err(|_| AppError::new("LOCAL_SERVICE_ERROR", "Служба занята."))?;
    operation(
        slot.as_ref()
            .ok_or_else(|| AppError::new("SETUP_REQUIRED", "Настройте рабочую область."))?,
    )
}
fn with_uat<T>(
    state: &ServiceState,
    operation: impl FnOnce(&Runtime) -> Result<T, AppError>,
) -> Result<T, AppError> {
    with_runtime(state, |runtime| {
        if runtime.config.mode != WorkspaceMode::Uat {
            return Err(AppError::new(
                "TEST_MODE_REQUIRED",
                "Операция доступна только в UAT.",
            ));
        }
        operation(runtime)
    })
}
fn save_workspace(path: &std::path::Path, config: &WorkspaceConfig) -> Result<(), AppError> {
    use std::io::Write;
    let temporary = path.with_extension("new.json");
    let mut file = std::fs::File::create(&temporary)?;
    file.write_all(&serde_json::to_vec(config)?)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(temporary, path)?;
    Ok(())
}
fn replace_workspace(state: &ServiceState, config: WorkspaceConfig) -> Result<(), AppError> {
    if config.mode == WorkspaceMode::Production
        && (config.cloud_url.as_deref().unwrap_or_default().is_empty()
            || config.terminal_id.as_deref().unwrap_or_default().is_empty()
            || config.school_id.as_deref().unwrap_or_default().is_empty())
    {
        return Err(AppError::new(
            "CLOUD_CONFIG_INVALID",
            "Production требует подтверждённые параметры Cloud.",
        ));
    }
    let mut slot = state
        .runtime
        .lock()
        .map_err(|_| AppError::new("LOCAL_SERVICE_ERROR", "Служба занята."))?;
    if let Some(previous) = slot.as_ref() {
        previous
            .database
            .lock()
            .map_err(|_| AppError::new("LOCAL_DATABASE_ERROR", "База занята."))?
            .checkpoint_for_maintenance()?;
    }
    let temporary = state.data_root.join("workspace.new.json");
    std::fs::write(
        &temporary,
        serde_json::to_vec(&config).map_err(|_| {
            AppError::new(
                "LOCAL_STORAGE_ERROR",
                "Не удалось сохранить рабочую область.",
            )
        })?,
    )
    .map_err(|_| {
        AppError::new(
            "LOCAL_STORAGE_ERROR",
            "Не удалось сохранить рабочую область.",
        )
    })?;
    let saved = state
        .data_root
        .join(if config.mode == WorkspaceMode::Uat {
            "uat"
        } else {
            "production"
        })
        .join("workspace.json");
    let replacement = Runtime::open_service(&state.data_root, config.clone())?;
    if let Some(previous) = slot.as_ref() {
        let old = state
            .data_root
            .join(if previous.config.mode == WorkspaceMode::Uat {
                "uat"
            } else {
                "production"
            })
            .join("workspace.json");
        save_workspace(&old, &previous.config)?;
    }
    save_workspace(&saved, &config)?;
    std::fs::rename(&temporary, state.data_root.join("workspace.json")).map_err(|_| {
        AppError::new(
            "LOCAL_STORAGE_ERROR",
            "Не удалось применить рабочую область.",
        )
    })?;
    if let Some(previous) = slot.replace(Arc::new(replacement)) {
        if let Ok(database) = previous.database.lock() {
            let _ = database.checkpoint_for_maintenance();
        }
    }
    *state
        .startup_error
        .lock()
        .map_err(|_| AppError::new("LOCAL_SERVICE_ERROR", "Служба занята."))? = None;
    state
        .sessions
        .lock()
        .map_err(|_| AppError::new("LOCAL_SERVICE_ERROR", "Служба занята."))?
        .clear();
    Ok(())
}

#[cfg(windows)]
fn caller_is_windows_administrator(pipe: &NamedPipeServer) -> bool {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Security::{
            CheckTokenMembership, CreateWellKnownSid, RevertToSelf, WinBuiltinAdministratorsSid,
            PSID,
        },
        System::Pipes::ImpersonateNamedPipeClient,
    };
    // Called AFTER reading the request prefix. No await is allowed while impersonating.
    unsafe {
        if ImpersonateNamedPipeClient(HANDLE(pipe.as_raw_handle())).is_err() {
            return false;
        }
        let mut buffer = [0u32; 17];
        let mut size = std::mem::size_of_val(&buffer) as u32;
        let sid = PSID(buffer.as_mut_ptr().cast());
        let mut member = Default::default();
        let accepted = CreateWellKnownSid(WinBuiltinAdministratorsSid, None, Some(sid), &mut size)
            .is_ok()
            && CheckTokenMembership(None, sid, &mut member).is_ok()
            && member.as_bool();
        // Never return a Tokio thread to the pool under the caller's identity.
        if RevertToSelf().is_err() {
            std::process::abort();
        }
        accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn command(state: &ServiceState, name: &str, payload: Value) -> Result<Value, AppError> {
        dispatch(
            state.clone(),
            AdminRequest {
                protocol_version: 1,
                request_id: uuid::Uuid::new_v4().to_string(),
                command: name.into(),
                payload,
            },
        )
    }
    #[tokio::test]
    async fn admin_domain_card_backup_restore_and_http_reopen() {
        let (_directory, state) = crate::tests::fixture();
        let config = WorkspaceConfig {
            mode: WorkspaceMode::Uat,
            cloud_url: None,
            terminal_id: None,
            school_id: None,
            device_name: "Admin integration UAT".into(),
        };
        command(
            &state,
            "SetActiveWorkspace",
            serde_json::to_value(&config).unwrap(),
        )
        .unwrap();
        let created = command(&state,"CreateUatReader",json!({"externalId":"ADMIN-PIPE-TEST","fullName":"Тестовый читатель IPC","personType":"STUDENT","className":"7 Т","positionName":null,"status":"ACTIVE"})).unwrap();
        let id = created["reader"]["id"].as_str().unwrap();
        command(
            &state,
            "BindUatCard",
            json!({"readerId":id,"card":"00009871\r\n"}),
        )
        .unwrap();
        let backup = command(&state, "CreateBackup", json!({})).unwrap();
        assert!(command(
            &state,
            "RestoreBackup",
            json!({"fileName":backup["fileName"]})
        )
        .is_err());
        command(
            &state,
            "RestoreBackup",
            json!({"fileName":backup["fileName"],"confirmation":"RESTORE UAT"}),
        )
        .unwrap();
        assert!(state.sessions.lock().unwrap().is_empty());
        state.runtime.lock().unwrap().take();
        let reopened = Runtime::open_service(&state.data_root, config).unwrap();
        *state.runtime.lock().unwrap() = Some(std::sync::Arc::new(reopened));
        let (cookie, csrf) = crate::tests::auth(&state).await;
        let (status, result) = crate::tests::call(
            &state,
            "/api/local/v1/identify/card",
            json!({"code":"00009871"}),
            &cookie,
            &csrf,
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(result["reader"]["id"], id);
        assert!(command(&state, "ExecuteSql", json!({})).is_err());
    }
}
