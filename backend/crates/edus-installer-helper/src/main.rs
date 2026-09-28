// Narrow installer helper: only this product's preflight, stop and readiness.
// No caller-selected service names, shell execution, registry writes or DB access.
use serde::Serialize;
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::ERROR_SUCCESS,
        NetworkManagement::IpHelper::*,
        System::{Environment::ExpandEnvironmentStringsW, Registry::*},
    },
};
use windows_service::{
    service::{Service, ServiceAccess, ServiceState},
    service_manager::{ServiceManager, ServiceManagerAccess},
};
const NAME: &str = "EDUSLibraryService";
const EXE: &str = "EDUSLibraryService.exe";

#[derive(Debug, Serialize)]
struct Failure {
    stage: Box<str>,
    operation: Box<str>,
    category: &'static str,
    application_code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    win32_code: Option<i32>,
    message: Box<str>,
    recovery_hint: &'static str,
}
type Result<T> = std::result::Result<T, Failure>;
fn fail(code: &'static str, message: impl Into<String>) -> Failure {
    Failure {
        stage: "preflight".into(),
        operation: "validate".into(),
        category: "EDUS",
        application_code: code,
        win32_code: None,
        message: message.into().into_boxed_str(),
        recovery_hint: "Preserve data. Review this run's diagnostic log before retrying.",
    }
}
fn win(operation: &str, e: windows_service::Error) -> Failure {
    let code = match &e {
        windows_service::Error::Winapi(e) => e.raw_os_error(),
        _ => None,
    };
    Failure {
        operation: operation.into(),
        category: "WIN32",
        win32_code: code,
        ..fail("SCM_API_FAILED", e.to_string())
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn expand(s: &str) -> Result<String> {
    let input = wide(s);
    let size = unsafe { ExpandEnvironmentStringsW(PCWSTR(input.as_ptr()), None) };
    if size == 0 || size > 32768 {
        return Err(fail("INVALID_SERVICE_PATH", "Environment expansion failed"));
    }
    let mut output = vec![0; size as usize];
    let written = unsafe { ExpandEnvironmentStringsW(PCWSTR(input.as_ptr()), Some(&mut output)) };
    if written == 0 || written > size {
        return Err(fail(
            "INVALID_SERVICE_PATH",
            "Environment changed during expansion",
        ));
    }
    String::from_utf16(&output[..written as usize - 1])
        .map_err(|_| fail("INVALID_SERVICE_PATH", "Invalid Unicode"))
}
/// Refuse ambiguous unquoted spaces. No shell parsing, quote removal or substring match.
fn command_path(command: &str) -> Result<PathBuf> {
    let command = command.trim();
    let (path, args) = if let Some(rest) = command.strip_prefix('"') {
        let end = rest
            .find('"')
            .ok_or_else(|| fail("INVALID_SERVICE_PATH", "Unclosed executable quote"))?;
        (&rest[..end], &rest[end + 1..])
    } else {
        let end = command.find(char::is_whitespace).unwrap_or(command.len());
        (&command[..end], &command[end..])
    };
    // The shipped SCM service has no arguments; explicit --service is supported.
    if !matches!(args.trim(), "" | "--service") {
        return Err(fail(
            "UNSUPPORTED_SERVICE_ARGUMENTS",
            "Ambiguous executable or unsupported service arguments",
        ));
    }
    if path.contains('"') || path.contains('%') {
        return Err(fail("INVALID_SERVICE_PATH", "Unresolved path"));
    }
    let p = PathBuf::from(path);
    if !p.is_absolute()
        || path.starts_with(r"\\")
        || path.contains('/')
        || p.components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(fail(
            "INVALID_SERVICE_PATH",
            "An absolute local drive path is required",
        ));
    }
    Ok(p)
}
fn no_reparse(path: &Path) -> Result<()> {
    use std::os::windows::fs::MetadataExt;
    for part in path.ancestors() {
        match fs::symlink_metadata(part) {
            Ok(m) if m.file_attributes() & 0x400 != 0 => {
                return Err(fail("REPARSE_POINT_REJECTED", part.display().to_string()))
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(fail("PATH_INSPECTION_FAILED", e.to_string())),
        }
    }
    Ok(())
}
fn equal_path(a: &Path, b: &Path) -> bool {
    a.as_os_str()
        .to_string_lossy()
        .eq_ignore_ascii_case(&b.as_os_str().to_string_lossy())
}
fn registry(key: &str, value: &str, view: REG_ROUTINE_FLAGS) -> Option<String> {
    let key = wide(key);
    let value = wide(value);
    let mut buf = vec![0u16; 32768];
    let mut size = (buf.len() * 2) as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(key.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_SZ | view,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if result != ERROR_SUCCESS || size < 2 {
        return None;
    }
    String::from_utf16(&buf[..size as usize / 2 - 1]).ok()
}
fn registry_dword(key: &str, value: &str, view: REG_ROUTINE_FLAGS) -> Option<u32> {
    let key = wide(key);
    let value = wide(value);
    let mut result = 0u32;
    let mut size = 4;
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(key.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_DWORD | view,
            None,
            Some((&mut result as *mut u32).cast()),
            Some(&mut size),
        )
    };
    (status == ERROR_SUCCESS && size == 4).then_some(result)
}
fn portable_record(product: &str, layout: u32, root: &str) -> Option<PathBuf> {
    if product != "EDUS-Library-Portable" || layout != 1 {
        return None;
    }
    let executable = command_path(&format!(
        "\"{}\"",
        Path::new(root).join("backend").join(EXE).display()
    ))
    .ok()?;
    no_reparse(&executable).ok()?;
    executable.parent().map(Path::to_path_buf)
}
fn installation_records() -> Vec<PathBuf> {
    let mut records: Vec<_> = [RRF_SUBKEY_WOW6432KEY, RRF_SUBKEY_WOW6464KEY]
        .iter()
        .flat_map(|view| {
            ["EDUS Library Edge", "EDUS Library Backend"]
                .into_iter()
                .filter_map(move |product| {
                    let dir = registry(r"Software\EDUS\Library", "InstallDir", *view)?;
                    let uninstall =
                        format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{product}");
                    let display = registry(&uninstall, "DisplayName", *view)?;
                    let command = registry(&uninstall, "UninstallString", *view)?;
                    let uninstaller = command_path(&expand(&command).ok()?).ok()?;
                    let directory = PathBuf::from(dir);
                    if !matches!(
                        display.as_str(),
                        "EDUS Library Terminal Edge" | "EDUS Library Backend"
                    ) || !equal_path(&uninstaller, &directory.join("Uninstall.exe"))
                    {
                        return None;
                    }
                    Some(directory)
                })
        })
        .collect();
    // Product marker and root are protected by HKLM administrator ACL, not inferred
    // from an executable name, version, hash or a caller-controlled config file.
    let key = r"Software\EDUS\Library\Portable";
    let portable = (|| {
        portable_record(
            &registry(key, "Product", RRF_SUBKEY_WOW6464KEY)?,
            registry_dword(key, "LayoutVersion", RRF_SUBKEY_WOW6464KEY)?,
            &registry(key, "InstallRoot", RRF_SUBKEY_WOW6464KEY)?,
        )
    })();
    if let Some(directory) = portable {
        records.push(directory);
    }
    records
}
fn validate_ownership(actual: &Path, records: &[PathBuf]) -> Result<()> {
    if !records.iter().any(|dir| equal_path(actual, &dir.join(EXE))) {
        return Err(fail(
            "SERVICE_OWNERSHIP_MISMATCH",
            format!(
                "SCM executable={} ; confirmed installation paths={records:?}",
                actual.display()
            ),
        ));
    }
    no_reparse(actual)
}
fn open_service() -> Result<Option<Service>> {
    open_service_with_access(
        ServiceAccess::QUERY_CONFIG | ServiceAccess::QUERY_STATUS | ServiceAccess::STOP,
    )
}
fn open_service_with_access(access: ServiceAccess) -> Result<Option<Service>> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| win("OpenSCManagerW", e))?;
    match manager.open_service(NAME, access) {
        Ok(service) => Ok(Some(service)),
        Err(windows_service::Error::Winapi(e)) if e.raw_os_error() == Some(1060) => Ok(None),
        Err(e) => Err(win("OpenServiceW", e)),
    }
}
fn service_configuration(service: &Service) -> Result<(String, String)> {
    use windows::Win32::{
        Foundation::ERROR_INSUFFICIENT_BUFFER,
        System::Services::{QueryServiceConfigW, QUERY_SERVICE_CONFIGW, SC_HANDLE},
    };
    let handle = SC_HANDLE(service.raw_handle());
    let mut needed = 0;
    // windows-rs captures last error before returning; no intervening logging/API.
    let sizing = unsafe { QueryServiceConfigW(handle, None, 0, &mut needed) };
    if !matches!(sizing, Err(ref e) if e.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult())
        || needed < std::mem::size_of::<QUERY_SERVICE_CONFIGW>() as u32
        || needed > 8192
    {
        return Err(fail(
            "SERVICE_CONFIG_SIZE_INVALID",
            "Unexpected QueryServiceConfigW sizing result",
        ));
    }
    let mut buffer = vec![0usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
    let config = buffer.as_mut_ptr().cast::<QUERY_SERVICE_CONFIGW>();
    unsafe { QueryServiceConfigW(handle, Some(config), needed, &mut needed) }.map_err(|e| {
        Failure {
            category: "WIN32",
            win32_code: Some(e.code().0 & 0xffff),
            operation: "QueryServiceConfigW".into(),
            ..fail("SCM_API_FAILED", "Cannot read service configuration")
        }
    })?;
    let config = unsafe { &*config };
    let path = unsafe { config.lpBinaryPathName.to_string() }
        .map_err(|_| fail("SERVICE_CONFIG_INVALID", "Invalid executable Unicode"))?;
    let account = unsafe { config.lpServiceStartName.to_string() }
        .map_err(|_| fail("SERVICE_CONFIG_INVALID", "Invalid account Unicode"))?;
    Ok((path, account))
}
fn preflight(expected: &Path) -> Result<Option<Service>> {
    let expected = command_path(&format!("\"{}\"", expected.join(EXE).display()))?;
    no_reparse(&expected)?;
    if let Some(program_data) = std::env::var_os("ProgramData") {
        no_reparse(&PathBuf::from(program_data).join("EDUS Library"))?;
    }
    let service = open_service()?;
    if service.is_none() && expected.exists() {
        validate_ownership(&expected, &installation_records())?;
    }
    if let Some(ref service) = service {
        let (raw, account) = service_configuration(service)?;
        let actual = command_path(&expand(&raw)?)?;
        validate_ownership(&actual, &installation_records()).map_err(|mut e| {
            e.message = format!("{} ; expected new path={}", e.message, expected.display()).into();
            e
        })?;
        if !account.eq_ignore_ascii_case(r"NT AUTHORITY\LocalService") {
            return Err(fail("SERVICE_IDENTITY_MISMATCH","Existing logon account is not LocalService; automatic identity migration is forbidden"));
        }
        println!("Verified EDUS executable: {}; destination: {}; logon: LocalService (not SCM security owner)",actual.display(),expected.display());
    }
    Ok(service)
}
fn stop(service: &Service) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut stop_sent = false;
    loop {
        let status = service
            .query_status()
            .map_err(|e| win("QueryServiceStatusEx", e))?;
        if status.current_state == ServiceState::Stopped {
            return Ok(());
        }
        if !matches!(
            status.current_state,
            ServiceState::StartPending | ServiceState::StopPending
        ) && !stop_sent
        {
            match service.stop() {
                Ok(_) => stop_sent = true,
                Err(windows_service::Error::Winapi(e)) if e.raw_os_error() == Some(1062) => (),
                Err(e) => return Err(win("ControlService(STOP)", e)),
            }
        }
        if Instant::now() >= deadline {
            return Err(fail(
                "SERVICE_STOP_TIMEOUT",
                format!(
                    "State={:?}; checkpoint={}",
                    status.current_state, status.checkpoint
                ),
            ));
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}
#[derive(Serialize)]
struct Listener {
    address: String,
    port: u16,
    pid: u32,
}
fn tcp_table(family: u32) -> Result<(Vec<u64>, usize)> {
    let mut size = 0;
    let sizing = unsafe {
        GetExtendedTcpTable(
            None,
            &mut size,
            false,
            family,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        )
    };
    if sizing != 122 && sizing != 0 {
        return Err(Failure {
            win32_code: Some(sizing as i32),
            ..fail("TCP_INSPECTION_FAILED", "GetExtendedTcpTable sizing")
        });
    }
    if !(4..=16 * 1024 * 1024).contains(&size) {
        return Err(fail("TCP_INSPECTION_FAILED", "Invalid TCP table size"));
    }
    let mut buf = vec![0u64; (size as usize).div_ceil(8)];
    let result = unsafe {
        GetExtendedTcpTable(
            Some(buf.as_mut_ptr().cast()),
            &mut size,
            false,
            family,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        )
    };
    if result != 0 {
        return Err(Failure {
            win32_code: Some(result as i32),
            ..fail("TCP_INSPECTION_FAILED", "GetExtendedTcpTable")
        });
    }
    Ok((buf, size as usize))
}
fn listeners() -> Result<Vec<Listener>> {
    let (buf, size) = tcp_table(2)?;
    let count = unsafe { buf.as_ptr().cast::<u32>().read() } as usize;
    let offset = std::mem::offset_of!(MIB_TCPTABLE_OWNER_PID, table);
    if count > size / std::mem::size_of::<MIB_TCPROW_OWNER_PID>()
        || offset + count * std::mem::size_of::<MIB_TCPROW_OWNER_PID>() > size
    {
        return Err(fail("TCP_INSPECTION_FAILED", "Invalid TCP table"));
    }
    let rows = unsafe {
        std::slice::from_raw_parts(
            buf.as_ptr()
                .cast::<u8>()
                .add(offset)
                .cast::<MIB_TCPROW_OWNER_PID>(),
            count,
        )
    };
    let mut result: Vec<_> = rows
        .iter()
        .filter(|r| u16::from_be(r.dwLocalPort as u16) == 43180)
        .map(|r| Listener {
            address: std::net::Ipv4Addr::from(r.dwLocalAddr.to_ne_bytes()).to_string(),
            port: 43180,
            pid: r.dwOwningPid,
        })
        .collect();
    let (buf, size) = tcp_table(23)?;
    let count = unsafe { buf.as_ptr().cast::<u32>().read() } as usize;
    let offset = std::mem::offset_of!(MIB_TCP6TABLE_OWNER_PID, table);
    if count > size / std::mem::size_of::<MIB_TCP6ROW_OWNER_PID>()
        || offset + count * std::mem::size_of::<MIB_TCP6ROW_OWNER_PID>() > size
    {
        return Err(fail("TCP_INSPECTION_FAILED", "Invalid IPv6 TCP table"));
    }
    let rows = unsafe {
        std::slice::from_raw_parts(
            buf.as_ptr()
                .cast::<u8>()
                .add(offset)
                .cast::<MIB_TCP6ROW_OWNER_PID>(),
            count,
        )
    };
    result.extend(
        rows.iter()
            .filter(|r| u16::from_be(r.dwLocalPort as u16) == 43180)
            .map(|r| Listener {
                address: std::net::Ipv6Addr::from(r.ucLocalAddr).to_string(),
                port: 43180,
                pid: r.dwOwningPid,
            }),
    );
    Ok(result)
}
fn listener_pids() -> Result<Vec<u32>> {
    Ok(listeners()?.into_iter().map(|v| v.pid).collect())
}
fn inspect() -> Result<serde_json::Value> {
    let service =
        open_service_with_access(ServiceAccess::QUERY_CONFIG | ServiceAccess::QUERY_STATUS)?;
    let state = if let Some(service) = service {
        let (binary_path, account) = service_configuration(&service)?;
        let configuration = service
            .query_config()
            .map_err(|e| win("QueryServiceConfigW", e))?;
        let status = service
            .query_status()
            .map_err(|e| win("QueryServiceStatusEx", e))?;
        let sid = service
            .get_config_service_sid_info()
            .map_err(|e| win("QueryServiceConfig2W(SID)", e))?;
        let recovery = service
            .get_failure_actions()
            .map_err(|e| win("QueryServiceConfig2W(FAILURE_ACTIONS)", e))?;
        let executable = expand(&binary_path).and_then(|p| command_path(&p)).ok();
        let actions: Vec<_> = recovery.actions.unwrap_or_default().iter().map(|a| serde_json::json!({"type": a.action_type.to_raw(), "delay_ms": a.delay.as_millis()})).collect();
        serde_json::json!({"exists":true,"binary_path":binary_path,"executable_path":executable,"account":account,"startup_type":configuration.start_type.to_raw(),"state":status.current_state as u32,"pid":status.process_id,"sid_type":sid as u32,"recovery":{"reset_seconds":recovery.reset_period.to_raw(),"actions":actions}})
    } else {
        serde_json::json!({"exists":false,"binary_path":null,"executable_path":null,"account":null,"startup_type":null,"state":null,"pid":null,"sid_type":null,"recovery":null})
    };
    Ok(serde_json::json!({"service":state,"listeners":listeners()?}))
}
fn health(service: &Service) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let status = service
            .query_status()
            .map_err(|e| win("QueryServiceStatusEx", e))?;
        if status.current_state == ServiceState::Stopped {
            return Err(fail(
                "SERVICE_EXITED_DURING_START",
                format!("{:?}", status.exit_code),
            ));
        }
        let pids = listener_pids()?;
        if pids.iter().any(|pid| Some(*pid) != status.process_id) {
            return Err(fail(
                "PORT_OWNERSHIP_MISMATCH",
                "Port 43180 is owned by a different process",
            ));
        }
        if status.current_state == ServiceState::Running && !pids.is_empty() {
            if let Ok(mut socket) = TcpStream::connect_timeout(
                &"127.0.0.1:43180".parse::<SocketAddr>().unwrap(),
                Duration::from_secs(1),
            ) {
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .map_err(|e| fail("HEALTH_FAILED", e.to_string()))?;
                socket.write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1:43180\r\nConnection: close\r\n\r\n").map_err(|e|fail("HEALTH_FAILED",e.to_string()))?;
                let mut response = String::new();
                socket
                    .take(16385)
                    .read_to_string(&mut response)
                    .map_err(|e| fail("HEALTH_FAILED", e.to_string()))?;
                if response.len() > 16384 {
                    return Err(fail("HEALTH_INVALID", "Response too large"));
                }
                if let Some((head, body)) = response.split_once("\r\n\r\n") {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
                        if head.starts_with("HTTP/1.1 200")
                            && v["application"] == NAME
                            && v["pid"].as_u64() == status.process_id.map(u64::from)
                            && matches!(
                                v["state"].as_str(),
                                Some("READY" | "SETUP_REQUIRED" | "FRONTEND_MISSING")
                            )
                        {
                            println!("{}", v["state"]);
                            return Ok(());
                        }
                        if v["state"] == "ERROR" {
                            return Err(fail(
                                "SERVICE_RUNTIME_ERROR",
                                "Service reports ERROR; inspect service diagnostics",
                            ));
                        }
                    }
                }
            }
        }
        if Instant::now() >= deadline {
            return Err(fail(
                "SERVICE_HEALTH_TIMEOUT",
                "SCM and HTTP did not reach EDUS ready state",
            ));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
fn execute(action: &str, directory: &Path) -> Result<()> {
    let service = preflight(directory)?;
    match action {
        "preflight" => {
            let own_pid = service
                .as_ref()
                .map(|s| s.query_status())
                .transpose()
                .map_err(|e| win("QueryServiceStatusEx", e))?
                .and_then(|s| s.process_id);
            if listener_pids()?.iter().any(|pid| Some(*pid) != own_pid) {
                return Err(fail(
                    "PORT_IN_USE",
                    "Port 43180 belongs to another process; it was not stopped",
                ));
            }
            Ok(())
        }
        "stop" => service.as_ref().map(stop).unwrap_or(Ok(())),
        "health" => health(
            &service.ok_or_else(|| fail("SERVICE_NOT_INSTALLED", "EDUS service not registered"))?,
        ),
        _ => Err(fail(
            "INSTALLER_COMMAND_REJECTED",
            "Unknown installer operation",
        )),
    }
}
fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 4 {
        eprintln!("Usage: EDUSInstallerHelper preflight|stop|health|inspect INSTALLDIR RUN_LOG");
        std::process::exit(2);
    }
    let action = args[1].to_string_lossy();
    // Only read-only preflight/health and the fixed service stop are supported.
    let result = if action == "inspect" {
        inspect().map(Some)
    } else {
        execute(&action, Path::new(&args[2])).map(|()| None)
    };
    let line=match &result {
        Ok(Some(value))=>value.to_string(),
        Ok(None)=>serde_json::json!({"stage":action,"operation":action,"result":"PASS","time":format!("{:?}",std::time::SystemTime::now())}).to_string(),
        Err(error)=>{ let mut value=serde_json::to_value(error).unwrap(); value["stage"]=serde_json::json!(action); value.to_string() }
    };
    if let Ok(mut f) = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&args[3])
    {
        let _ = writeln!(f, "{line}");
    } else {
        eprintln!("INSTALL_LOG_UNAVAILABLE");
        std::process::exit(3);
    }
    println!("{line}");
    if result.is_err() {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_record_requires_product_layout_and_absolute_root() {
        assert_eq!(
            portable_record("EDUS-Library-Portable", 1, r"X:\EDUS Library"),
            Some(PathBuf::from(r"X:\EDUS Library\backend"))
        );
        assert!(portable_record("Other", 1, r"X:\EDUS Library").is_none());
        assert!(portable_record("EDUS-Library-Portable", 2, r"X:\EDUS Library").is_none());
        assert!(portable_record("EDUS-Library-Portable", 1, "relative").is_none());
        assert!(portable_record("EDUS-Library-Portable", 1, r"X:\EDUS\..\Other").is_none());
    }
    #[test]
    fn quoted_unicode_arguments() {
        assert_eq!(
            command_path(r#""C:\Program Files\Школа\EDUSLibraryService.exe" --service"#).unwrap(),
            PathBuf::from(r"C:\Program Files\Школа\EDUSLibraryService.exe")
        );
    }
    #[test]
    fn ambiguous_or_relative_rejected() {
        for p in [
            r"C:\Program Files\EDUSLibraryService.exe",
            r".\EDUSLibraryService.exe",
            r#""C:\EDUS\EDUSLibraryService.exe" --console"#,
            r#""C:\EDUS\EDUSLibraryService.exe" --arbitrary"#,
            r"%ROOT%\EDUSLibraryService.exe",
        ] {
            assert!(command_path(p).is_err(), "{p}");
        }
    }
    #[test]
    fn equality_not_substring() {
        assert!(!equal_path(
            Path::new(r"C:\bad\C:\EDUS\EDUSLibraryService.exe"),
            Path::new(r"C:\EDUS\EDUSLibraryService.exe")
        ));
        assert!(equal_path(
            Path::new(r"C:\EDUS\x.exe"),
            Path::new(r"c:\edus\X.EXE")
        ));
    }
    #[test]
    fn records_required_even_when_filename_matches() {
        assert_eq!(
            validate_ownership(Path::new(r"C:\foreign\EDUSLibraryService.exe"), &[])
                .unwrap_err()
                .application_code,
            "SERVICE_OWNERSHIP_MISMATCH"
        );
    }
    #[test]
    fn known_missing_binary_repair_and_changed_directory() {
        assert!(validate_ownership(
            Path::new(r"X:\Old EDUS\EDUSLibraryService.exe"),
            &[PathBuf::from(r"X:\Old EDUS")]
        )
        .is_ok());
    }
    #[test]
    fn application_failure_has_no_fake_win32_zero() {
        let v = serde_json::to_value(fail("SERVICE_OWNERSHIP_MISMATCH", "x")).unwrap();
        assert!(v.get("win32_code").is_none());
    }
}
