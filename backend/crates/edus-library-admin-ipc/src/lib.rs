//! Shared wire contract only. No OS, SQL, credentials or domain implementation.
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub const PIPE_NAME: &str = r"\\.\pipe\edus-library-admin-v1";
pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_MESSAGE_BYTES: usize = 256 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub protocol_version: u8,
    pub request_id: String,
    pub command: String,
    #[serde(default)]
    pub payload: Value,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response<E = Value> {
    pub request_id: String,
    pub ok: bool,
    pub result: Option<Value>,
    pub error: Option<E>,
}
pub fn allowed(command: &str) -> bool {
    matches!(
        command,
        "GetServiceStatus"
            | "GetWorkspaceStatus"
            | "SetActiveWorkspace"
            | "SetCloudConfiguration"
            | "EnrollDevice"
            | "RevokeDeviceCredential"
            | "ListUatData"
            | "CreateUatReader"
            | "PreviewUatReaders"
            | "ImportUatReaders"
            | "CreateUatBook"
            | "BindUatCard"
            | "BindUatScannerCode"
            | "ResetUatData"
            | "CreateBackup"
            | "ListBackups"
            | "RestoreBackup"
            | "GetHardwareStatus"
            | "CollectDiagnostics"
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_generic_execution() {
        for name in [
            "ExecuteSql",
            "ExecuteCommand",
            "ReadAnyFile",
            "ModifyRegistryArbitrary",
        ] {
            assert!(!allowed(name));
        }
    }
    #[test]
    fn request_is_versioned() {
        let r:Request=serde_json::from_value(serde_json::json!({"protocolVersion":1,"requestId":"test","command":"GetServiceStatus","payload":{}})).unwrap();
        assert_eq!(r.protocol_version, PROTOCOL_VERSION);
        assert!(allowed(&r.command));
    }
}
