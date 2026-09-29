# BeautyApp — аудит backend `assistant` и план переделки под платформу

Дата: 2026-09-29 · Статус: аудит только чтением, в `~/Desktop/assistant` ничего не менялось, кода BeautyApp нет.
Основа для требований: `PRODUCT_SPEC.md` (разделы 10–15, 17, 18, 21).

---

## 1. Коротко

- `assistant` — рабочий **Rust-монолит** (Axum 0.7, sqlx 0.7, PostgreSQL/Neon, Koyeb, Cloudflare R2): ~117 000 строк, 310 файлов, 157 миграций, 98 таблиц. Он обслуживает **несколько несвязанных продуктов**: ChefOS (кухня/рецепты/склад, iOS), «Свет Иконы» (церковный контент, заказы икон, Telegram-бот), AlmaBuild (стройка), czystetrojmiasto (уборка), SEO-страницы, AI-копилоты (Gemini/Groq).
- Технический уровень хороший: слои DDD, value objects, argon2id, JWT, rate limit, i18n на 4 языка (pl/en/uk/ru — ровно наши), 486 тестов, интеграционные тесты на `sqlx::test`.
- Но **как основа BeautyApp он не подходит «как есть»**: доменная модель другая (ресторан/церковь), тенантность неполная (нет RLS, один пользователь = один тенант, роли не те), нет ключевых для нас вещей (exclusion constraint на слоты, Connect-платежи, outbox уведомлений, согласия GDPR), а в той же базе живут боевые данные других сайтов.
- **Рекомендация:** не «переделывать» `assistant` на месте, а создать **новый backend-репозиторий** (Rust/Axum/sqlx/Postgres — тот же стек) и **перенести в него выверенные кусочки инфраструктуры** (конфиг, ошибки, JWT/argon2, R2, проверка Stripe-подписи, rate limit, Docker/Koyeb, паттерн тестов). Доменную часть и миграции писать заново под `PRODUCT_SPEC.md`.

---

## 2. Что проверено и что нет

**Проверено (чтение):** `Cargo.toml`, README/ARCHITECTURE/SECURITY, `Dockerfile`, `koyeb.yaml`, `Makefile`, структура `src/` (все файлы и размеры), `main.rs`, `tenant.rs`, `user.rs`, `auth.rs`, `jwt.rs`, `middleware.rs`, `site_context.rs`, `stripe_service.rs` (сигнатуры), миграция `initial_schema` и `sites_and_admin_site_scope`, `tests/`, git-состояние, секреты в репозитории (значения маскировались).

**Не проверено:** сборка и запуск (`cargo build/test` не запускал), проверка прав в каждом HTTP-хендлере, реальный регион Neon/Koyeb, содержимое локального `.env` (специально не открывал), нагрузка. Выводы «в хендлерах нет RBAC» я не делаю — только «централизованного RBAC не нашёл».

---

## 3. Карта репозитория

| Область | Что там | Для BeautyApp |
|---|---|---|
| `domain/` | tenant, user, auth (refresh token), assistant wizard, catalog ингредиентов, inventory, recipe, dish, menu_engineering, engines/tools (nutrition, flavor, unit converter…) | почти всё — не наше |
| `application/` | 100+ файлов: rulebot (чат-бот кулинарии), ai_sous_chef, lab_combos, laboratory, copilot, intent_pages (pSEO), analytics, cms_service, prayer_visualizer, usage_service… | почти всё — не наше |
| `infrastructure/` | config, jwt, password, r2_client, local_storage, stripe_service, cache (moka), gemini/groq клиенты, repositories | **основной источник переиспользования** |
| `interfaces/http/` | `routes.rs` (2178 строк, ~400 маршрутов), church_content (3147), church_orders, icons_site, almabuild, admin_*, public/tools, telegram/ | инфраструктурные части (error, middleware, health) |
| `shared/` | error, result, types (ID), pagination, language, i18n | **переиспользуется** |
| `bin/` | `admin_tool` (тяжёлые задачи локально), `generate_admin_hash` | паттерн для `worker`/`admin` |
| `migrations/` | 157 файлов, 98 `CREATE TABLE`, 28 файлов с seed-данными, цепочки `fix_*/cleanup_*/backfill_*` | **не переносить**, писать чистую схему |
| `tests/` | `site_isolation.rs` (`#[sqlx::test]`), `telegram_webhook.rs` | паттерн сохранить |

---

## 4. Аудит: находки

Уровни: **H** — блокирует использование как основы, **M** — нужно исправить при переносе, **L** — мелочь, **+** — сильная сторона.

