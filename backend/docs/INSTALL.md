# Установка EDUS из одного ZIP

Текущая поставка: `EDUS-Library-Portable-2.0.0-rc1.zip`. Распакуйте целиком папку EDUS-Library. Откройте Windows PowerShell x64 от администратора и из этой папки выполните:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\Setup-EDUS.ps1
.\scripts\Verify-EDUS.ps1
```

Один Setup устанавливает service/backend, production frontend и native Configurator. Отдельные Backend/Frontend/Configurator installers больше не используются. Исходные проекты при этом остаются независимыми. Подробная действующая инструкция входит в ZIP: docs/QUICK_START_RU.md.

После Setup откройте единственный ярлык EDUS Terminal Configurator в меню Пуск и настройте UAT без Cloud либо Production enrollment. Проверьте http://127.0.0.1:43180. Assigned Access настраивается отдельно средствами Windows; Setup не создаёт kiosk-аккаунты и не меняет политики ОС.

Service name EDUSLibraryService, LocalService identity, ProgramData и DPAPI user scope сохраняются. Не копируйте старую per-user Tauri DB или plaintext keys вручную. Repair не удаляет базы/secrets. Remove сохраняет ProgramData, если явно не указан подтверждённый RemoveData.

Microsoft Edge должен быть установлен заранее. Все EDUS runtime-файлы находятся в ZIP; интернет для UAT не требуется. Возможность Windows Assigned Access зависит от ОС физического терминала и требует отдельной проверки.
