# Local API v1

The backend is the source of truth: `openapi/local-service-v1.openapi.json`. Copy a reviewed contract revision when updating the client, never import source from another project. Contract tests validate actual client requests, required nullable fields and unknown-field rejection against this snapshot.

All paths are centralized beneath `/api/local/v1`; no Cloud endpoint or credentials are exposed. The service issues an HttpOnly SameSite Strict session and a transient CSRF token. Mutations use JSON and `X-EDUS-CSRF`. Session renewal may replay only a rejection that occurred before domain dispatch. Network uncertainty on a mutation retains `operationId` and switches to receipt verification. It never blindly repeats issue/return.

SSE endpoint `/events` emits `status`. EventSource reconnects; a failed stream triggers a safe read refresh which can renew expired HTTP session context. Closing the UI disposes the stream.

API v1 supports snapshots, card identification, reader loans, scan resolution, catalogue search/availability, issue/return/reservation/cancellation, operation receipts, safe sync state and language. Administrative actions exist only in the backend Configurator and are absent here.
