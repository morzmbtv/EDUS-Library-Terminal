# Карта интеграции с поставкой `canteen-front-main`

Дата анализа: 17 сентября 2026. Исходный архив распакован только для чтения в `.reference/canteen-front-inspected-20260917/`; ни один его файл не подключён к библиотечному терминалу.

## Вывод

Поставка подтверждает только клиентскую схему фронтенда модуля питания: Vue/Vite, Axios, относительная локальная сеть и Docker-образ Nginx. Она **не подтверждает** наличие библиотечного backend, его URL, DTO, авторизацию, CORS, считыватели или схему развёртывания на терминале. Клиентский flow карты в питании после успешного GET отдельно создаёт запись питания. Это не доказывает побочный эффект самого GET, но endpoint остаётся неподтверждённым для библиотеки и не переиспользуется без server contract.

## Значимые вызовы

| Файл и участок | Запрос и вход | Поля, коды, эффект | Категория | Статус доказательства |
| --- | --- | --- | --- | --- |
| `src/axios.js` | base URL `http://192.168.1.10/api/{ru}/`, `Content-Type: application/json`, timeout 100000 ms | язык из `localStorage.currentLanguage`; token/interceptors закомментированы | Паттерн конфигурации можно адаптировать, IP и auth нельзя копировать | Подтверждён только в canteen frontend |
| `src/components/ScanComponent.vue` | `GET canteen/get/student?full_hex=<card>` | читает `data.full_name`, `data.iin`, `status`, `status_code`; сам клиент не делает POST внутри GET | Canteen-specific card lookup; person data и food eligibility смешаны в UI | GET side effect не подтверждён без server source; не переносится без contract |
| `src/components/CartCheckComponent.vue` | `GET canteen/get/student?full_hex=<card>` | обрабатывает `004`, `003`, `002` | Canteen-only бизнес-коды | Подтверждены только для питания |
| `src/store/index.js` | `GET canteen/count?date=…` | `data.total`, `data.eat`, `data.not_eat` | Canteen-only | Подтверждено в frontend, не библиотека |
| `src/store/index.js` | `GET canteen/free/menu?date=…` | `data` меню | Canteen-only | Подтверждено в frontend, не библиотека |
| `src/store/index.js` | `GET canteen/list` | список операций питания | Canteen-only | Подтверждено в frontend, не библиотека |
| `src/store/index.js` | `POST canteen/send/food/log` body `{ iin, food_positions, meal_type_id }` | создаёт запись выдачи питания; вызывается `ScanComponent` после client-side `status_code === "001"` | Canteen-only, необратимое действие | Подтверждено в frontend |
| `src/views/nutriation/PaidNutriation.vue` | `GET canteen/paid/menu?date=…`; `GET canteen/paid/get/student?full_hex=…` | платное меню и читатель питания | Canteen-only | Подтверждено в frontend |
| `src/views/nutriation/PaidNutriation.vue` | `POST canteen/paid/send/food/log` body `{ iin, food_positions, price }` | создаёт платную операцию | Canteen-only, необратимое действие | Подтверждено в frontend |
| `src/store/index.js` | `GET license/valid` | `data.license` | Возможна общая серверная проверка, но назначение и контракт не подтверждены | Требует backend-подтверждения |
| `db.js` | IndexedDB `apiCache/responses` | кэш ответов; в активных маршрутах не используется как надёжный offline-контур | Не переносить как источник библиотечных операций | Подтверждено как неиспользуемая заготовка |
| `public/serial_port.js` | нет активного вызова | весь файл закомментирован | Нельзя считать интеграцией оборудования | Отсутствует доказательство |
| `Dockerfile` | Nginx Alpine, статика на 8081 | копирует `dist`, меняет порт Nginx | Повторно используемая идея серверной раздачи статики | Подтверждён статический frontend, не backend/topology |
| `.gitlab-ci.yml` | `npm install`, `npm run build`, Docker push | registry `gitlab.srv.intronet.kz:5050/smart/canteen-front` | Исторический CI пример | Не использовать без доступа и подтверждения |

## Переиспользование и границы

| Потребность библиотеки | Решение | Основание |
| --- | --- | --- |
| Runtime-настройки без пересборки | новый `public/runtime-config.js`, валидация в `src/infrastructure/runtimeConfig.ts` | адаптирована идея LAN-конфигурации, без жёсткого IP |
| HTTP boundary | новый `src/infrastructure/libraryApi.ts` с `/api/{lang}/library/*` только как кандидатным контрактом | спецификация v1.2, но endpoint не подтверждён сервером |
| Карта ученика | библиотечный экран вызывает только библиотечный адаптер | исключает регистрацию питания |
| Идемпотентность | `operation_id` в кандидатных write body и endpoint проверки операции | пользовательские требования и существующий demo-domain |
| Статическая поставка | Nginx Dockerfile в release package + Ubuntu launcher | повторяет только подтверждённый способ раздачи статики |

## План изменений

| Область | Действие |
| --- | --- |
| Canteen source | Ничего не копировать и не модифицировать |
| EDUS library frontend | Изолировать runtime config и API boundary; продолжать использовать свой Vue/TypeScript стек |
| Реальный library adapter | Создать после передачи library backend source/contract и тестовой LAN-среды |
| Auth, cookies, CORS, CSRF | Проверить в Chromium против реального origin; до этого не предполагаются |
| NFC/scanner | Подключить отдельный adapter после подтверждения модели/протокола устройства; keyboard-wedge остаётся безопасным fallback |

## Что требуется от владельца платформы

Нужны фактические library API routes и версии, примеры успешных/ошибочных ответов, способ аутентификации терминала, разрешённые origin/cookie/CORS/CSRF правила, стабильные идентификаторы школы/устройства, способ выдачи card ID и тестовая школа с обезличенными записями. Без них нельзя заменять блокирующий production adapter реальными запросами.

## Уточнение: GET карты и запись питания — разные действия

`ScanComponent.vue` вызывает `GET canteen/get/student` в `processCardData()` и сохраняет ответ в `studentData`; имя считывается из `studentData.data.full_name`. Затем отдельная ветка `if (studentData.status_code === "001")` формирует food log и выполняет `store.dispatch("registerMeal", ...)`; именно store отправляет `POST canteen/send/food/log`. Для `003`, `002` и `004` UI показывает food-specific сообщения и POST не вызывается.

Это позволяет отделить **клиентское получение ответа GET** от **клиентской регистрации питания**. Однако серверный source отсутствует: нельзя подтвердить, что GET сам не имеет side effect, нельзя доказать, возвращается ли `data.full_name` без права на питание, и нельзя назначать `status_code` универсальной семантикой личности. Ни одна операция к server школы для этого анализа не выполнялась.