### 4.1. Архитектура и домен
| # | Ур. | Находка | Следствие для нас |
|---|---|---|---|
| A1 | H | Один процесс обслуживает 4+ несвязанных продукта; домен — кухня/церковь/стройка. ~117k строк, `routes.rs` 2178 строк. | Прямая «переделка» = удалить >90% кода. Проще и безопаснее начать с чистого репозитория. |
| A2 | H | В продовой базе живут данные других сайтов (ChefOS iOS, Свет Иконы, заказы, лиды). | Любая миграция под BeautyApp рискует боевыми данными; смешивать данные о здоровье клиентов (GDPR, Art. 9) с CMS/SEO — недопустимо. |
| A3 | M | Многосайтовость сделана через `sites` + три захардкоженных UUID в коде (`site_context.rs`) и колонку `site_id` в отдельных таблицах. Это скоупинг контента, а не полноценная тенантность. | Не переносить; тенантность строить заново (раздел 6). |
| A4 | + | Чёткие слои domain/application/infrastructure/interfaces, value objects (`Email`, `Password`, `TenantName`), типизированные ID. | Стиль переносим как есть. |
| A5 | L | Документация устарела: README ссылается на `docs/` (нет), ARCHITECTURE называет Rust 1.83 и 9 миграций, Dockerfile на `rust:latest`. | В новом репозитории — актуальные docs и закреплённая версия toolchain. |

### 4.2. Тенантность, аутентификация, роли
| # | Ур. | Находка | Следствие |
|---|---|---|---|
| B1 | H | `users.email` **глобально уникален**, у пользователя ровно один `tenant_id` (он же в JWT). | Нельзя быть клиентом у одного мастера и сотрудником у другого; спека требует `user` ↔ несколько `membership`. |
| B2 | H | Роли: `owner / manager / staff`. Нет `customer`, нет `reception`, нет permissions. Централизованной проверки прав не нашёл (есть `is_owner()`). | Нужна модель `membership(role, permissions)` и серверный RBAC-экстрактор (спека 17). |
| B3 | H | **Нет RLS** в PostgreSQL (`ENABLE ROW LEVEL SECURITY`/`CREATE POLICY` не встречаются). Изоляция — ручным `WHERE tenant_id = …` (≈150 упоминаний в репозиториях). | Одна забытая проверка = утечка данных клиента. Для нас — обязательный второй рубеж (RLS). |
| B4 | M | Refresh-токен **не ротируется**: `refresh()` возвращает тот же токен; нет привязки к устройству и обнаружения повторного использования. | Нужны ротация, `device_id`, отзыв по устройству (спека 9.1). |
| B5 | M | JWT: HS256 с одним общим секретом, без `aud`, `kid`, `jti`, без ролей/членств в claims; `AuthUser` делает запрос в БД на **каждый** запрос (язык пользователя). | Для нескольких клиентов (iOS/Android/web) — asymmetrical или хотя бы `kid` + ротация ключей; язык брать из claims/кэша. |
| B6 | M | Вход только email+пароль (argon2id). Нет OTP, Sign in with Apple, Google. | Спека: passwordless + Apple обязателен для App Store. Argon2 остаётся для админки. |
| B7 | + | Пароли — argon2id, refresh хранится хэшем (SHA-256), rate limit через `governor`, admin-JWT отдельным секретом. | Переносим. |

### 4.3. Данные и миграции
| # | Ур. | Находка | Следствие |
|---|---|---|---|
| C1 | H | Нет `btree_gist` и exclusion constraint (совпадения в `migrations` — это `EXCLUDED` в UPSERT). Календаря/записей нет вообще. | Ядро продукта (спека 12.2) пишем с нуля. |
| C2 | M | 157 миграций с `fix_*`, `cleanup_*`, `backfill_*`, seed-данные (ингредиенты, рыба, церковный календарь) внутри миграций; нет down-миграций. | Не переносить историю; в новом репозитории — чистая начальная схема + линейные миграции без seed-данных бизнеса. |
| C3 | M | Миграции применяются **при старте** (`sqlx::migrate!` в `main.rs`). | При нескольких инстансах и зонах даунтайма — гонки; вынести в отдельный шаг деплоя/job. |
| C4 | M | 673 runtime-запроса `sqlx::query*`, макросов `query!` и `.sqlx/` нет. | Нет проверки SQL на этапе компиляции; для календаря и денег — включить `query!` + `cargo sqlx prepare` в CI. |
| C5 | M | Деньги/время: используются `rust_decimal`, `time` и `chrono` одновременно. | В новом проекте — деньги в minor units (i64) + `currency`, время одним крейтом (`time`), UUIDv7. |
| C6 | + | Есть паттерн идемпотентности (`stripe_idempotency`, `idempotency_and_limits`) и `#[sqlx::test(migrations = …)]`. | Переносим как основу тестов и идемпотентных ключей. |

