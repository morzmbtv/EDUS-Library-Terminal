# Core dependency audit

Resolved from Cargo metadata for x86_64-pc-windows-msvc. No package upgraded for this cleanup. Removed unused production dependencies: keyring, directories, thiserror.

| Dependency | Resolved version | License | Actual use |
|---|---|---|---|
| chrono | 0.4.45 | MIT OR Apache-2.0 | UTC operation timestamps and due dates |
| hmac | 0.12.1 | MIT OR Apache-2.0 | Card HMAC-SHA256 |
| rand | 0.9.5 | MIT OR Apache-2.0 | Cryptographically random credential material |
| rusqlite | 0.40.2 | MIT | SQLCipher SQL transactions and backup API |
| serde | 1.0.229 | MIT OR Apache-2.0 | Typed request/response serialization |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | Canonical payload/receipt JSON |
| sha2 | 0.10.9 | MIT OR Apache-2.0 | Operation hashes and card HMAC digest |
| ureq | 3.4.2 | MIT OR Apache-2.0 | Bounded HTTPS Cloud transport |
| url | 2.5.8 | MIT OR Apache-2.0 | Cloud URL validation |
| uuid | 1.26.1 | Apache-2.0 OR MIT | Stable operation/entity identifiers |
| tempfile | 3.27.0 | MIT OR Apache-2.0 | Isolated test data directories (test only) |
| windows | 0.61.3 | MIT OR Apache-2.0 | DPAPI protect/unprotect bindings (Windows only) |

The patched native SQLCipher/OpenSSL chain has distinct notices: see VENDOR_SQLCIPHER.md. This is direct dependency usage/license evidence, not a claim that all transitive dependencies passed a new advisory scan. Root release audit records actual cargo-audit results. No runtime native library was replaced.
