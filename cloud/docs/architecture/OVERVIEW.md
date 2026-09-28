# EDUS Library Cloud — architecture

`edus-library-cloud` is a Laravel 12 API backed by PostgreSQL 17. It is the authoritative multi-terminal record. The Windows terminal owns only its encrypted local operational projection and outbox.

- Device enrollment consumes a scoped, one-use enrollment code and returns one terminal credential. The server stores only its SHA-256 digest.
- Every protected request authenticates a device credential and derives `school_id` from its terminal record. A body/query school id is neither accepted nor trusted.
- Bootstrap and changes return a minimum terminal projection for exactly that school. Person projections never include IIN, contacts, photos, health, grades, attendance, food or documents.
- The delta cursor signs `{school_id, sequence}` with the local server secret. It is opaque to terminals.
- Mutation handling uses a PostgreSQL transaction: domain change, terminal operation, idempotency receipt, audit event and delta records commit together.
- Face templates are opaque ciphertext to the cloud. No raw photo, video, embedding or plaintext template is created by the API.

Production deployment requires an approved domain, TLS certificate, secret store, PostgreSQL deployment and terminal enrollment process. The local service is only `http://127.0.0.1:8088`.
