-- Additive local audit for protected terminal administration.
-- Deliberately excludes usernames, passwords, credential blobs and tokens.
CREATE TABLE IF NOT EXISTS admin_audit_events (
  id TEXT PRIMARY KEY,
  event_type TEXT NOT NULL CHECK (event_type IN (
    'ADMIN_ACCESS_ATTEMPT', 'ADMIN_ACCESS_GRANTED', 'ADMIN_ACCESS_DENIED',
    'SERVICE_MODE_ENTER', 'SERVICE_MODE_EXIT',
    'WINDOWS_MAINTENANCE_REQUESTED', 'WINDOWS_MAINTENANCE_STARTED',
    'KIOSK_RETURNED'
  )),
  reason_code TEXT NOT NULL,
  occurred_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_admin_audit_events_occurred_at
  ON admin_audit_events(occurred_at DESC);

INSERT OR IGNORE INTO schema_migrations(id, applied_at)
VALUES ('0003_admin_audit', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
