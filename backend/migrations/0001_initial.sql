PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schools (
  id TEXT PRIMARY KEY, cloud_id TEXT, name TEXT NOT NULL, version INTEGER NOT NULL DEFAULT 1, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS terminals (
  id TEXT PRIMARY KEY, school_id TEXT NOT NULL REFERENCES schools(id), cloud_id TEXT, name TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'ACTIVE', version INTEGER NOT NULL DEFAULT 1, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS classes (
  id TEXT PRIMARY KEY, school_id TEXT NOT NULL REFERENCES schools(id), name TEXT NOT NULL, version INTEGER NOT NULL DEFAULT 1, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS persons (
  id TEXT PRIMARY KEY, cloud_id TEXT, school_id TEXT NOT NULL REFERENCES schools(id), full_name TEXT NOT NULL,
  person_type TEXT NOT NULL CHECK(person_type IN ('STUDENT','TEACHER','STAFF')), class_id TEXT REFERENCES classes(id), class_name TEXT,
  position_name TEXT, status TEXT NOT NULL CHECK(status IN ('ACTIVE','INACTIVE')), version INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL, deleted_at TEXT
);
CREATE TABLE IF NOT EXISTS cards (
  id TEXT PRIMARY KEY, person_id TEXT NOT NULL REFERENCES persons(id), card_lookup_hash TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL CHECK(status IN ('ACTIVE','REVOKED')), version INTEGER NOT NULL DEFAULT 1, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS face_templates (
  id TEXT PRIMARY KEY, person_id TEXT NOT NULL REFERENCES persons(id), template_ciphertext BLOB NOT NULL,
  model_id TEXT NOT NULL, model_version TEXT NOT NULL, template_version INTEGER NOT NULL, consent_reference TEXT,
  consent_status TEXT NOT NULL CHECK(consent_status IN ('ACTIVE','REVOKED')), enrolled_at TEXT NOT NULL, updated_at TEXT NOT NULL, revoked_at TEXT
);
CREATE TABLE IF NOT EXISTS library_locations (
  id TEXT PRIMARY KEY, school_id TEXT NOT NULL REFERENCES schools(id), name TEXT NOT NULL, code TEXT NOT NULL UNIQUE, version INTEGER NOT NULL DEFAULT 1, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS book_titles (
  id TEXT PRIMARY KEY, cloud_id TEXT, school_id TEXT NOT NULL REFERENCES schools(id), isbn TEXT, title TEXT NOT NULL, authors TEXT NOT NULL,
  language TEXT NOT NULL, publisher TEXT, publication_year INTEGER, subject TEXT, grade TEXT, version INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL, deleted_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_book_titles_isbn ON book_titles(isbn) WHERE deleted_at IS NULL;
CREATE TABLE IF NOT EXISTS book_copies (
  id TEXT PRIMARY KEY, cloud_id TEXT, book_title_id TEXT NOT NULL REFERENCES book_titles(id), inventory_number TEXT,
  barcode TEXT, safeschool_code TEXT, status TEXT NOT NULL CHECK(status IN ('AVAILABLE','ON_LOAN','RESERVED','REPAIR','LOST','WRITTEN_OFF','VERIFYING')),
  location_id TEXT REFERENCES library_locations(id), version INTEGER NOT NULL DEFAULT 1, updated_at TEXT NOT NULL, deleted_at TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_copy_inventory ON book_copies(inventory_number) WHERE inventory_number IS NOT NULL AND deleted_at IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS uq_copy_barcode ON book_copies(barcode) WHERE barcode IS NOT NULL AND deleted_at IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS uq_copy_safeschool_code ON book_copies(safeschool_code) WHERE safeschool_code IS NOT NULL AND deleted_at IS NULL;
CREATE TABLE IF NOT EXISTS legacy_title_stock (
  book_title_id TEXT PRIMARY KEY REFERENCES book_titles(id), total_quantity INTEGER NOT NULL CHECK(total_quantity >= 0), available_quantity INTEGER NOT NULL CHECK(available_quantity >= 0 AND available_quantity <= total_quantity), version INTEGER NOT NULL DEFAULT 1, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS loans (
  id TEXT PRIMARY KEY, cloud_id TEXT, reader_id TEXT NOT NULL REFERENCES persons(id), book_title_id TEXT NOT NULL REFERENCES book_titles(id),
  book_copy_id TEXT REFERENCES book_copies(id), accounting_mode TEXT NOT NULL CHECK(accounting_mode IN ('COPY','LEGACY_TITLE')), quantity INTEGER NOT NULL,
  issued_at TEXT NOT NULL, due_at TEXT, returned_at TEXT, status TEXT NOT NULL CHECK(status IN ('ACTIVE','RETURNED','CANCELLED','CONFLICT')),
  created_terminal_id TEXT NOT NULL REFERENCES terminals(id), version INTEGER NOT NULL DEFAULT 1, updated_at TEXT NOT NULL,
  CHECK((accounting_mode='COPY' AND book_copy_id IS NOT NULL AND quantity=1) OR (accounting_mode='LEGACY_TITLE' AND book_copy_id IS NULL AND quantity>=1))
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_active_copy_loan ON loans(book_copy_id) WHERE status='ACTIVE' AND book_copy_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_active_reader_loans ON loans(reader_id, status);
CREATE TABLE IF NOT EXISTS reservations (
  id TEXT PRIMARY KEY, cloud_id TEXT, reader_id TEXT NOT NULL REFERENCES persons(id), book_title_id TEXT NOT NULL REFERENCES book_titles(id),
  created_at TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN ('WAITING','FULFILLED','CANCELLED')), version INTEGER NOT NULL DEFAULT 1, updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_waiting_reservation ON reservations(reader_id, book_title_id) WHERE status='WAITING';
CREATE TABLE IF NOT EXISTS library_operations (
  id TEXT PRIMARY KEY, operation_id TEXT NOT NULL UNIQUE, type TEXT NOT NULL CHECK(type IN ('ISSUE','ACCEPT','REGISTER','RESERVE')), entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL, reader_id TEXT REFERENCES persons(id), terminal_id TEXT NOT NULL REFERENCES terminals(id), occurred_at TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('COMMITTED','SYNCED','CONFLICT','UNKNOWN')), payload_hash TEXT NOT NULL, payload_json TEXT NOT NULL, result_json TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sync_outbox (
  id TEXT PRIMARY KEY, operation_id TEXT NOT NULL UNIQUE REFERENCES library_operations(operation_id), operation_type TEXT NOT NULL,
  payload_json TEXT NOT NULL, payload_hash TEXT NOT NULL, created_at TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN ('PENDING','SENDING','SYNCED','CONFLICT','FAILED')),
  retry_count INTEGER NOT NULL DEFAULT 0, next_retry_at TEXT, last_error_code TEXT, last_error_message TEXT, synced_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_outbox_pending ON sync_outbox(status, next_retry_at, created_at);
CREATE TABLE IF NOT EXISTS sync_conflicts (
  id TEXT PRIMARY KEY, operation_id TEXT, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, local_version INTEGER, remote_version INTEGER,
  reason TEXT NOT NULL, local_payload_json TEXT NOT NULL, remote_payload_json TEXT, status TEXT NOT NULL CHECK(status IN ('OPEN','RESOLVED')), created_at TEXT NOT NULL, resolved_at TEXT
);
CREATE TABLE IF NOT EXISTS sync_state (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE VIRTUAL TABLE IF NOT EXISTS book_title_fts USING fts5(title, authors, subject, content='book_titles', content_rowid='rowid');
CREATE TRIGGER IF NOT EXISTS book_title_ai AFTER INSERT ON book_titles BEGIN INSERT INTO book_title_fts(rowid,title,authors,subject) VALUES (new.rowid,new.title,new.authors,coalesce(new.subject,'')); END;
CREATE TRIGGER IF NOT EXISTS book_title_ad AFTER DELETE ON book_titles BEGIN INSERT INTO book_title_fts(book_title_fts,rowid,title,authors,subject) VALUES ('delete',old.rowid,old.title,old.authors,coalesce(old.subject,'')); END;
CREATE TRIGGER IF NOT EXISTS book_title_au AFTER UPDATE OF title,authors,subject ON book_titles BEGIN INSERT INTO book_title_fts(book_title_fts,rowid,title,authors,subject) VALUES ('delete',old.rowid,old.title,old.authors,coalesce(old.subject,'')); INSERT INTO book_title_fts(rowid,title,authors,subject) VALUES (new.rowid,new.title,new.authors,coalesce(new.subject,'')); END;
