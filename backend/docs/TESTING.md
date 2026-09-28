# Backend verification

Use an MSVC developer environment and E: Cargo/temp/target paths on the development host. Run `cargo fmt --all -- --check`, `cargo clippy --workspace`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --release --locked`. Backend tests do not run Vue tests.

Core tests exercise encrypted DB mutations, HMAC normalization, receipts/outbox, conflicts, encrypted backup/restore and isolation. Service tests use the production router and temporary SQLCipher/DPAPI workspaces. Contract tests check real JSON against canonical OpenAPI. Configurator tests include rejecting a real foreign named pipe; elevated GUI and SCM acceptance are separate.

The ignored `prepare_browser_fixture` creates one explicit isolated E: workspace only when absent. Run the new service `--console` with EDUS_SERVICE_DATA_ROOT pointing there and EDUS_SERVICE_WEB_ROOT pointing to an independently built frontend dist. Then test actual Microsoft Edge over HTTP. Never report this as installed LocalService.

Installation matrix requiring authorized disposable Windows: clean install, repair, stopped/running service, partial legacy installation, foreign service refusal, occupied port, Unicode install path, upgrade preserving DPAPI/outbox, offline install and uninstall preservation. Hardware/Assigned Access are NOT EXECUTED without the dedicated device.
