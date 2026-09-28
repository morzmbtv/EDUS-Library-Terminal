# EDUS Library Frontend

Version 2.0.0-rc.1. Independent Vue 3 / TypeScript interface for Microsoft Edge. It has no native shell, database, Cloud connection, service administration or production mock engine.

The browser uses the **same origin** as EDUS Library Local Backend. Installed URL: `http://127.0.0.1:43180`. Frontend and backend sources stay independent; a separate deployment workspace assembles them into one portable ZIP.

## Development

`npm ci`, `npm run typecheck`, `npm test`, `npm run lint`, `npm run build`.

Use `npm run dev` with a separately running Local Backend. Vite is a development tool only and is never installed on the terminal. There is no fake database fallback when the backend is absent.

## Delivery

1. Extract EDUS-Library-Portable-2.0.0-rc1.zip.
2. Run scripts/Setup-EDUS.ps1 from administrator PowerShell.
3. Open EDUS Terminal Configurator and configure UAT or Production.
4. Run scripts/Verify-EDUS.ps1, verify the local URL, then configure Microsoft Edge Assigned Access externally.

Setup does not configure kiosk policies. ProgramData, service credentials and operational records belong to the backend and are preserved during normal Setup/Repair/Remove.

See [architecture](docs/ARCHITECTURE_FRONTEND.md), [API contract](openapi/local-service-v1.openapi.json), [testing](docs/TESTING.md) and [release](docs/RELEASE.md). Legacy combined delivery remains in the old terminal project; it is not a dependency of this project.
