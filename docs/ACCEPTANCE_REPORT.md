# Отчёт приёмки и план верификации

## Выполнено локально на Windows, 17 сентября 2026

| Тип проверки | Результат | Доказательство и предел |
| --- | --- | --- |
| Mock/domain test | 55/55 PASS | `npm test`; проверяет reader-first return, wrong reader, ISBN ownership, partial legacy, double-submit, unknown result. Не обращается к серверу. |
| Client boundary test | PASS | Проверяет candidate `LibraryApiClient`: terminal context headers, body `operation_id`, error mapping и пустую runtime config. Fake transport, не настоящий API. |
| Static quality | PASS | `npm run typecheck`, `npm run lint`, `npm run build`. |
| Production boundary | PASS | `npm run qa:production-boundary`; 1280×800 Chromium, нет demo controls и при пустом config операция блокируется. Это не server write. |
| Release composition | PASS | release app не содержит demo seed; ZIP содержит static app, Nginx config, Ubuntu scripts и документы. |
| Canteen/source analysis | PASS | frontend питания разобран локально; серверный source/API не найден. Сетевых запросов к школе не было. |

## Не выполнено: интеграционный и аппаратный тест

Не существует разрешённого test backend в доступных материалах. Поэтому ни один из следующих пунктов не отмечен как выполненный:

| Сценарий реального test server | Доказательство, которое требуется | Статус |
| --- | --- | --- |
| Card identification | server response с обезличенным reader ID; отсутствие food-log write | Ожидает server contract + test LAN |
| Active loans | независимое GET/DB view с loan IDs выбранного reader | Ожидает backend/data |
| Issue конкретного copy | server receipt и независимое повторное чтение loan | Ожидает backend/data |
| Reload browser | созданный server loan остаётся после нового browser session | Ожидает backend/data |
| Return copy | server закрывает нужный loan, другой reader не меняется | Ожидает backend/data |
| Partial legacy return | server сохраняет корректный remaining quantity | Ожидает backend/data |
| Idempotent repeat | одинаковый `operation_id` возвращает один receipt, не второй loan | Ожидает backend/data |
| Loss after write | `GET operations/{id}` подтверждает applied/pending without blind retry | Ожидает backend/data |
| Ubuntu kiosk | Chrome, 1280×800 touch, restart, LAN failure/recovery | Ожидает совместимый Ubuntu terminal |
| Hardware | card reader, scanner, printer and disconnect recovery | Ожидает физическое оборудование |

## Критерий реальной приёмки

Реальный issue/return считается проверенным только при наличии записи и последующего независимого чтения на разрешённом test backend. Изменение UI-счётчика или mock test не является таким доказательством. Перед production нужны server-side transaction/idempotency, назначенный владелец deployment и журнал результатов без ПДн.

См. [PROJECT_STATE.md](../PROJECT_STATE.md), [BLOCKERS.md](../BLOCKERS.md) и [карту интеграции питания](CANTEEN_INTEGRATION_MAP.md).
