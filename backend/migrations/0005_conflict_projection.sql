CREATE TABLE IF NOT EXISTS conflict_projections (
    operation_id TEXT PRIMARY KEY REFERENCES library_operations(operation_id),
    applied_at TEXT NOT NULL
);
