-- Additive, in-place migration for Cloud projection reconciliation.
-- It never removes operational data, pending outbox records or sync conflicts.
CREATE TABLE IF NOT EXISTS schema_migrations (
  id TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cloud_entity_versions (
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  version INTEGER NOT NULL,
  deleted_at TEXT,
  updated_at TEXT NOT NULL,
  PRIMARY KEY(entity_type, entity_id)
);

INSERT OR IGNORE INTO schema_migrations(id, applied_at)
VALUES ('0002_cloud_projection', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));