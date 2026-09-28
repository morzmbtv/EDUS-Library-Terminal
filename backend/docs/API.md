# Local Service API v1

Source of truth: `openapi/local-service-v1.openapi.json`. Version endpoint exposes backend version and supported frontend range (RC currently exact 2.0.0-rc.1). Contract test validates real router JSON, including nullable values, enum values and date/date-time wire formats. Frontend keeps a checked vendored snapshot; no filesystem cross-import.

Host is fixed `127.0.0.1:43180`. Mutation requests require exact Origin, JSON Content-Type, HttpOnly SameSite Strict session cookie and CSRF header. Bodies are bounded; rejected extractor requests receive stable JSON errors. SSE `/api/local/v1/events` emits `status`. Clients renew an expired session and reconnect; uncertain mutations retain operationId and query the receipt instead of creating a new operation.

Search/availability projections are calculated in backend. Reader context is checked for issue, return and reservations. Operational persistence never lives in Edge storage. Language is persisted by the service settings endpoint. CSRF protects browser-origin requests, not all local malicious processes; Windows account/ACL and kiosk policy remain separate trust boundaries.