### 4.4. Платежи
| # | Ур. | Находка | Следствие |
|---|---|---|---|
| D1 | M | Stripe используется для **предоплаченных пакетов AI-действий** (Checkout Session, `STRIPE_PRICE_ACTIONS_*`). Не Connect, не PaymentIntent, не возвраты. | Модель платежей BeautyApp (спека 15) — новая: Connect + PaymentIntent + webhook как источник истины. |
| D2 | + | Проверка подписи webhook: HMAC, сравнение за константное время (`subtle`), окно от replay 5 минут. | Переносим целиком. |

### 4.5. Эксплуатация и безопасность
| # | Ур. | Находка | Следствие |
|---|---|---|---|
| E1 | + | Секреты: `.env` в `.gitignore`, в истории git не найден; в трекинге только `.env.example` и маскированные примеры; `koyeb.yaml` перечисляет секреты как «вносить через UI». | Ок. Локальный `.env` существует (создан июль) — содержимое не читал; убедиться, что ключи после ротации. |
| E2 | M | Koyeb: `min 1 / max 1` инстанс, тяжёлые задачи вынесены в `admin_tool` (хороший приём). Регион Neon/Koyeb неизвестен. | Для GDPR — регион ЕС и DPA с провайдерами; для нас нужен отдельный `worker`. |
| E3 | M | CORS-список включает `localhost` и личные сайты; `Dockerfile` — `rust:latest`, запуск от root, без cargo-chef. | Отдельные окружения (dev/staging/prod), закреплённый toolchain, non-root, кэширование сборки. |
| E4 | L | `unwrap()`: 180, `expect()`: 19 (включая тесты внутри `src`); `panic!`/`unsafe`: 0; TODO/FIXME: 3. | При переносе не тащить; в новом коде — `clippy::unwrap_used` deny вне тестов. |
| E5 | + | 486 тестовых функций, JSON-логи `tracing`, `/health`, кэш `moka`, R2 с presigned upload. | Переносим подходы. |

### 4.6. Чего в `assistant` нет, но нужно BeautyApp (из спеки)
Записи/календарь/слоты, holds и защита от double booking, каталог услуг beauty, клиенты + lash-профиль, согласия и аудит по GDPR, экспорт/удаление данных клиента, outbox уведомлений (APNs/SMS/email), Stripe Connect, отзывы, портфолио, отчёты по доходу, OpenAPI-контракт, роли customer/employee/reception/manager.

---

## 5. Варианты и рекомендация

| Вариант | Суть | Плюсы | Минусы |
|---|---|---|---|
| **A. Новый репозиторий + перенос инфраструктуры (рекомендуется)** | `beauty-backend` (Rust/Axum/sqlx), копируем/адаптируем модули из раздела 7 | Чистая схема и домен, нет риска для боевых сайтов, нет чужих данных рядом с данными о здоровье, быстрая сборка, понятные границы GDPR | Нужно заново поднять каркас (1–2 недели) |
| B. Переделка `assistant` на месте | Добавить «beauty» как ещё один «сайт» | Быстрый старт, общий деплой | Смешение данных (GDPR), общий пул/секреты, риск для прод-сайтов, 117k лишних строк, кривая тенантность (A3, B1) |
| C. Общий `platform-core` + отдельные продукты | Вынести общее ядро в крейт и переиспользовать во всех продуктах | Долгосрочно красиво | Дорого сейчас: рефакторинг боевого кода ради будущей выгоды |

**Рекомендую A.** Позже, если понадобится, общие куски можно вынести в C, не трогая боевой `assistant`.

Стек нового backend: Rust 1.9x (закрепить), Axum, sqlx (макросы `query!`), PostgreSQL 16+ (EU-регион), `btree_gist`, RLS, `utoipa` для OpenAPI 3.1, `tracing`, worker-бинарь для фоновых задач.

---

## 6. Целевая архитектура backend (по спеке)

```
beauty-backend/
├── crates/
│   ├── api/            # Axum: маршруты /v1, экстракторы (AuthUser, RBAC), middleware
│   ├── domain/         # чистая логика: availability, booking rules, policies, money
│   ├── infra/          # sqlx-репозитории, Stripe, R2, APNs/FCM, SMS/email, config
│   └── shared/         # ошибки (RFC 9457), ID (UUIDv7), Language, i18n, пагинация
├── bins/
│   ├── api             # HTTP-сервер
│   ├── worker          # holds-expiry, outbox, reminders, retention, метрики
│   └── admin           # разовые операции (аналог admin_tool)
├── migrations/         # чистая схема, линейная, без seed бизнес-данных
├── openapi/            # сгенерированный контракт → клиенты Swift/Kotlin
└── docs/
```

