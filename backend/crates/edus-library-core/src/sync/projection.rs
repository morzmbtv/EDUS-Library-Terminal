use crate::{db::LocalDatabase, domain::AppError};
use serde_json::Value;

/// Applies Cloud snapshots and delta pages only through LocalDatabase.
/// HTTP transport never receives access to SQL connections.
pub struct CloudProjectionApplier;

impl CloudProjectionApplier {
    pub fn apply_bootstrap(
        database: &mut LocalDatabase,
        snapshot: &Value,
        terminal_id: &str,
        protocol: &str,
    ) -> Result<(), AppError> {
        database.apply_cloud_bootstrap(snapshot, terminal_id, protocol)
    }

    pub fn apply_delta_batch(
        database: &mut LocalDatabase,
        school_id: &str,
        changes: &[Value],
        next_cursor: &str,
    ) -> Result<(), AppError> {
        database.apply_cloud_delta_batch(school_id, changes, next_cursor)
    }

    pub fn reconcile_ack(database: &mut LocalDatabase, operation_id: &str) -> Result<(), AppError> {
        database.reconcile_cloud_ack(operation_id)
    }

    pub fn reconcile_conflict(
        database: &mut LocalDatabase,
        operation_id: &str,
        reason: &str,
        local_payload: &str,
        cloud_details: &str,
    ) -> Result<(), AppError> {
        database.reconcile_cloud_conflict(operation_id, reason, local_payload, cloud_details)
    }
}
