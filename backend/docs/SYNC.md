# Cloud sync boundary

The local backend alone owns `sync::ProductionCloudHttpApi`, `SyncEngine`, canonical DTO validation and `CloudProjectionApplier`. The cloud repository remains a separate deployment; no frontend imports or Laravel source are in this workspace.

Runtime selects a production transport only from explicitly enrolled workspace configuration and protected device credential. UAT has no Cloud transport. In-memory TestCloud was removed from the production binary; a small fixture exists only under cfg(test). No internet-detection fallback exists.

Enrollment, bootstrap, push ACK/conflict and FULL/TOMBSTONE delta apply reuse the existing protocol v1. Terminal version is independently reported as 2.0.0-rc.1. Projection and cursor advance commit atomically. Pending local rows are protected; conflicts and rejected operations remain retained. The background worker borrows the DB mutex only during database work, not across HTTP calls (`sync_shared`). Requests have a 15-second transport timeout and redirects disabled.

Cloud URL requires HTTPS; debug-only explicit loopback HTTP opt-in is for integration testing. No browser-facing endpoint exposes Cloud credentials. Real HTTP two-database integration test is ignored by default because it requires a configured isolated Laravel/PostgreSQL fixture; a skipped test must not be reported as passed.
