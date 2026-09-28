# Vendored SQLCipher 4.18.0

This crate is a local patch of `libsqlite3-sys 0.38.2` used by `rusqlite 0.40.2`.

- Upstream source: <https://github.com/sqlcipher/sqlcipher/releases/tag/v4.18.0>
- Generated with upstream `nmake /f Makefile.msc sqlite3.c USE_AMALGAMATION=1 NO_TCL=1` using MSVC on Windows.
- `sqlcipher/sqlite3.c` SHA-256: `964C72BD1D3E031862588202E2BF6342D36EC68A2CAE4F4276D9CD79E6571ACB`
- Purpose: incorporate SQLCipher 4.18.0's Windows fix for logging when `PRAGMA cipher_memory_security = ON`.

The Rust bindings, crate version, OpenSSL provider configuration, and enabled Cargo features remain the existing `libsqlite3-sys 0.38.2` interface.
