# Iteration 2 browser evidence

Verified locally on 2026-09-16 using isolated Chromium contexts at http://127.0.0.1:5173. The suite uses visible controls and scanner-like keyboard events; DOM access only measures rendered layout and observes NFC state transitions. It does not import application code or inject application/storage state.

## Result

- 28/28 scenario groups passed: 7 at each of 1920×1080, 1600×900, 1366×768, and mobile fallback 390×844, DPR 1.
- 164 full viewport PNGs, 41 per viewport, plus the six-screen overview at `screenshots/iteration2/overview.png` (1672×1614).
- No JavaScript/console errors, HTTP failures, failed network requests, or external network requests.
- No horizontal overflow, text clipping, text below 16px, visible touch targets below 56px, or obscured centers of enabled controls in the captured states.
- Desktop home, identification and success screens fit without page scrolling. Mobile uses vertical scrolling. Long lists scroll internally; at 1366×768 the second outstanding-loan row extends below the scroll viewport.

## Coverage

1. Three home actions; separate issue/return reader-first screens; instruction, manual lookup, no technical footer; contextual help and restored search focus.
2. Return via card; unknown-card error; real adapter reading/success transitions; current-reader copy; another reader's copy rejected with explicit warning; explicit change-reader confirmation; repeated scanner events blocked while help remains open; confirmation, success, cleared next session.
3. Return via manual reader lookup; ISBN resolves against selected reader and quantity; canceling a reader change preserves context; confirmed change clears basket; ISBN uses the newly selected reader; legacy return completes.
4. Issue regression; repeated copy does not duplicate basket; lost local server blocks continuation while preserving context; internet outage does not block local operations; unknown result survives reload and recovers safely.
5. Registration with Kazakh text; on-screen keyboard remains usable after contextual help; full unique-copy registration completes.
6. Four card illustration states; reduced-motion idle, reading, success and error. Reading line and outcome markers stay visible; running animations equal zero. Idle does not invent success.
7. Direct issue/accept scan/confirm routes require selecting a reader.

## Reproduction

`node scripts/qa-iteration2.mjs` runs all four sizes.

`node scripts/qa-iteration2.mjs --viewport=1366x768` reruns one size and retains the other latest results in the combined report.

`node scripts/qa-iteration2.mjs --motion-only` refreshes the four-state and reduced-motion evidence at all four sizes while preserving the other scenario results.

`node scripts/qa-iteration2.mjs --compose-only` rebuilds the contact sheet from the existing unaltered 1600×900 PNGs.

The bundled runtime provides Playwright and Sharp; `CODEX_NODE_MODULES` can override its package root, and `EDUS_QA_URL` can override the local URL. No project dependency was added.

## Evidence files

- `iteration2-report.json`: complete combined measurements, screenshot paths, observations and per-viewport timestamps.
- `iteration2-summary.json`: scenario results and condensed error/overflow/touch/text evidence.
- `iteration2-<viewport>.json`: latest evidence for each viewport.
- `iteration2-overview.json`: overview dimensions and the six source captures.

Rendered PNGs were opened and visually inspected, including all six overview screens, 1366×768 return scanning and help, mobile header/help, keyboard/help registration and reduced-motion reading. An initial overlap between the return scanner, heading and manual button was fixed and recaptured. A single interrupted 1366 test coincided with a confirmed HMR reload; a complete stable rerun passed. Stale failure captures from that interrupted run were removed.

Hardware is represented by the application's existing demonstration adapter. These results verify browser workflows and rendered states; they do not claim verification of a physical card reader or library server.
