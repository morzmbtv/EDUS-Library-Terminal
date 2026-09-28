# Portable release acceptance

This package changes delivery, not the domain model. Frontend/backend remain independent repositories and share HTTP/SSE API version 1. Cloud is not packaged. Tauri rollback and existing databases are untouched.

## Evidence boundaries

The release build runs frontend ci/typecheck/tests/lint/format/build; Rust fmt/clippy/check/tests/release; PowerShell 5.1 parser and safe filesystem/lifecycle tests; payload hash validation before ZIP and again after extraction. Logs and artifact provenance are saved under deployment `.build/<run-id>`. Package manifest lists every payload file, SHA-256, size, versions, build time and source revisions including dirty state.

2026-09-25 executed on the development host, all test files on E:: frontend 14/14 PASS; Rust workspace 68 PASS / 3 explicit fixture tests ignored; PowerShell 5.1 deployment checks 15/15 PASS (SCM side effects mocked, filesystem activation and in-memory Windows ACL rules real). Full typecheck/lint/format/build and Rust fmt/clippy `-D warnings`/check/release PASS. Linker LNK4099 warnings concern missing OpenSSL PDB debug symbols in the prewarmed native archive; no missing runtime DLL or unresolved linker symbol.

Actual installed Microsoft Edge + extracted release EXE + isolated UAT SQLCipher database: 8/8 browser scenarios PASS, no unexpected page/HTTP errors. Issue, search, return, reservation and new browser session language persistence were exercised. Failure/version mismatch scenarios used explicitly injected HTTP faults. Screenshots: `E:\Codex\artifacts\edus-portable-2.0-edge-e2e`. These are software-generated HID frames, not physical hardware.

Separate extracted-EXE crash/reopen probe: READY/UAT, same reader lookup, loan/reservation/copy state, locale and DPAPI blob preserved; actual VCRUNTIME140 module loaded beside the EXE, not from the development toolchain. Safe sibling frontend resolution was exercised without EDUS_SERVICE_WEB_ROOT. Scope is development-user console process, not Windows SCM recovery.

Read-only SCM inspection and a console service on an isolated E: workspace do not prove LocalService startup, service SID/ACL or DPAPI reopen under SCM. PowerShell mocked lifecycle tests do not register a service. No Windows policies are applied on the development computer.

## Target acceptance not executed

**INSTALLATION NOT VERIFIED** until an authorized disposable Windows VM or dedicated test machine completes:

| Scenario | Required assertion | Result |
|---|---|---|
| Clean Setup | SCM registered; LocalService; automatic; service SID; restricted ACL; health SETUP_REQUIRED | NOT EXECUTED |
| Repeat Setup, stopped/running service | Bounded stop; same identity and data; valid files/health | NOT EXECUTED |
| Repair/upgrade/partial prior install | Ownership first; no foreign overwrite; DB/secrets/outbox unchanged | NOT EXECUTED |
| SCM restart/crash/reboot | Recovery delays; same DPAPI key/card HMAC/SQLCipher data | NOT EXECUTED |
| Elevated Configurator | Trusted named pipe; UAT create/bind; restart; Edge identifies same card | NOT EXECUTED |
| Offline install | Network disabled; no downloads; all components launch | NOT EXECUTED |
| Remove | Service removed; ProgramData preserved by default | NOT EXECUTED |
| Explicit RemoveData | Separate confirmation; only owned data root removed | NOT EXECUTED |
| Physical Assigned Access | Reboot/autologon/Edge/touch/card/scanner/camera/breakout | NOT EXECUTED |

Unsupported production restore and legacy Tauri migration must fail closed. No empty replacement DB is created over an inaccessible encrypted database. Physical Face provider/PAD, deployed Cloud/TLS and code signing remain separate production prerequisites.
