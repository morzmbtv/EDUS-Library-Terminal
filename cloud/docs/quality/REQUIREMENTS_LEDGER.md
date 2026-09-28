# Cloud requirements ledger

| Area | Status | Evidence |
|---|---|---|
| PostgreSQL local service/data on E: | PASS | `edus-postgres-17`, `E:\Codex\data\edus-postgres` |
| Laravel 12 Cloud project | PASS | `artisan --version` |
| Tenant schema, constraints, indexes | PASS | migration `2026_09_22_000100...` |
| Enrollment/device auth | PASS | feature tests |
| Bootstrap/delta/cursor | PASS | feature tests |
| Issue/return/reservation/idempotency/audit | PASS | feature tests |
| Terminal HTTP adapter/outbox UAT | PASS; bootstrap/delta apply PENDING | Rust tests |
| Face provider | NOT APPLICABLE | explicitly out of scope |
| Production infrastructure | BLOCKED | domain/TLS/secret store/live data not supplied |
