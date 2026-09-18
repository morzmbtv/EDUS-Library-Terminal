# Реальное состояние проекта — 17 сентября 2026

## Краткий вывод

Рабочая библиотека с server-side сохранением **ещё не подключена**. В доступной согласованной рабочей области нет repository серверной платформы Safe School, OpenAPI/Swagger, library API contract или разрешённого тестового origin. Есть только frontend модуля питания. Поэтому production защищённо останавливает библиотечную операцию; эта защита остаётся включённой до реальной интеграции.

## Как production выбирает адаптер

- Vite compile-time constant `__EDUS_DEMO_RUNTIME__` равен `true` только вне production build. В dev он загружает `createMockAdapter` для изолированных UI/test сценариев.
- В production `src/composables/useTerminal.ts` создаёт `createUnavailableAdapter(...)` из `src/infrastructure/unavailableAdapter.ts`.
- `unavailableAdapter` возвращает пустой snapshot с `connection.server: false`; поиск не выдаёт данные, а идентификация, выдача, приём, регистрация и проверка операции завершаются `DomainError('SERVER_OFFLINE', ...)`. Он не пишет в `localStorage`, не делает HTTP-запросов и не создаёт учебных операций.
- Production bundle проверен: в `dist/assets` и release `app/assets` отсутствуют `EDUS-1001` и `edus-library-demo-v1`.

## Почему `libraryApi.ts` не подключён

`src/infrastructure/libraryApi.ts` — изолированный **кандидатный** HTTP client, основанный на приложенном SPEC v1.2, а не найденный контракт сервера. Показанные в нём `/api/{lang}/library/*` — предполагаемые пути и не должны считаться существующими маршрутами.

Для подключения недостаточно `runtime-config.js`. Нужна реализация или подтверждённый контракт серверной части, потому что frontend должен знать:

1. фактические routes, HTTP methods, версии и JSON DTO для card lookup, reader search, active loans, scan resolution, issue, return, registration и operation lookup;
2. механизм terminal authentication/enrolment, cookie/header, CORS, CSRF и права устройства;
3. stable IDs reader, title, copy и loan, а также правила COPY и legacy-фонда;
4. envelope, business errors, timeout/unknown semantics и реальные правила reconciliation;
5. server-side транзакции и idempotency. Сам факт передачи `operation_id` клиентом ничего не доказывает: сервер обязан сохранять ключ, сравнивать payload и возвращать прежний результат на повторе;
6. миграции/ограничения библиотечных данных в общей Safe School платформе.

## Что найдено в предоставленных проектах

- `canteen-front-main` — Vue frontend. Он ссылается на `http://192.168.1.10/api/{lang}/`, но не содержит API implementation, модели, миграции, controllers, OpenAPI или server runtime.
- В согласованной рабочей области не найден код Student/Person/Card/School, auth server-side, terminal identity, turnstile service, операция записи или синхронизация Safe School.
- Имена файлов и исходники `canteen-front-main` подтверждают Docker/Nginx-раздачу его статики на порту 8081, но не подтверждают расположение или архитектуру backend.

## Независимо завершённая подготовка

- Vue 3/Vite/TypeScript frontend, локальные EDUS assets и встроенные шрифты; внешние runtime CDN не используются.
- Экранные потоки выдачи, приёма, reader-first return, partial legacy return, unknown operation и registration проверены в демонстрационном domain adapter.
- 10,1″ 1280×800 POS-композиция, контекстная помощь, line-art карта и экран приёма с активными loans проверялись в Chromium на Windows.
- Добавлены безопасная runtime-конфигурация, candidate client, release package и Ubuntu launcher; они не являются реализацией backend.

См. [BLOCKERS.md](BLOCKERS.md), [карту интеграции питания](docs/CANTEEN_INTEGRATION_MAP.md), [кандидатную API-границу](docs/LIBRARY_API_CONTRACT.md) и [план приёмки](docs/ACCEPTANCE_REPORT.md).

## TERMINAL TEST / hardware UAT

Frontend готов к отдельной локальной проверке терминала без Safe School backend: `npm run build:terminal-test`, затем `npm run package:terminal-test`. Пакет запускает только статическую test-сборку на loopback с отдельным Chrome profile; он не использует canteen routes и не меняет production release. Карта и scanner проверяются через явный capture в закрытой test panel, а его mappings/operations переживают restart браузера в test-only localStorage namespace. Физические NFC/scanner/touch acceptance ещё должны быть выполнены на терминале по [terminal-test/HARDWARE_TEST.md](terminal-test/HARDWARE_TEST.md).
