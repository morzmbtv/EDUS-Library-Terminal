# EDUS Library Terminal — TERMINAL TEST

> **Portable package:** for “unpack → open shortcut → Chrome”, build `npm run package:terminal-test` and use `release/edus-library-terminal-test-portable-<version>.zip`. Its opening instructions are in [PORTABLE_README.md](PORTABLE_README.md). The installed package described below remains available separately.
This is a separate offline package for preparing the touchscreen frontend and connected keyboard-wedge hardware for on-site acceptance. It serves the prebuilt static application only on `127.0.0.1:43180` through Python 3 standard library and opens it in a dedicated Chrome/Chromium kiosk profile.

It does not call Safe School, EDUS production, canteen, or other school APIs. Test readers, cards, copies, receipts, and local mappings stay in the test browser profile under the test-only localStorage namespace.

## Ubuntu install

1. Copy this directory to the terminal and make the shell scripts executable if the filesystem did not preserve modes:
   ```sh
   chmod +x scripts/*.sh scripts/terminal_test_server.py
   ```
2. Confirm dependencies before install:
   ```sh
   python3 --version
   google-chrome --version || chromium --version
   ```
3. Install only the test package:
   ```sh
   ./scripts/install-terminal-test.sh
   ```
4. Start from the created desktop entry **EDUS Библиотека — тест** or run:
   ```sh
   ~/.local/share/edus-library-terminal-test/scripts/launch-terminal-test.sh
   ```

The installer writes only to `~/.local/share/edus-library-terminal-test` and its desktop launcher. It does not overwrite the production library terminal configuration or profile.

The launcher uses the service only when its own loopback health endpoint identifies the package and exact version. It reuses that service if healthy; it does not kill an unknown process. A conflicting port produces an actionable error. Browser flags do not disable sandboxing or certificate checks.

See [HARDWARE_TEST.md](HARDWARE_TEST.md) for the on-site protocol.

\n