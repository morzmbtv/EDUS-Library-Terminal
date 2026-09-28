# Windows Service

Name `EDUSLibraryService`; display name `EDUS Library Local Service`; account `NT AUTHORITY\LocalService` with dedicated service SID. Automatic start and recovery restart after 10/30/60 seconds are configured by portable Setup-EDUS.ps1. No Session 0 UI, Explorer, CredUI or kiosk provisioning. The service binary lives under `%ProgramFiles%\EDUS Library\backend`; fixed sibling frontend is selected by `deployment.json`.

Machine data root `%ProgramData%\EDUS Library` retains production/uat directories, SQLCipher DB/WAL, secrets, settings, logs and backups. ACL allows SYSTEM, Administrators and service SID. DPAPI is **user scope under the service account**, not machine scope. Identity must remain stable on upgrade. Missing/corrupt keys fail closed; Configurator must never generate service keys in its own profile.

HTTP `/health` distinguishes READY, SETUP_REQUIRED, FRONTEND_MISSING and ERROR. SCM RUNNING alone is insufficient; installer helper validates listener PID and application health. Stop checkpoints the DB after work is drained; recovery does not delete outbox.

`--console` with explicit E: test roots is a development harness. It proves actual HTTP/SQLCipher/DPAPI under that test user, not LocalService permissions. SCM/ACL/reboot verification requires an authorized disposable Windows environment.

If Edge navigates before any listener exists, application JavaScript cannot run. Administrator must verify service startup before configuring autologon; actual boot ordering/retry behavior requires physical UAT. Once HTML has loaded, the client handles service/session/SSE reconnection.
