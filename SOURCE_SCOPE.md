# Publication scope

Published to the public GitHub repository `morzmbtv/EDUS-Library-Terminal` on branch `codex/portable-2.0-source`.

| Folder | Source | Files |
|---|---|---:|
| `frontend/` | `edus-library-frontend`, commit `c94f59c203c652aa47a1f19765566d664bdd4a94` | 70 |
| `backend/` | `edus-library-backend`, commit `f24098901b6fc97f28bc8af2c7d1fb62f24244dd` | 94 |
| `deployment/` | `edus-library-deployment`, commit `bb3510d54c86d6c47d3c3e0e8a1b4ebfacce0da4` | 16 |
| `cloud/` | `edus-library-cloud` local source snapshot (no Git repository) | 60 |

Cloud publication excludes `.env`, `.env.testing`, local SQLite databases, Composer `vendor`, runtime `storage`, PHPUnit cache, and generated/empty favicon files. The other folders use Git-tracked source files, excluding Cargo's generated `.cargo-ok` marker from vendored dependencies.

The consolidated source snapshot contains 240 project files plus this scope note and the root README. No release ZIP, installer, build output, test database, log, or secret-bearing local configuration was uploaded.
