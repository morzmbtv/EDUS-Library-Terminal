# Backend architecture

`EDUSLibraryService` enters Windows Service Dispatcher; its control handler owns start/stop/shutdown. Axum serves loopback HTTP/SSE, core owns one active workspace and transactions. A separate named pipe authenticates the impersonated client using BUILTIN Administrators well-known SID. No administration endpoints exist in browser HTTP.

Crates:

- `edus-library-core`: SQLCipher, migrations, identity/card HMAC, domain validation, receipts/outbox, Cloud transport/projections, backup.
- `edus-library-service`: SCM lifecycle, REST/SSE, workspace lease, static manifest validation, administrative pipe server.
- `edus-library-admin-ipc`: versioned protocol/allowlist; no DB or OS access.
- `edus-library-configurator`: native Win32 elevated UI and authenticated pipe client; no browser or direct SQLCipher access.
- `edus-installer-helper`: typed SCM configuration/ownership/lifecycle checks; fixed-purpose command allowlist.

Cloud remains the separate Laravel/PostgreSQL system. Only core contacts Cloud. Frontend source is absent. Core is statically linked into service; SQLCipher is statically bundled. Installer carries native VC Runtime, not build tools.

Current portable deployment is external: `%ProgramFiles%\EDUS Library\deployment.json` permits only the fixed sibling `frontend` directory next to `backend`. Its `manifest.json` supplies frontend/API compatibility and file hashes. Invalid deployment configuration fails closed. No frontend yields FRONTEND_MISSING while APIs and Configurator remain usable. There are no cross-repository imports; development environment overrides are explicit paths, not source dependencies. Legacy split layout `%ProgramW6432%\EDUS Library Frontend\active.json` remains readable only when portable configuration is absent; it is not the current delivery method.

Portable activation stages and validates the complete application tree, stops the service, renames directories and preserves prior binaries. Users reload Edge after an update. The legacy active-pointer reader also retains tests for its validated previous asset fallback; new portable delivery does not create active/previous pointers.
