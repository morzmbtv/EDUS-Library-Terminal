# EDUS Library Local Backend 2.0.0-rc.1

Independent Rust workspace: Windows SCM service, authoritative library core, native administrative Configurator and narrow installer helper. No Vue, Node, Tauri or browser business engine.

The independently built frontend uses only same-origin HTTP/SSE on `http://127.0.0.1:43180`. Both components are delivered in one portable ZIP assembled in the separate deployment workspace, with native Configurator included as an EXE. Canonical contract: [OpenAPI](openapi/local-service-v1.openapi.json). See [architecture](docs/ARCHITECTURE_BACKEND.md), [installation](docs/INSTALL.md), [tests](docs/TESTING.md), [release](docs/RELEASE.md).

This is an unsigned RC/UAT release. Console integration verification is distinct from SCM/LocalService installation and physical Assigned Access acceptance. Production restore and legacy per-user Tauri import remain fail-closed; existing data is not automatically migrated or replaced.
