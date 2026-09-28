-- Additive only: existing admin_audit_events and every previous record stay untouched.
CREATE TABLE IF NOT EXISTS recovery_action_audit (
 id TEXT PRIMARY KEY,
 action TEXT NOT NULL CHECK(action IN ('ENCRYPTED_BACKUP_CREATED','WORKSPACE_SWITCH_REQUESTED','KIOSK_SETUP_REQUESTED')),
 reason_code TEXT NOT NULL,
 occurred_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_recovery_action_audit_time ON recovery_action_audit(occurred_at DESC);
INSERT OR IGNORE INTO schema_migrations(id,applied_at) VALUES('0006_recovery_action_audit',strftime('%Y-%m-%dT%H:%M:%fZ','now'));
