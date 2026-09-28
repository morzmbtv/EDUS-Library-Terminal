# Reproducible build

Node/npm are build-time only. Pinned package-lock controls Vue3, Lucide, Vite, TypeScript, ESLint and Prettier. Runtime dependencies: Vue3 and Lucide Vue only. Both use MIT licenses; local Noto Sans font uses SIL OFL. `npm audit` reports no current vulnerabilities at the tested lock revision.

`npm run build` generates production files then `frontend-manifest.json`: frontend version, required API1, deterministic SHA256 build hash, UTC build time, per-file SHA256. The manifest does not hash itself. No source maps, backend binary or secrets are packaged.

Portable assembly is outside this repository: `E:\Codex\projects\edus-library-deployment\build-edus-portable.ps1`. It validates dist hashes, assembles release binaries/static files/scripts, creates one ZIP and re-extracts it for verification. There is no current frontend NSIS command. Development caches/artifacts use E; installation never requires E on a terminal.
