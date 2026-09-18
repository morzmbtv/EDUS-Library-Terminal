#!/usr/bin/env sh
set -eu
APP_NAME="edus-library-terminal-test"
INSTALL_DIR="$HOME/.local/share/$APP_NAME"
PORT=43180
PYTHON=$(command -v python3 || true)
CHROME=$(command -v google-chrome-stable || command -v google-chrome || command -v chromium-browser || command -v chromium || true)
echo "EDUS terminal test diagnostics"
echo "Install: $INSTALL_DIR"
[ -n "$PYTHON" ] && echo "python3: $PYTHON" || echo "python3: MISSING"
[ -n "$CHROME" ] && echo "browser: $CHROME" || echo "browser: MISSING"
if [ -n "$PYTHON" ] && [ -f "$INSTALL_DIR/scripts/terminal_test_server.py" ]; then
  "$PYTHON" "$INSTALL_DIR/scripts/terminal_test_server.py" --check --port "$PORT" --version "$(cat "$INSTALL_DIR/VERSION")" && echo "loopback service: OK" || echo "loopback service: not running or wrong owner"
fi
[ -d "$INSTALL_DIR/app" ] && echo "static bundle: OK" || echo "static bundle: MISSING"
