#!/usr/bin/env sh
set -eu
APP_NAME="edus-library-terminal-test"
VERSION=$(cat "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)/VERSION")
INSTALL_DIR="$HOME/.local/share/$APP_NAME"
PORT=43180
URL="http://127.0.0.1:$PORT/"
PYTHON=$(command -v python3 || true)
CHROME=$(command -v google-chrome-stable || command -v google-chrome || command -v chromium-browser || command -v chromium || true)
SERVER="$INSTALL_DIR/scripts/terminal_test_server.py"
PID_FILE="$INSTALL_DIR/run/server.pid"
LOG_FILE="$INSTALL_DIR/run/server.log"
PROFILE="$HOME/.config/$APP_NAME/chrome-profile"
if [ -z "$PYTHON" ]; then echo "Не найден python3. Установка тестового режима невозможна." >&2; exit 3; fi
if [ -z "$CHROME" ]; then echo "Не найден Google Chrome или Chromium. Установка тестового режима невозможна." >&2; exit 4; fi
if [ ! -f "$SERVER" ] || [ ! -d "$INSTALL_DIR/app" ]; then echo "Тестовый пакет не установлен: $INSTALL_DIR" >&2; exit 5; fi
set +e
"$PYTHON" "$SERVER" --check --port "$PORT" --version "$VERSION"
STATUS=$?
set -e
if [ "$STATUS" -eq 11 ]; then echo "Порт $PORT уже занят другим сервисом. Не изменяйте его автоматически; освободите порт или обратитесь к администратору." >&2; exit 6; fi
if [ "$STATUS" -eq 10 ]; then
  if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then echo "Предыдущий тестовый сервис не отвечает, но его процесс ещё работает. Он не будет остановлен автоматически." >&2; exit 7; fi
  mkdir -p "$INSTALL_DIR/run" "$PROFILE"
  nohup "$PYTHON" "$SERVER" --root "$INSTALL_DIR/app" --port "$PORT" --version "$VERSION" > "$LOG_FILE" 2>&1 &
  echo $! > "$PID_FILE"
  tries=0
  until "$PYTHON" "$SERVER" --check --port "$PORT" --version "$VERSION" >/dev/null 2>&1; do tries=$((tries + 1)); [ "$tries" -lt 20 ] || { echo "Тестовый сервис не запустился. См. $LOG_FILE" >&2; exit 8; }; sleep 0.15; done
fi
exec "$CHROME" --kiosk --no-first-run --no-default-browser-check --user-data-dir="$PROFILE" "$URL"