Ключевые решения (все из спеки):
1. **Тенантность:** `business_id` в каждой бизнес-таблице + **RLS** (`SET LOCAL app.business_id`), `membership(user, business, role, permissions)`; `client` принадлежит бизнесу.
2. **Слоты:** чистая функция `availability(...)` в `domain` (property-тесты) + **exclusion constraint** `EXCLUDE USING gist (staff_id WITH =, tstzrange(start_at, block_end_at) WITH &&) WHERE status IN (…) AND NOT override_overlap`.
3. **Holds** с TTL, идемпотентные POST (`Idempotency-Key`), `If-Match`/`version` для переносов.
4. **Платежи:** Stripe Connect, PaymentIntent, webhook — источник истины; проверка подписи из `assistant`.
5. **Outbox:** `notification_outbox` + `worker`, каналы push/SMS/email, локализованные шаблоны 4 языков.
6. **Auth:** OTP, Sign in with Apple/Google, refresh с ротацией и `device_id`, JWT с `kid`/`aud`, роли в членствах.
7. **GDPR:** таблицы `consent`, `audit_log`, шифрование полей здоровья, экспорт/удаление/анонимизация, retention-задачи.
8. **Контракт:** OpenAPI как источник правды → Swift-клиент для iOS.

---

## 7. Карта переиспользования

| Из `assistant` | Решение | Что изменить |
|---|---|---|
| `shared/error.rs`, `result.rs` | Адаптировать | Формат ответов RFC 9457, машинные коды (`slot_unavailable`, …) |
| `shared/types.rs` (ID-обёртки) | Адаптировать | UUIDv7, новые ID (BusinessId, StaffId, AppointmentId…) |
| `shared/language.rs`, `i18n.rs` | Адаптировать | `Language` pl/en/ru/uk уже есть; заменить пищевые сообщения на наши, шаблоны уведомлений вынести в БД/файлы |
| `shared/pagination.rs` | Адаптировать | Курсорная пагинация |
| `infrastructure/config.rs` | Адаптировать | Убрать Gemini/Groq/GA4/Telegram-переменные, добавить Stripe Connect, APNs, SMS/email, регионы |
| `infrastructure/security/jwt.rs` | Переписать по образцу | `aud`, `kid`, роли, ротация refresh, device |
| `infrastructure/security/password.rs` (argon2id) | Оставить для админки | Для пользователей — OTP/Apple/Google |
| `infrastructure/r2_client.rs`, `storage/*` | Перенести | Presigned upload для портфолио/фото, удаление EXIF |
| `infrastructure/stripe_service.rs` | Перенести **только** проверку подписи и replay-окно | Остальное заменить на Connect/PaymentIntent |
| `infrastructure/cache.rs` (moka) | Перенести | Кэш публичных профилей/услуг |
| `interfaces/http/error.rs`, `middleware.rs`, `health.rs`, `cache_middleware.rs` | Адаптировать | Экстрактор `AuthUser` без запроса в БД на каждый вызов; RBAC |
| Rate limit (`governor`) | Перенести | Лимиты на OTP и публичные эндпоинты записи |
| `tests/site_isolation.rs` | Взять как шаблон | Тесты изоляции тенантов на RLS |
| `src/bin/admin_tool.rs` | Взять паттерн | `admin` и `worker` бинарники |
| `Dockerfile`, `koyeb.yaml`, `Makefile` | Адаптировать | Закрепить Rust, cargo-chef, non-root, отдельные окружения |
| `interfaces/telegram/*` | По желанию (V2) | Канал уведомлений мастеру в Telegram — популярен у мастеров PL/UA/RU |
| `infrastructure/gemini_service.rs`, `groq_service.rs`, `llm_adapter.rs` | Не переносить сейчас | Возможно V3: помощник по описаниям/ответам на отзывы |
| Всё остальное (`rulebot`, `ai_sous_chef`, `lab_combos`, `laboratory`, `copilot`, `intent_pages`, `analytics`, `church_*`, `almabuild`, `icons_site`, catalog/inventory/recipe/dish, `public/tools/*`) | **Не переносить** | — |
| `migrations/*` | **Не переносить** | Новая схема по `PRODUCT_SPEC` §10 |

---

## 8. План работ (backend)

Оценки — грубые, для одного разработчика с ИИ-помощью; уточняются после решений Q1–Q8.

