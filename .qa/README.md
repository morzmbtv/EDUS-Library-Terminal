# Browser verification evidence

Generated: 2026-09-16T07:23:58.791Z

The suite uses fresh isolated Playwright Chromium contexts and visible UI actions. It does not import application modules, mutate hidden state or storage, or connect to the user browser. DOM measurements and the short NFC state observer are read-only.

| Viewport | Scenario checks | PNGs | Failures | Runtime errors | External requests |
| --- | --- | --- | --- | --- | --- |
| 1920×1080 @1× | 8/8 | 53 | 0 | 0 | 0 |
| 1600×900 @1× | 9/9 | 62 | 0 | 0 | 0 |
| 1366×768 @1× | 8/8 | 53 | 0 | 0 | 0 |
| 390×844 @1× | 8/8 | 54 | 0 | 0 | 0 |

Run from the repository root: `node scripts/qa-browser.mjs`. For a targeted rerun use `--viewport=390x844`; aggregate reports retain the latest evidence for all four sizes. The script uses the existing bundled Playwright and Sharp dependencies; it installs nothing.

`browser-report.json` contains complete measurements and browser events. `browser-summary.json` gives the concise results. `browser-<width>x<height>.json` records each isolated viewport run. The main full-size PNGs are `screenshots/1600x900/01-home.png`, `02-identify.png`, `09-scan-two.png` and `10-confirmation.png`. The `screenshots/overview.png` contact sheet uses the equivalent `nfc-idle-pause.png` frame for the second tile, where the blue card has approached and is fully opaque. The full-size original captures are preserved. Rebuild just the contact sheet with `node scripts/qa-browser.mjs --compose-only`.

The gallery captures all five NFC states and reduced-motion behavior. The real mock-adapter identification records idle → reading → success before showing the selected reader. At 1600×900 the second display is checked in idle, active, success, and after its real success timeout clears personal information. No physical NFC/scanner device, Ubuntu kiosk session, or real backend is covered by these browser tests.
