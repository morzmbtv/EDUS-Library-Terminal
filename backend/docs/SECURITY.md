# Backend security boundaries

The service binds loopback only. Same-origin Host/Origin, session/CSRF and request bounds protect the browser API; CSRF is not authentication of every local process. Sensitive commands are available only via administrator-authenticated named pipe, never terminal HTTP. Configurator requests elevation and does not store Windows passwords.

## Stable DPAPI model

`CredentialStore::for_service_workspace` uses Windows **user-scope DPAPI under the service identity**. CryptProtectData receives CRYPTPROTECT_UI_FORBIDDEN; it does NOT receive CRYPTPROTECT_LOCAL_MACHINE. This exactly preserves the previous service scope. Service identity must remain LocalService. Files remain in Production/UAT `secrets.dpapi` using the same namespace and key names; setup must restrict workspace ACL to service SID, SYSTEM, Administrators. A Configurator never opens DB or creates its keys.

Legacy per-user Credential Manager / keyring entry points were removed from this new backend. The old Tauri project remains untouched as rollback and is not a supported automatic source of service secrets. Existing service databases/keys are not moved or recreated.

Raw card input is normalized once and HMAC-SHA256 stored; leading zeros remain meaningful. Database key, HMAC secret, Cloud credential, raw UID and image/template content must never appear in logs or frontend payloads. Service key absence with existing DB fails closed. Face success requires the configured provider boundary; production without provider is unavailable, not simulated.

## Recovery

UAT restoration requires authenticated admin IPC and explicit confirmation. Production restore is deliberately refused: historical sync cursor/device state cannot safely be rewound independently of Cloud reconciliation. No generic SQL, command, registry or arbitrary file API is exposed. Protected backups remain local and encrypted under the unchanged workspace key.

SQLCipher hardening is mandatory; there is no no-mlock fallback. Actual service ACL/DPAPI after SCM startup, upgrade and reboot must be verified on an authorized test machine; console tests do not prove them.
