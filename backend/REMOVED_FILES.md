# Excluded and rewritten migration sources

The legacy repository has not been deleted or cleaned. ARCHIVE means it remains there and is excluded from this repository. Complete source inventory: `E:\Codex\projects\EDUS_SOURCE_INVENTORY.csv`; entrypoint map is adjacent.

| Source group | Decision | Usage/reason |
|---|---|---|
| src/, public/, Vue/TypeScript tests, package*.json, Vite | MOVE to independent frontend selectively | browser UI; no place in backend |
| src-tauri/src and Tauri config/capabilities | ARCHIVE | obsolete shell remains rollback; no new runtime imports |
| src-tauri/src/configurator.rs and Vue configurator entry | REWRITE native Win32 crate | remove shared kiosk bundle, Node and WebView2 dependency |
| scripts/windows-kiosk, terminal-test Linux/browser harness | ARCHIVE | prior delivery/OS-control models; not called by new installer |
| .qa, screenshots, dist, target, logs, old installers | ARCHIVE | generated evidence/artifacts, not source dependencies |
| old docs and reports | REWRITE current responsibility docs | historical PASS cannot establish new acceptance |
| core per-user Runtime::open, keyring/directories/thiserror deps, production LocalTestCloud | DELETE from migrated core after usage review | service-only DPAPI ownership; test fixtures remain cfg(test) |
| combined packaging Node script | ARCHIVE | current external deployment workspace assembles one ZIP from independent builds |
| duplicated admin protocol structs | MOVE common admin-ipc crate | Configurator/service use one versioned allowlist |

SQLCipher vendor is intentionally retained with provenance/hash/license. Existing seven migrations are unchanged; migration 8 adds a recovery audit table without deleting historical audits. Unknown root files were not blindly copied.

Portable delivery revision: removed `installer/backend.nsi` and `scripts/package-windows.ps1` after checking usages. Replaced by `edus-library-deployment/build-edus-portable.ps1` and fixed-purpose Setup/Verify/Repair/Remove scripts. Existing historical installers and source checkpoint remain on E:. No DB or credential removed.
