# BLOCKERS: подключение библиотеки к Safe School server platform

Статус на 17 сентября 2026: frontend и release-пакет подготовлены, но реальная выдача/приём не могут быть реализованы только настройками терминала. В согласованной рабочей области найден только frontend питания; server repository, OpenAPI и тестовый библиотечный контракт не предоставлены.

## Точный запрос к владельцу server platform

Нужен **репозиторий или API-доступ к существующей серверной платформе Safe School**, обслуживающей питание и турникеты, либо её versioned API documentation. Не нужен заранее готовый «backend библиотеки»: если library module отсутствует, его следует разработать в этом существующем серверном коде, с общими Student/Person/Card/School и единым auth/terminal identity.

Минимальный набор для старта:

1. server repository или OpenAPI с версией, включая shared Student/Person/Card/School, terminal auth, device enrolment и error envelope;
2. модели/миграции/transaction mechanism и способ расширить их library entities: title, copy, loan, operation receipt;
3. фактический card lookup contract: формат card ID, reader DTO и distinction между «не найден» и «нет права на питание»;
4. тестовый LAN origin и обезличенные readers, copies, active loans, включая copy и legacy title; разрешение выполнять тестовые записи;
5. contract для idempotent write и reconciliation: persistent `operation_id`, payload fingerprint, `applied/pending/not-found`, transaction boundaries;
6. CORS/cookie/CSRF/session policy и разрешённые browser origin для terminal;
7. модель NFC/card reader, scanner и принтера либо подтверждение keyboard-wedge input.

## Почему это блокирует подключение

| Блокер | Нельзя заменить настройкой | Безопасное текущие поведение |
| --- | --- | --- |
| Library routes и DTO | `runtime-config.js` задаёт только адрес; он не создаёт API, DTO или semantic mapping | `unavailableAdapter` не выполняет запись |
| Server auth и terminal rights | Нельзя помещать секрет в frontend и угадывать cookie/CORS/CSRF | нет скрытого bypass |
| Общие people/card data | Нельзя создавать вторую базу учеников или брать food eligibility как library identity | ручной поиск и карта в production не выдают ложный успех |
| Library data schema | Нужны FK/constraints/transactions между reader, copy и loan | mock seed не подменяет реальные данные |
| Idempotency | Клиентский UUID бесполезен без persistent server receipt | timeout/unknown не вызывает слепой retry |
| Test LAN / hardware | Невозможно доказать server write или card/scanner I/O по UI-счётчику | browser QA остаётся client-only |

## Уточнение по `canteen/get/student`

- Клиентский GET и food-log POST — разные запросы. Никаких тестовых запросов к школе не выполнялось.
- В `ScanComponent.vue` GET вызывается с `full_hex`, после чего клиент читает `response.data.data.full_name`, `data.iin`, `status` и `status_code`.
- POST `canteen/send/food/log` вызывается отдельно через `store.dispatch('registerMeal', …)` только в ветке `status_code === '001'`; он отправляет `{ iin, food_positions, meal_type_id }`.
- Клиентские сообщения `003`, `002`, `004` связаны с назначением/получением питания. Следовательно, этот ответ используется приложением питания для food eligibility, но исходники клиента не доказывают, что GET не может вернуть person data без права на питание.
- Server source отсутствует, поэтому побочные действия самого GET и точная семантика `status_code` не подтверждены. Endpoint не подключается к библиотеке без разрешённого server contract.

## Запрещённые обходы

- Не использовать `/api/{lang}/canteen/*`, `canteen/get/student` или food log как library API.
- Не возвращать mock/localStorage fallback в production.
- Не считать candidate `/library/*` routes существующими до подтверждения server owner.
- Не ретраить write после timeout/unknown без server-side reconciliation.
- Не менять running Nginx, питание или турникеты без отдельного разрешения.

## Что не блокирует frontend hardware UAT

Отсутствие library backend блокирует только настоящие школьные выдачи и приёмы. Оно не блокирует отдельный TERMINAL TEST: test package не делает запросов к школе, работает на loopback и использует только изолированные локальные данные. Физическую карту/NFC, scanner keyboard-wedge, touch, kiosk start и persistence надо подтвердить по [terminal-test/HARDWARE_TEST.md](terminal-test/HARDWARE_TEST.md); результаты не должны трактоваться как подтверждение backend-интеграции.
