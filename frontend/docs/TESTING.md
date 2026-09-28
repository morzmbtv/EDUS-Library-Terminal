# Verification

`npm run typecheck` checks strict TypeScript and Vue templates. `npm run lint` checks ESLint. `npm test` executes actual transport, HTTP contract, HID, camera lifecycle, navigation/draft/receipt recovery and architecture-boundary tests. Test doubles live only in tests; frontend tests never open SQLCipher.

`scripts/edge-e2e.mjs` uses installed Microsoft Edge against a separately started backend and an explicitly prepared synthetic UAT workspace. It captures 1280×800 screens and checks real HTTP library operations, reload persistence, RU/KK, no operational browser storage, failure/retry and version mismatch. Its injected error responses are marked as fault tests. The test runtime dependency path is configurable in the release environment; it is not a production dependency.

Historical split-delivery NSIS probes are not evidence for the current portable package. NSIS packaging has been retired; the deployment project owns PowerShell package validation/staging/lifecycle tests and records their actual scope.

Assigned Access, Windows reboot, physical touch/card/scanner/camera, elevation, clean install, repair/upgrade/uninstall in Program Files are NOT EXECUTED without an authorized test machine.

2026-09-25: npm ci, typecheck, lint, 14/14 tests and production build PASS. Real Microsoft Edge against final release backend2.0.0-rc.1 (console PID15708, isolated E UAT workspace): 8/8 browser scenarios PASS, zero unexpected page/network errors. Issue, return and reservation were committed through HTTP; language persisted into a fresh browser context. Evidence: E:/Codex/artifacts/edus-split-2.0-frontend-e2e/result.json and PNG files. No physical devices were asserted.

Set `EDUS_E2E_OUTPUT` to a fresh evidence directory when rerunning Edge E2E against the portable package. Preserve historical evidence separately. Current portable acceptance is recorded in the deployment project.
