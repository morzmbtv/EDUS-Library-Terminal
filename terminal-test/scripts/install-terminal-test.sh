#!/usr/bin/env sh
set -eu
APP_NAME="edus-library-terminal-test"
INSTALL_DIR="$HOME/.local/share/$APP_NAME"
DESKTOP_DIR="$HOME/.local/share/applications"
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
find_chrome() { command -v google-chrome-stable || command -v google-chrome || command -v chromium-browser || command -v chromium || true; }
if ! command -v python3 >/dev/null 2>&1; then echo "Не найден python3. Установите Python 3 и повторите установку." >&2; exit 3; fi
if [ -z "$(find_chrome)" ]; then echo "Не найден Google Chrome или Chromium. Установите браузер и повторите установку." >&2; exit 4; fi
mkdir -p "$HOME/.local/share" "$DESKTOP_DIR"
rm -rf "$INSTALL_DIR"
mkdir -p "$INSTALL_DIR"
cp -a "$SCRIPT_DIR/app" "$SCRIPT_DIR/scripts" "$SCRIPT_DIR/icons" "$SCRIPT_DIR/HARDWARE_TEST.md" "$SCRIPT_DIR/README.md" "$SCRIPT_DIR/VERSION" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/scripts/"*.sh "$INSTALL_DIR/scripts/terminal_test_server.py"
sed "s|__INSTALL_DIR__|$INSTALL_DIR|g" "$SCRIPT_DIR/edus-library-terminal-test.desktop.template" > "$DESKTOP_DIR/edus-library-terminal-test.desktop"
chmod 644 "$DESKTOP_DIR/edus-library-terminal-test.desktop"
echo "Установлен тестовый терминал: $INSTALL_DIR"
echo "Запуск: $INSTALL_DIR/scripts/launch-terminal-test.sh"
