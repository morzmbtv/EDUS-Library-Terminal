# Local database

The Windows service exclusively owns `LocalDatabase` in `crates/edus-library-core/src/db`. ProgramData's existing `production/library.db` and `uat/library.db` paths remain unchanged. Development tests use E:\Codex\temp.

SQLCipher 4.18.0 is enforced at open; `cipher_memory_security=ON`, WAL, `foreign_keys=ON`, `secure_delete=ON`. An existing database requires its existing DPAPI key; a missing key never triggers replacement. New UAT creates fictional inventory only; readers are explicit administrative actions. Production never seeds.

Domain validation, mutations, durable operation receipt and outbox are performed in the same SQL transaction. Stable operation IDs prevent duplicate issue/return, and COPY versus LEGACY_TITLE constraints remain database-enforced. Indexes cover active reader loans, active copy uniqueness, pending outbox and FTS search. Migrations remain embedded Rust compile-time assets, not frontend files.

Backup uses SQLite Backup API, not copying a live WAL database. See BACKUP_RESTORE.md. Browser storage has no operational ownership.

Verification: core tests cover SQLCipher no-key failure, reopen, idempotency, foreign-copy protection, partial return, reservation uniqueness, cursor atomicity and card HMAC. DPAPI in a test process is not evidence of installed LocalService ACL/reboot acceptance.
