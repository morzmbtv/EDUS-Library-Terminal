# EDUS Library source snapshot

This branch consolidates the active EDUS source projects for review and checkout. The existing `main` branch remains unchanged.

## Projects

- `frontend/` — Vue 3 + TypeScript Edge kiosk frontend.
- `backend/` — Rust local backend, Windows service, Configurator, SQLCipher core, migrations, and local API.
- `deployment/` — Windows deployment and portable packaging scripts and guides.
- `cloud/` — Laravel/PostgreSQL Cloud source snapshot, API contract, migrations, and tests.

## Install order

See `deployment/docs/QUICK_START_RU.md` and the per-project READMEs. Cloud deployment is a separate server operation.

## Source hygiene

This branch contains source, tests, dependency lockfiles, licenses, and required vendored SQLCipher build source. It intentionally excludes generated builds, installers/archives, dependency caches, runtime logs, local databases, and machine-specific secrets/configuration. Cloud's `.env.example` is included; local `.env` files are not.

The Cloud folder was copied as a source snapshot because the active Cloud directory did not have its own Git repository at publication time. The other projects were taken from their clean committed source trees.
