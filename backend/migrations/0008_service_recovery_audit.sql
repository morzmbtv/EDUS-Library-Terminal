-- Additive service recovery evidence; preserve historical admin/recovery rows.
CREATE TABLE IF NOT EXISTS service_recovery_events (
 id TEXT PRIMARY KEY,
 action TEXT NOT NULL CHECK(action IN ('UAT_BACKUP_RESTORED')),
 reason_code TEXT NOT NULL,
 occurred_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_service_recovery_time ON service_recovery_events(occurred_at DESC);
INSERT OR IGNORE INTO schema_migrations(id,applied_at) VALUES('0008_service_recovery_audit',strftime('%Y-%m-%dT%H:%M:%fZ','now'));