| Этап | Содержание | Результат | Оценка |
|---|---|---|---|
| **0. Решения и безопасность** | Подтвердить вариант A; создать репозиторий/окружения; тег `pre-beauty` на `assistant`; проверить, что ключи в локальном `.env` ротированы; выбрать регион ЕС (БД, хранилище); ответить на Q1, Q5, Q7, Q8 | Решения зафиксированы | 2–3 дня |
| **1. Каркас** | Workspace, CI (fmt, clippy `-D warnings`, тесты на Postgres-контейнере, `cargo audit/deny`, `sqlx prepare`), перенос shared/config/error/R2/Stripe-verify/rate-limit, Docker, health, логи | Пустой сервер деплоится в staging | 1 неделя |
| **2. Схема и тенантность** | Начальная миграция: users/auth/membership/business/staff, RLS, `audit_log`, `consent`; `btree_gist`; тесты изоляции | Схема + тесты RLS | 1 неделя |
| **3. Identity** | OTP, Apple/Google, refresh с ротацией, devices, RBAC-экстрактор, инвайты сотрудников | Полный вход и роли | 1–1,5 недели |
| **4. Каталог и расписание** | Категории/услуги/варианты/добавки, `refill_rule`, графики, перерывы, time-off, исключения; i18n-поля | CRUD + валидации | 1 неделя |
| **5. Availability + Booking** | Чистая функция слотов, holds, exclusion constraint, идемпотентность, статусы, перенос/отмена по политике, `appointment_event`; property- и конкурентные тесты | Ядро продукта, без double booking | 2 недели |
| **6. Клиенты (CRM) и GDPR** | `client`, lash-профиль (шифрование полей), согласия, экспорт/удаление/анонимизация, retention-job | Соответствие спеке 18 | 1–1,5 недели |
| **7. Платежи** | Connect onboarding, PaymentIntent депозита, webhook (идемпотентно), возвраты/удержание по политике, отчёт по платежам | Депозиты работают в test-mode | 1,5–2 недели |
| **8. Уведомления** | Outbox, worker, APNs, SMS, email, шаблоны pl/en/ru/uk, правила напоминаний, дедупликация и отмена при переносе | Цикл напоминаний | 1–1,5 недели |
| **9. Публичная часть и контент** | Публичные эндпоинты профиля/доступности, портфолио (R2), отзывы, slug/ссылки | Страница записи возможна | 1 неделя |
| **10. Отчёты** | Агрегаты `daily_metrics`, выручка/визиты/no-show | Экран статистики | 0,5–1 неделя |
| **11. Контракт и клиенты** | `utoipa` → OpenAPI 3.1 → генерация Swift-клиента, контрактные тесты | iOS может подключаться | 0,5–1 неделя (идёт параллельно с 3–10) |
| **12. Усиление** | Нагрузочные тесты записи (гонки), бэкапы/PITR, алерты, DPA/реестр обработки, pentest-чеклист ASVS L2 | Готовность к бете | 1–2 недели |

Итого MVP-backend: ориентировочно **11–15 недель**. Первые «видимые» результаты для iOS-разработки (identity + каталог + слоты + запись) — примерно через 6–7 недель, поэтому Swift-клиент можно начинать параллельно по OpenAPI-контракту и моку.

### Ближайшие шаги после вашего «да»
1. Подтвердить вариант A и ответы на Q1, Q5, Q7 (название/домен/bundle id), Q8 (провайдеры и регион).
2. Я создаю каркас нового репозитория (этап 1) — без переноса чужой доменной логики и без изменений в `assistant`.
3. Первая миграция и тесты RLS — по схеме `PRODUCT_SPEC` §10.

---

## 9. Риски миграции

| Риск | Митигация |
|---|---|
| Случайно затронуть боевые сайты | Отдельный репозиторий, отдельная БД и секреты, `assistant` не трогаем |
| Перенос «скрытых» зависимостей от кухонного домена | Переносим только перечисленные в §7 модули, каждый с ревью и тестом |
| Утечка чувствительных данных между тенантами | RLS + тесты изоляции + аудит доступа с первого дня |
| Double booking при гонках | Ограничение в БД + конкурентные тесты в CI |
| Ошибки в платежах | Webhook как источник истины, идемпотентность, test-mode прогон сценариев возврата/удержания |
| Рост сроков из-за широкой спеки | Жёсткое разделение MVP/V2 (спека §3), лист ожидания, card-on-file, роли Manager/Reception — после MVP |
| Секреты | Ротация ключей, менеджер секретов, отдельные ключи staging/prod |
