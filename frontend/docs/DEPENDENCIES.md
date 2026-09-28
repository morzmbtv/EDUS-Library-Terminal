# Dependency review

Runtime direct dependencies: Vue3.5.42 (MIT), @lucide/vue1.46.0 (MIT). Both are imported by production UI. Development tools remain in package-lock only and are absent from the installer. Prettier3.9.9 adds the requested format gate; no other dependency was upgraded for this extraction. Removed Tauri/native/browser mock packages.

2026-09-25 npm audit:0 critical/high/moderate/low/info vulnerabilities. This is an advisory snapshot, not a permanent security guarantee. Locked direct/transitive package versions and declared licenses are recorded in DEPENDENCY_LICENSES.json (174 packages, including platform optional packages). Noto Sans distribution retains its SIL Open Font License. No external runtime font/CDN request.
