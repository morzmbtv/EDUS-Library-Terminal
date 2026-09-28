# Frontend release 2.0.0-rc.1

The current delivery is ONE portable ZIP assembled outside this source repository:
`E:\Codex\artifacts\EDUS-Library-Portable-2.0.0-rc1.zip`.

This repository builds only production `dist` and its frontend-manifest.json. The deployment pipeline validates it, names the delivered metadata frontend/manifest.json, and places it beside backend binaries in one release assembly. No Rust/Vue source layers are merged.

Run the portable Setup-EDUS.ps1 after extraction from administrator PowerShell. Frontend is installed under `%ProgramFiles%\EDUS Library\frontend`. Setup stages and validates the complete new release before stopping the known service and activating it. Repeated Setup and Repair restore missing/corrupted static files without deleting ProgramData. Source tests and E-only activation tests do not prove actual elevated Windows installation.

Separate NSIS/MSI scripts are retired from the current source; their earlier versions remain in Git/checkpoints. Do not run the old frontend installer/uninstaller to maintain a portable deployment.

If Edge opens before the backend listener exists, Vue cannot load its own retry screen. Loaded-UI recovery is implemented; physical boot ordering requires Assigned Access UAT. No extra localhost server is introduced.
