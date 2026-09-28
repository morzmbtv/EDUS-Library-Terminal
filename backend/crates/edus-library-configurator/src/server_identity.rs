//! Authenticate the connected pipe server before writing any request bytes.
//! The SCM, not pipe naming or a PID supplied in JSON, identifies the service.
use std::{mem::size_of, os::windows::io::AsRawHandle};
use tokio::net::windows::named_pipe::NamedPipeClient;
use windows::{
    core::w,
    Win32::{
        Foundation::HANDLE,
        System::{
            Pipes::GetNamedPipeServerProcessId,
            Services::{
                CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatusEx, SC_HANDLE,
                SC_MANAGER_CONNECT, SC_STATUS_PROCESS_INFO, SERVICE_QUERY_STATUS, SERVICE_RUNNING,
                SERVICE_STATUS_CURRENT_STATE, SERVICE_STATUS_PROCESS,
            },
        },
    },
};

struct ServiceHandle(SC_HANDLE);
impl Drop for ServiceHandle {
    fn drop(&mut self) {
        // Every successful SCM open has exactly one owner, including early errors.
        unsafe {
            let _ = CloseServiceHandle(self.0);
        }
    }
}

fn matches_running_service(
    pipe_pid: u32,
    service_pid: u32,
    state: SERVICE_STATUS_CURRENT_STATE,
) -> bool {
    pipe_pid != 0 && service_pid != 0 && pipe_pid == service_pid && state == SERVICE_RUNNING
}

fn query_status(service: &ServiceHandle) -> Result<SERVICE_STATUS_PROCESS, String> {
    let mut status = SERVICE_STATUS_PROCESS::default();
    let mut required = 0;
    // The API writes the native typed structure. No hardcoded architecture offsets.
    let bytes = unsafe {
        std::slice::from_raw_parts_mut(
            (&mut status as *mut SERVICE_STATUS_PROCESS).cast::<u8>(),
            size_of::<SERVICE_STATUS_PROCESS>(),
        )
    };
    unsafe {
        QueryServiceStatusEx(
            service.0,
            SC_STATUS_PROCESS_INFO,
            Some(bytes),
            &mut required,
        )
    }
    .map_err(|_| "ADMIN_IPC_SERVER_UNVERIFIED: Не удалось проверить статус службы Windows.")?;
    Ok(status)
}

pub(crate) fn verify(pipe: &NamedPipeClient) -> Result<(), String> {
    let manager = ServiceHandle(
        unsafe { OpenSCManagerW(None, None, SC_MANAGER_CONNECT) }.map_err(|_| {
            "ADMIN_IPC_SERVER_UNVERIFIED: Windows Service Control Manager недоступен."
        })?,
    );
    let service = ServiceHandle(unsafe { OpenServiceW(manager.0, w!("EDUSLibraryService"), SERVICE_QUERY_STATUS) }
        .map_err(|_| "ADMIN_IPC_SERVER_UNVERIFIED: Зарегистрированная служба EDUSLibraryService недоступна. Консольный процесс не принимается.")?);
    let before = query_status(&service)?;
    let mut pipe_pid = 0;
    unsafe { GetNamedPipeServerProcessId(HANDLE(pipe.as_raw_handle()), &mut pipe_pid) }.map_err(
        |_| "ADMIN_IPC_SERVER_UNVERIFIED: Не удалось подтвердить процесс сервера named pipe.",
    )?;
    let after = query_status(&service)?;
    if !matches_running_service(pipe_pid, before.dwProcessId, before.dwCurrentState)
        || !matches_running_service(pipe_pid, after.dwProcessId, after.dwCurrentState)
    {
        return Err("ADMIN_IPC_SERVER_MISMATCH: Канал не принадлежит работающей службе EDUSLibraryService. Запрос не отправлен.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Services::{
        SERVICE_START_PENDING, SERVICE_STOPPED, SERVICE_STOP_PENDING,
    };

    #[test]
    fn matching_running_service_only() {
        assert!(matches_running_service(781, 781, SERVICE_RUNNING));
        assert!(!matches_running_service(781, 782, SERVICE_RUNNING));
    }
    #[test]
    fn missing_or_pending_server_fails_closed() {
        assert!(!matches_running_service(0, 0, SERVICE_RUNNING));
        assert!(!matches_running_service(0, 781, SERVICE_RUNNING));
        assert!(!matches_running_service(781, 0, SERVICE_RUNNING));
        for state in [SERVICE_START_PENDING, SERVICE_STOP_PENDING, SERVICE_STOPPED] {
            assert!(!matches_running_service(781, 781, state));
        }
    }

    #[test]
    fn real_pipe_owned_by_test_process_is_not_the_service() {
        use tokio::{
            io::AsyncReadExt,
            net::windows::named_pipe::{ClientOptions, ServerOptions},
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            // Unique ephemeral pipe: no service registration or system configuration.
            let name = format!(r"\\.\pipe\edus-configurator-test-{}", uuid::Uuid::new_v4());
            let mut server = ServerOptions::new()
                .first_pipe_instance(true)
                .create(&name)
                .unwrap();
            let client = ClientOptions::new().open(&name).unwrap();
            server.connect().await.unwrap();
            assert!(verify(&client).is_err());
            drop(client);
            let mut received = [0u8; 1];
            assert!(server.read_exact(&mut received).await.is_err());
        });
    }
}
