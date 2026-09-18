# Browser verification evidence

Generated: 2026-09-16T10:20:36.252Z

The suite uses fresh isolated Playwright Chromium contexts and visible UI actions. It does not import application modules, mutate hidden state or storage, or connect to the user browser. DOM measurements and the short NFC state observer are read-only.

| Viewport | Scenario checks | PNGs | Failures | Runtime errors | External requests |
| --- | --- | --- | --- | --- | --- |
| 1600×900 @1× | 9/9 | 62 | 0 | 0 | 0 |

Run from the repository root: `node scripts/qa-browser.mjs`. For a targeted rerun use `--viewport=390x844`; aggregate reports retain the latest evidence for all four sizes. The script uses the existing bundled Playwright and Sharp dependencies; it installs nothing.

`browser-report.json` contains complete measurements and browser events. `browser-summary.json` gives the concise results. `browser-<width>x<height>.json` records each isolated viewport run. Reports are stored in `.qa/reference-regression` and full-size PNGs in `screenshots/reference-regression/<viewport>`. The current main captures are `01-home.png`, `02-identify.png`, `09-scan-two.png` and `10-confirmation.png`; `screenshots/reference-regression/overview.png` assembles these four screens. Earlier baseline captures in other folders are preserved. Rebuild this contact sheet with `node scripts/qa-browser.mjs --compose-only`, using the same EDUS_QA_REPORT_ROOT and EDUS_QA_SCREENSHOT_ROOT environment overrides.

The gallery captures all four school-card states (idle, reading, success, error) and reduced-motion behavior. Static idle is sampled over time and may never invent success. Real mock-adapter identification records idle → reading → success before showing the selected reader. Both return modes require reader identification before books. At 1600×900 the second display is checked in idle, active, success, and after its real success timeout clears personal information. No physical NFC/scanner device, Ubuntu kiosk session, or real backend is covered by these browser tests.
