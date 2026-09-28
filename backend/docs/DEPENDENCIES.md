# Backend dependency and advisory audit

Audit date: 2026-09-25. Scope: the new `edus-library-backend` Rust workspace only. No dependency versions were upgraded by this audit; no compiler/toolchain was installed. The old terminal and Cloud repositories were not changed.

## Resolved inventory

Windows target metadata contains **153 packages**: **5 internal workspace**, **18 direct external**, **130 transitive external**. Direct means used by at least one workspace crate; a package may also be transitively referenced. Dev dependencies are included and identified. Machine inventory: [dependencies.inventory.json](dependencies.inventory.json). It records versions, license expressions, enabled features, dependency edges and direct crate owners.

Cargo.lock SHA-256 at audit: `43498BEE8BA1553C8D9AA36CD3BF50A259E1934B929C6B9B864F430676B9D23E`.

| Direct dependency | Resolved version | Declared license | Existing use |
|---|---|---|---|
| axum | 0.8.9 | MIT | Loopback REST routing, typed JSON and HTTP middleware |
| chrono | 0.4.45 | MIT OR Apache-2.0 | UTC timestamps, operational dates |
| hmac | 0.12.1 | MIT OR Apache-2.0 | Card HMAC-SHA256 |
| rand | 0.9.5 | MIT OR Apache-2.0 | Random service credentials |
| rusqlite | 0.40.2 | MIT | SQLCipher domain transactions and Backup API |
| serde | 1.0.229 | MIT OR Apache-2.0 | Typed domain, HTTP and admin IPC serialization |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | Canonical operation/receipt and bounded IPC JSON |
| sha2 | 0.10.9 | MIT OR Apache-2.0 | Content, operation and static-manifest hashes |
| tempfile | 3.27.0 | MIT OR Apache-2.0 | Isolated test DB fixtures (dev only) |
| tokio | 1.53.1 | MIT | Async HTTP, SSE, admin pipe and timers |
| tokio-stream | 0.1.19 | MIT | SSE stream adapters |
| tower | 0.5.3 | MIT | Router integration test harness (dev only) |
| ureq | 3.4.2 | MIT OR Apache-2.0 | Bounded HTTPS Cloud transport with redirects disabled |
| url | 2.5.8 | MIT OR Apache-2.0 | Validation of configured Cloud URL |
| uuid | 1.26.1 | Apache-2.0 OR MIT | Operation/entity IDs, sessions and nonces |
| windows | 0.61.3 | MIT OR Apache-2.0 | Typed Win32/DPAPI/token/SCM/native Configurator bindings |
| windows-service | 0.8.1 | MIT OR Apache-2.0 | Windows SCM dispatcher, state and control handles |
| zeroize | 1.9.0 | Apache-2.0 OR MIT | Wipe temporary sensitive native UI request buffers |

The split removed unused core `keyring`, `directories` and `thiserror`, plus Tauri/WebView2 terminal-shell dependencies. The native Configurator now uses Win32 bindings rather than the complete terminal frontend. Production TestCloud transport was removed; its independent unit fixture compiles only under cfg(test). No Rust package upgrades were used to hide architectural boundaries.

## RustSec execution

Executed the installed `cargo-audit` against this Cargo.lock, with no ignored advisories:

```text
cargo-audit audit --json --db E:\Codex\cache\edus-library-tauri\cargo\advisory-db --file Cargo.lock
```

Result: **exit 0; 0 known vulnerable package matches; no informational warnings**. This is an actual advisory scan, not inference from package names. Cargo-audit scanned **187 lockfile dependencies**; that count differs from the platform-filtered metadata because the lockfile also retains platform alternatives. Evidence: [dependencies.rustsec.json](dependencies.rustsec.json).

Official database origin was verified as `https://github.com/RustSec/advisory-db.git`. Snapshot commit `593df8c1b5ed0bcde9dddadfeeead776fa514ff8`, updated `2026-09-24T16:38:19+02:00`, **1269 advisories**. Official sources: [RustSec database](https://rustsec.org/advisories/), [database repository](https://github.com/RustSec/advisory-db).

## License/native boundaries and limitations

All 148 external packages in Windows-filtered metadata declare license expressions. These are permissive families (MIT, Apache-2.0, BSD, ISC, Unicode-3.0, CDLA-Permissive-2.0 or combinations), not proof of a complete legal redistribution review. The five internal EDUS packages do not declare a public license in Cargo.toml; no license was invented for company-owned source.

The locally patched libsqlite3-sys 0.38.2 uses SQLCipher 4.18.0. Metadata calls the bindings MIT; this does **not** replace the SQLCipher embedded BSD-style notice, SQLite public-domain notice, or OpenSSL notices. See [VENDOR_SQLCIPHER.md](VENDOR_SQLCIPHER.md) for upstream source, build flags and independently verified amalgamation hash. The patch is unchanged from the migration source.

RustSec only reports known advisories represented in its database and matched by package/version. It does not prove that custom Rust/FFI code is secure, fully assess vendored native SQLCipher/OpenSSL source against every upstream CVE, review every vendored license text, or cover Microsoft Edge, Windows, NSIS and the Microsoft VC Runtime. Installer provenance, runtime component signing and Windows patch level are separate release checks. Physical service identity/ACL/DPAPI tests likewise cannot be inferred from a dependency scan.

Re-run this audit if Cargo.lock or the native vendor changes. The inventory is for the Windows MSVC target; other operating-system dependency trees are not acceptance targets for this backend.
