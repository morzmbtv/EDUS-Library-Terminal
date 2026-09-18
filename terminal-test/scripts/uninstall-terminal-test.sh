#!/usr/bin/env sh
set -eu
APP_NAME="edus-library-terminal-test"
INSTALL_DIR="$HOME/.local/share/$APP_NAME"
rm -rf "$INSTALL_DIR"
rm -f "$HOME/.local/share/applications/$APP_NAME.desktop"
echo "Тестовый пакет удалён. Отдельный профиль Chrome сохранён в $HOME/.config/$APP_NAME/chrome-profile."
