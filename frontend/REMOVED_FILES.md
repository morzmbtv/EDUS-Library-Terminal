# Not transferred from the read-only migration source

| Source/module | Decision | Reason |
|---|---|---|
| src-tauri, installer-edge, Windows/kiosk scripts | KEEP in legacy / omit here | Backend or obsolete delivery responsibilities |
| src/infrastructure/tauriAdapter, unavailableAdapter, runtimeConfig, libraryApi | Omit | One same-origin production API client replaces delivery branches |
| src/domain mock engine, persistence, fixtures | Omit | Backend owns business state; tests use transport doubles only |
| RegistrationView, StudentDisplay, ComponentGallery | Omit | Not part of agreed kiosk routes |
| TestReadersPanel, TerminalTestPanel, ServiceDiagnostics, CardReaderDiagnostics | Omit | Administrative Configurator ownership |
| ReaderSearch, DemoScenarioPanel | Omit | Manual reader search and runtime mock scenarios prohibited |
| configurator.html / configurator.ts | Omit | Configurator belongs to backend package |
| public/runtime-config.js | Omit | No legacy cloud/mock runtime config |
| old card artwork variants and assets/validation | Omit | Not referenced by current SVG card component; QA material remains in original project |
| .qa, screenshots, logs, old dist/installers, target, node_modules | Omit | Generated material; dependencies rebuilt from lockfile |

No original source file, database, credential, installer or rollback was deleted. Temporary extraction utilities are archived outside the new source tree. Current runtime imports and installer file list were checked before exclusion.

Portable delivery revision: retired `installer/frontend.nsi`, `scripts/package-windows.mjs` and `package:windows`. `npm run build` still builds the independent frontend. The external deployment workspace collects only its production dist into one ZIP. Historical installer artifacts remain available; this release does not generate a frontend installer.
