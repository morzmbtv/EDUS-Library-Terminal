-- Local-only metadata for the explicitly enabled terminal UAT namespace.
CREATE TABLE IF NOT EXISTS terminal_test_readers (
  person_id TEXT PRIMARY KEY REFERENCES persons(id) ON DELETE CASCADE,
  external_id TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_terminal_test_readers_external_id
  ON terminal_test_readers(external_id);

INSERT OR IGNORE INTO schema_migrations(id, applied_at)
VALUES ('0004_terminal_test_readers', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
