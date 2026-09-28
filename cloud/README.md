# EDUS Library Cloud 1.1.0

Laravel 12 + PostgreSQL 17. Server-side device credential determines the school; request school_id cannot override it. This is separate infrastructure, never bundled into the terminal application.

See docs/DEPLOYMENT.md for production deployment. OpenAPI: openapi/terminal-v1.openapi.json. Minimum terminal: 1.1.0, sync protocol 1, canonical FULL/TOMBSTONE payloads.

Terminal setup now uses the administrator-protected native first-run wizard: HTTPS URL, one-use enrollment code, device name. Cloud returns school/terminal IDs, device credential and school card HMAC secret. Windows Credential Manager stores terminal secrets. No hidden TestCloud fallback.

Development tests require the dedicated edus_library_test PostgreSQL database. php artisan test exercises transactions, tenant isolation, idempotency, canonical conflicts and migrations. tests/integration-fixture.php refuses any other DB name and is CLI-only. Demo seeding is forbidden in production. Never run migrate:fresh on production.

Source migration history is additive. Back up PostgreSQL and APP_KEY before deployment; APP_KEY protects enrolled card keys and must not be regenerated on upgrade. Public Cloud deployment/TLS/monitoring and infrastructure policy approval remain external release gates.
