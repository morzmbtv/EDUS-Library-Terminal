# Кандидатный контракт Library API

Этот документ описывает **проектируемую границу frontend**, а не уже обнаруженный серверный контракт. Источник маршрутов — приложенная `SafeSchool_Library_Terminal_SPEC_v1.2.md`; backend-исходники или подтверждённый API библиотеки не предоставлены. Нельзя направлять production-терминал на эти URL до письменного/API-подтверждения владельца backend.

## Общие правила

- Базовый origin, `schoolId`, `deviceId`, язык и timeout задаются рядом со статикой в `runtime-config.js`; в bundle нет IP, токена или секрета.
- Предлагаемый путь: `{apiBaseUrl}{apiPrefix}/{language}/library/{route}`. По умолчанию `/api/ru/library`.
- Каждый запрос должен передавать `Accept: application/json`, `X-Edus-School-Id`, `X-Edus-Device-Id`, `X-Edus-Terminal-Type: LIBRARY`; реальные названия заголовков требуют подтверждения.
- Write-операции получают UUID `operation_id` в body. Повтор того же UUID и того же payload обязан вернуть прежний результат, другой payload — `OPERATION_CONFLICT`.
- Транспорт использует `credentials: include`, но не предполагает, что cookie/auth уже работает. Реальные CORS/SameSite/CSRF правила — blocker.
- Клиент ограничивает read 8 сек., write 15 сек., reconciliation 8 сек. и не повторяет write после неопределённого результата автоматически.

## Кандидатные методы

| Цель | Метод и путь | Минимальные входы | Нужный результат |
| --- | --- | --- | --- |
| Доступность | `GET health` | контекст терминала | состояние library API, версия, разрешение устройства |
| Карта читателя | `GET readers/by-card/{card}` | код карты | reader id, ФИО, роль/класс, безопасные display-поля |
| Ручной поиск | `GET readers/search?q=` | строка | ограниченный список readers |
| Скан | `POST scan/resolve` | `{ code }` | конкретный copy или edition/ambiguous; без выбора владельца по ISBN |
| Выдача | `POST loans/issue` | `{ operation_id, reader_id, items }` | operation id, copy/loan IDs, итог |
| Приём | `POST loans/return` | `{ operation_id, reader_id, items }` | operation id, закрытые/частичные loan IDs, итог |
| Неопределённый результат | `GET operations/{operation_id}` | UUID | applied/pending/not found + result |

## Критические доменные инварианты

1. Приём всегда начинается с читателя. `scan/resolve` не имеет права автоматически менять выбранного читателя.
2. Individual copy сопоставляется с активной выдачей выбранного читателя. Copy другого читателя возвращает понятную ошибку без side effect.
3. ISBN обозначает издание; для старого фонда backend возвращает выдачи **выбранного** читателя и требует явный count/loan id. Никакого случайного владельца.
4. Для черновика возврата server state не меняется до `POST loans/return`; у frontend отдельно активные loans и selected return items.
5. Ошибочные/неоднозначные scans не очищают существующий черновик.
6. `OPERATION_PENDING` и неизвестный результат требуют `GET operations/{operation_id}`, а не слепой повтор POST.

## Нормализация ошибок для UI

Предполагаемые коды: `READER_NOT_FOUND`, `BOOK_NOT_FOUND`, `SCAN_UNKNOWN`, `SCAN_AMBIGUOUS`, `BOOK_ALREADY_ON_LOAN`, `BOOK_NOT_ON_LOAN`, `LEGACY_READER_REQUIRED`, `LEGACY_LOAN_NOT_FOUND`, `IDENTIFIER_CONFLICT`, `VALIDATION_ERROR`, `FORBIDDEN`, `SESSION_EXPIRED`, `LOCAL_SERVICE_UNAVAILABLE`, `OPERATION_PENDING`, `OPERATION_ALREADY_APPLIED`.

Их существование и форма envelope пока не доказаны. `LibraryApiClient` сохраняет коды как строго изолированный кандидатный слой и в текущий terminal adapter не подключён.
