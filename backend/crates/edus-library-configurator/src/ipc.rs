use edus_library_admin_ipc::{
    allowed, Request, Response, MAX_MESSAGE_BYTES, PIPE_NAME, PROTOCOL_VERSION,
};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::ClientOptions,
};
use uuid::Uuid;
use zeroize::Zeroize;

#[derive(Deserialize)]
struct RemoteError {
    code: String,
    message: String,
}

fn encode(command: &str, payload: Value, id: &str) -> Result<Vec<u8>, String> {
    if !allowed(command) {
        return Err("ADMIN_COMMAND_REJECTED".into());
    }
    let mut bytes = serde_json::to_vec(&Request {
        protocol_version: PROTOCOL_VERSION,
        request_id: id.into(),
        command: command.into(),
        payload,
    })
    .map_err(|_| "ADMIN_IPC_INVALID_REQUEST")?;
    if bytes.len() > MAX_MESSAGE_BYTES {
        bytes.zeroize();
        return Err("ADMIN_IPC_INVALID_SIZE".into());
    }
    Ok(bytes)
}
fn decode(bytes: &[u8], id: &str) -> Result<Value, String> {
    let response: Response<RemoteError> =
        serde_json::from_slice(bytes).map_err(|_| "ADMIN_IPC_INVALID_RESPONSE")?;
    if response.request_id != id {
        return Err("ADMIN_IPC_PROTOCOL".into());
    }
    if response.ok {
        response
            .result
            .ok_or_else(|| "ADMIN_IPC_INVALID_RESPONSE".into())
    } else {
        Err(response
            .error
            .map(|e| format!("{}: {}", e.code, e.message))
            .unwrap_or_else(|| "ADMIN_ACCESS_DENIED".into()))
    }
}
/// Backups may perform two separately bounded encrypted database copies.
/// Deadlines do not authorize retrying an operation with an unknown result.
pub fn deadline(command: &str) -> Duration {
    Duration::from_secs(match command {
        "CreateBackup" | "RestoreBackup" => 180,
        _ => 35,
    })
}

pub fn request(command: String, payload: Value) -> Result<Value, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "ADMIN_IPC_RUNTIME")?;
    runtime.block_on(async {
        tokio::time::timeout(deadline(&command), async {
            let id = Uuid::new_v4().to_string();
            let mut pipe = ClientOptions::new().open(PIPE_NAME).map_err(|_| {
                "Служба недоступна или доступ запрещён. Проверьте EDUSLibraryService."
            })?;
            crate::server_identity::verify(&pipe)?;
            let bytes = zeroize::Zeroizing::new(encode(&command, payload, &id)?);
            pipe.write_all(&(bytes.len() as u32).to_le_bytes())
                .await
                .map_err(|_| "ADMIN_IPC_WRITE_FAILED")?;
            pipe.write_all(&bytes)
                .await
                .map_err(|_| "ADMIN_IPC_WRITE_FAILED")?;
            pipe.flush().await.map_err(|_| "ADMIN_IPC_WRITE_FAILED")?;
            let mut size = [0; 4];
            pipe.read_exact(&mut size)
                .await
                .map_err(|_| "ADMIN_IPC_READ_FAILED")?;
            let length = u32::from_le_bytes(size) as usize;
            if length == 0 || length > MAX_MESSAGE_BYTES {
                return Err("ADMIN_IPC_INVALID_RESPONSE".into());
            }
            let mut bytes = zeroize::Zeroizing::new(vec![0; length]);
            pipe.read_exact(&mut bytes)
                .await
                .map_err(|_| "ADMIN_IPC_READ_FAILED")?;
            decode(&bytes, &id)
        })
        .await
        .map_err(|_| "ADMIN_IPC_TIMEOUT: Результат неизвестен. Проверьте данные перед повтором.")?
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn rejects_arbitrary_commands() {
        for cmd in [
            "ExecuteSql",
            "ReadAnyFile",
            "ExecutePowerShell",
            "LaunchProcess",
        ] {
            assert!(encode(cmd, json!({}), "id").is_err());
        }
    }
    #[test]
    fn exact_request_contract() {
        let v: Value = serde_json::from_slice(
            &encode(
                "BindUatCard",
                json!({"readerId":"reader", "card":"00009"}),
                "id",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            v,
            json!({"protocolVersion":1,"requestId":"id","command":"BindUatCard","payload":{"readerId":"reader","card":"00009"}})
        );
    }
    #[test]
    fn response_correlated_and_fail_closed() {
        assert!(decode(br#"{"requestId":"other","ok":true,"result":{}}"#, "id").is_err());
        assert!(decode(br#"{"requestId":"id","ok":true}"#, "id").is_err());
        assert!(decode(
            br#"{"requestId":"id","ok":false,"error":{"code":"DENIED","message":"Denied"}}"#,
            "id"
        )
        .is_err());
    }
    #[test]
    fn bounded_request() {
        assert!(encode(
            "ImportUatReaders",
            json!({"csv":"x".repeat(MAX_MESSAGE_BYTES)}),
            "id"
        )
        .is_err());
    }

    #[test]
    fn only_database_backup_commands_have_extended_deadline() {
        assert_eq!(deadline("CreateBackup"), Duration::from_secs(180));
        assert_eq!(deadline("RestoreBackup"), Duration::from_secs(180));
        for command in [
            "ListBackups",
            "CreateUatReader",
            "ResetUatData",
            "GetServiceStatus",
            "EnrollDevice",
            "RestoreBackupOther",
        ] {
            assert_eq!(deadline(command), Duration::from_secs(35));
        }
    }
}
