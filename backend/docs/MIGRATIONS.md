# Schema migrations and preservation

The seven inherited SQL migration files at `migrations/` are byte-preserved and compiled into LocalDatabase::open. They run in a transaction and use existing compatible CREATE IF NOT EXISTS rules. The split introduces one additive service recovery audit table and no key migration and does not rename service workspace paths.

1. Initial domain/outbox/FTS schema.
2. Cloud projection/state tables.
3. Administrator audit table (historical records preserved).
4. UAT external reader identifiers.
5. Conflict reconciliation projection.
6. Recovery audit.
7. Query indexes.
8. Service recovery audit (new, additive; records completed UAT restore without rewriting legacy CHECK constraints).

Existing SQLCipher file without its corresponding key is refused. No empty replacement DB is created over it. No automatic per-user Tauri migration is provided by the split. Old Tauri data stay untouched; a clean UAT setup does not require migration. Binary rollback must not be used to reverse future incompatible DB migrations.

UAT backup restore requires exact current schema match. Future migrations must include upgrade and rollback strategy, fixtures preserving loans/outbox/conflicts and interrupted-transaction tests.
