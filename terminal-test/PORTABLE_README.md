# Portable EDUS Library Terminal TEST

`edus-library-terminal-test-portable-<version>.zip` is the no-installer package for a prepared Ubuntu terminal.

## What the operator opens

After unpacking, leave this structure intact and double-click **EDUS Библиотека — тест.desktop** in the `EDUS-Library-Test` folder. The desktop file intentionally locates `launch/launcher` beside itself; it is not a standalone shortcut and must not be moved out of the folder.

The first time Ubuntu asks, choose **Allow Launching** / **Allow executing file as program** in the file manager. This is a graphical trust decision; no shell command or administrator right is required. If a file manager opens `.desktop` as text or blocks arbitrary folder launchers, open that same unpacked folder in Files/Nautilus, right-click the desktop file and enable **Allow Launching** in its properties. The package does not bypass this Ubuntu protection.

## Prerequisites

- Python 3, available as `python3`;
- Google Chrome, available as `google-chrome` or `google-chrome-stable`.

No Node.js, npm, Docker, internet connection, systemd service or administrator right is needed on the terminal. Chromium is not selected automatically: absence of Google Chrome is reported as an actionable error.

The launcher starts a static loopback service only at `http://127.0.0.1:43180`, waits for its own version-and-instance health endpoint, then opens Google Chrome kiosk with the dedicated test profile. A conflicting service, other package instance or other version is never terminated or silently reused.

## Local state

The portable launcher shares the confirmed dedicated test Chrome profile with the installed terminal-test package:

- test operations, card mappings and scan mappings: `~/.config/edus-library-terminal-test/chrome-profile`;
- portable service PID, log and last graphical error: `~/.local/state/edus-library-terminal-test-portable/`.

It never deletes either location. The portable bundle contains no Safe School, canteen or production API configuration.

## Validation scope

The archive itself is validated after packaging: expected structure, UTF-8 filenames, executable Unix ZIP attributes and static app files. It is also unpacked to a path containing spaces and Cyrillic characters; the loopback server is tested for matching and mismatched package instance IDs. Actual click-to-launch via a Ubuntu graphical shell, a reboot test, and hardware tests remain on-site checks; use `HARDWARE_TEST.md` after launching.
