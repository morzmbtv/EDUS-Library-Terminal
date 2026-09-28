# SQLCipher provenance

- Original source: https://github.com/sqlcipher/sqlcipher/releases/tag/v4.18.0
- Vendor: `vendor/libsqlite3-sys-0.38.2`, patched amalgamation consumed by rusqlite 0.40.2.
- SQLCipher version: 4.18.0, enforced at runtime.
- `sqlcipher/sqlite3.c` SHA-256 independently recomputed: `964C72BD1D3E031862588202E2BF6342D36EC68A2CAE4F4276D9CD79E6571ACB`.
- Existing generation: upstream nmake Makefile.msc sqlite3.c USE_AMALGAMATION=1 NO_TCL=1; source unchanged during repository split.
- Cargo flags: bundled-sqlcipher-vendored-openssl, backup, chrono, uuid, serde_json. build.rs enables SQLITE_HAS_CODEC, SQLITE_ENABLE_FTS5, SQLITE_THREADSAFE=1, bundled OpenSSL provider.
- SQLCipher community code carries the embedded Zetetic BSD-style redistribution notice; SQLite portions are public domain; Rust libsqlite3-sys bindings are MIT. Vendored OpenSSL retains its own notices/licenses. See exact source notice and Cargo dependency license report; the libsqlite3-sys MIT file is not a replacement for SQLCipher notices.

No SQLCipher files belong in frontend. No vendor version upgrade was made as part of this refactor.
