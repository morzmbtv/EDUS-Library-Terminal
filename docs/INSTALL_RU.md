# Установка на Ubuntu — EDUS Library Terminal

## До начала

Нужны: Ubuntu с графической сессией, Chromium/Google Chrome, доступ к school-server URL, распакованный release archive и утверждённый `EDUS_LIBRARY_URL`. Реальная библиотечная операция остаётся заблокированной до подключения подтверждённого API; это не устраняется этой установкой.

## Установка

1. Распакуйте `edus-library-terminal-1.0.0.zip` в локальный каталог пользователя.
2. На school server заполните `config/runtime-config.js` и разверните статический image по [DEPLOYMENT.md](DEPLOYMENT.md).
3. На терминале выполните:

```bash
cd edus-library-terminal
sh scripts/install.sh
mkdir -p "$HOME/.config/edus-library-terminal"
printf '%s\n' 'EDUS_LIBRARY_URL=https://library-terminal.school.local/' > "$HOME/.config/edus-library-terminal/config.env"
chmod 600 "$HOME/.config/edus-library-terminal/config.env"
$HOME/.local/share/edus-library-terminal/scripts/diagnose.sh
```

4. Запустите ярлык **EDUS Library Terminal** из меню приложений или командой:

```bash
$HOME/.local/share/edus-library-terminal/scripts/launch-library.sh
```

Скрипт создаёт отдельный пользовательский профиль Chrome и не отключает browser security, sandbox или certificate verification.

## Обновление

Распакуйте новый release, не удаляя `$HOME/.config/edus-library-terminal/config.env`, затем:

```bash
cd edus-library-terminal
./scripts/update.sh
```

Проверьте `diagnose.sh`, запустите терминал и выполните UAT на тестовых данных. Runtime URL и server config сохраняются отдельно от release.

## Удаление

```bash
$HOME/.local/share/edus-library-terminal/scripts/uninstall.sh
```

Удаляются только файлы launcher-а и desktop entry текущего пользователя. Конфигурация с URL сохраняется, чтобы удаление не стирало настройки без явного решения оператора.

## Проверка на реальном терминале

До приёмки проверьте 1280×800, touch, fullscreen/kiosk, перезагрузку, потерю/возврат LAN, reader card, scan, отмену и повтор записи, печать (если подключена), права оператора и диагностику. Полный перечень в [ACCEPTANCE_REPORT.md](ACCEPTANCE_REPORT.md).

\n