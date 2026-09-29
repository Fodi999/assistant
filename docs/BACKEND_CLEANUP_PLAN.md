# BeautyApp — план полной очистки `assistant` до beauty-only backend

Дата: 2026-09-29 · Статус: **только план**, в `~/Desktop/assistant` ничего не менялось.
Дополняет `BACKEND_AUDIT_AND_PLAN.md` (аудит и этапы 1–12) и `PRODUCT_SPEC.md`.

**Решение владельца:** в этом backend остаётся **только BeautyApp**; всё остальное (кухня, церковь, стройка, уборка, SEO, AI-копилоты, Telegram-бот) удаляется; код полностью переделывается под одну платформу.
Это меняет рекомендацию из аудита (вариант A «новый репозиторий»): теперь вариант B — **очистка на месте**, но с обязательной «страховкой» из раздела 1, потому что удаление необратимо.

---

## 0. Как понимаю задачу

- «Одна платформа» = один продукт (beauty-бизнес), один домен, одна схема БД, один набор ролей. Не мульти-сайтовая CMS.
- После очистки repo — это `beauty-backend`: Rust/Axum/sqlx/Postgres, по архитектуре из `BACKEND_AUDIT_AND_PLAN.md` §6.
- Если «платформа» означает что-то ещё (например, «только iOS»), скажите — на очистку это не влияет, backend остаётся общим для iOS/Android/web.

---

## 1. Ворота безопасности (до первого удаления)

Ничего не удаляем, пока не закрыты пункты 1.1–1.5. Цена ошибки: потеря боевых данных и поломка сайтов, которые ходят в этот API.

| # | Проверка | Как |
|---|---|---|
| 1.1 | **Кто ещё использует этот backend.** В `koyeb.yaml` в CORS перечислены: czystetrojmiasto.pl, kazaxbud.pages.dev, b2b-saas-tau.vercel.app, dima-fomin.pl; на Рабочем столе есть `svetikony-admin`, `kazaxbud`, `admin`, `admin-2`, `blog`, ChefOS iOS (usage-трекинг в API) | Для каждого — решить: **отключить**, **перенести на legacy-сервис** или **уже не нужен** |
| 1.2 | **Бэкап БД (Neon):** `pg_dump` всех схем + сохранить Neon branch/snapshot | Хранить вне репозитория, проверить восстановление на пустой БД |
| 1.3 | **Бэкап файлов R2** (иконки, заказы, аудио, изображения) | Bucket не трогаем и не переименовываем; для beauty — **новый** bucket |
| 1.4 | **Git-страховка:** тег `legacy-final` на текущем `main`, ветка `legacy/main`, GitHub-репозиторий не удалять (в будущем переименовать в `assistant-legacy`, архивировать) | `git tag legacy-final && git push origin legacy-final legacy/main` — *команды выполню только после вашего «да»* |
| 1.5 | **Отключить автодеплой** `main → Koyeb` на время работ (сейчас push в `main` = автоматический redeploy) | Иначе первый же коммит с удалением выкатится на боевой сервис |

Ветка `remotes/origin/backend-only-cleanup` в репозитории уже есть — посмотреть, что в ней, чтобы не дублировать чужую работу (в этом аудите её содержимое не открывал).

**Сервис и БД для beauty — новые:** новый Koyeb-сервис (или другой EU-хостинг), новый Neon-проект в EU-регионе, новые ключи (JWT, Stripe, R2, APNs). Старый сервис либо продолжает жить с `legacy/main`, либо выключается осознанно (решение по 1.1).

---

## 2. Итоговая цель

| Показатель | Сейчас | После очистки |
|---|---|---|
| Строк кода | ~117 000 | ~3 500–4 500 (каркас), далее растёт только beauty-доменом |
| Файлов в `src/` | 310 | ~40–50 |
| Миграций | 157 (98 таблиц) | 0 старых; новая линейная схема по спеке §10 |
| Зависимости Cargo | ~35 | ~25 (без image, flate2, Gemini/Groq-обвязки и т.д.) |
| Продуктов в одном процессе | 5+ | 1 |
| Имя crate | `restaurant-backend` | `beauty-backend` |

---

## 3. Манифест: что сохраняем / переписываем / удаляем

### 3.1. Сохраняем и адаптируем (~3–4k строк)
| Путь | Что делаем |
|---|---|
| `src/shared/{error,result,types,pagination,language,i18n}.rs` | `TenantId` → `BusinessId`; ID на UUIDv7; ошибки в формате RFC 9457; `i18n.rs` — убрать пищевые сообщения, оставить механизм pl/en/ru/uk |
| `src/infrastructure/config.rs` | Удалить `AiConfig`, `TelegramConfig`, GA4/Google/Search Console; добавить Stripe Connect, APNs, SMS/email, регион |
| `src/infrastructure/security/{jwt,password}.rs` | JWT: `aud`, `kid`, роли/членства, ротация refresh (спека 9.1); argon2 оставить для админки |
| `src/infrastructure/{r2_client,storage/*}.rs` | Оставить; presigned upload для портфолио/фото, удаление EXIF |
| `src/infrastructure/stripe_service.rs` | Оставить **только** проверку подписи webhook и окно replay 5 мин; Checkout-бандиты AI-действий удалить |
| `src/infrastructure/cache.rs` (moka) | Оставить для публичных профилей/услуг |
| `src/interfaces/http/{error,middleware,health,cache_middleware}.rs` | `AuthUser` без запроса в БД на каждый вызов; добавить RBAC-экстрактор |
| `src/application/{admin_auth,auth,user}.rs`, `domain/{admin,auth,user,tenant}.rs` | Переписать под `user + membership + business`, роли owner/manager/reception/employee/customer |
| `src/infrastructure/persistence/{refresh_token,user,tenant}_repository.rs`, `mod.rs` | Переписать под новую схему |
| `src/bin/{admin_tool,generate_admin_hash}.rs` | Оставить каркас; команды переписать (миграции, проверки данных, retention) |
| `src/main.rs`, `src/lib.rs`, `src/interfaces/http/{mod,routes}.rs` | **Переписать с нуля** (сейчас `routes.rs` 2178 строк — вырезать целиком, новый роутер `/v1/...`) |
| `build.rs`, `Makefile`, `Dockerfile`, `koyeb.yaml`, `.sqlxrc`, `.gitignore`, `.dockerignore` | Адаптировать (раздел 7) |
| `SECURITY.md` | Оставить, поправить под BeautyApp |
| `tests/site_isolation.rs` | Оставить **как образец** паттерна `#[sqlx::test]`, содержимое заменить тестами изоляции `business_id`/RLS |

### 3.2. Удаляем целиком
| Что | Путь |
|---|---|
| Кулинарный/рецептный домен | `domain/{catalog,dish,inventory,recipe,recipe_v2,recipe_ai_insights,menu_engineering,tenant_ingredient,processing_state,classification_rules,report,usage,user_preferences,ai_ports}.rs`, `domain/{engines,tools,matter,assistant}/` |
| Кулинарные сервисы и AI | `application/{rulebot,ai_sous_chef,lab_combos,laboratory,copilot,smart_service,smart_parse,sous_chef}/`, `application/{admin_catalog,admin_nutrition,analytics,assistant_service,catalog,chat_events_service,cms_service,cook_suggestions,dish,intent_pages,inventory,inventory_alert,menu_engineering,prayer_visualizer,preferences_service,public_nutrition,public_seo_content,purchase_draft,recipe*,report,tenant_ingredient,usage_service}.rs` |
| Церковь, стройка, уборка, SEO, CMS | `interfaces/http/{church_content,church_orders,church_prayer_visualizer,icons_site,almabuild,admin_cms,admin_panel,admin_intent_pages,admin_lab_combos,admin_analytics,admin_search_console,admin_ai,admin_catalog,admin_nutrition,admin_states,chef_reference_public,site_context}.rs`, вся `interfaces/http/public/` |
| Кулинарные HTTP-ручки | `interfaces/http/{assistant,catalog,cook_suggestions,copilot,dish,inventory,laboratory,menu_engineering,preferences,recipe*,report,smart*,tenant_ingredient,usage,billing}.rs` |
| Telegram-бот «Свет Иконы» | `interfaces/telegram/`, `tests/telegram_webhook.rs` (идея канала для мастеров — вернуть в V2 из тега `legacy-final`) |
| AI-клиенты и кэши | `infrastructure/{gemini_service,groq_service,llm_adapter,ai_client_impl,icon_image_prompts,ingredient_cache}.rs`, `infrastructure/gemini/`, `assistant.code-workspace` |
| Репозитории кухни/CMS | все остальные файлы `infrastructure/persistence/` |
| Данные/артефакты | `migrations/*` (все 157), `cleanup_catalog.sql`, `AI_INSIGHTS_V2_ARCHITECTURE.md`, `uploads/`, `.claude/`, `.env` (локальный, не в git — передать секреты в менеджер, старые ключи ротировать), каталог `target/` (8,1 ГБ, `cargo clean`) |
| Docs | `README.md`, `ARCHITECTURE.md` — переписать заново (не «редактировать») |

### 3.3. Зависимости Cargo
Удалить: `image`, `flate2` (визуализатор молитв), `base64`, `home`/`base64ct` (пины под edition2024 — после закрепления актуального toolchain), `regex-lite` (если не нужен), `rust_decimal` и `chrono` (деньги в minor units, время только `time`), `const_format` (если не используется).
Оставить: `axum`, `tokio`, `tower(-http)`, `serde(_json)`, `sqlx`, `uuid` (+`v7`), `time`, `aws-sdk-s3/aws-config`, `argon2`, `jsonwebtoken`, `sha2`, `hmac`, `hex`, `subtle`, `validator`, `thiserror`, `anyhow`, `reqwest`, `tracing*`, `dotenvy`, `governor`, `mini-moka`, `async-trait`, `rand`, `deunicode` (slug мастеров).
Добавить: `utoipa` (+swagger/redoc), `sqlx` с макросами `query!` и офлайн-режимом, `cargo-deny`/`cargo-audit` в CI, позже клиент APNs, SMS/email-провайдеры.

---

## 4. Порядок работ и контрольные точки

Каждый шаг заканчивается зелёным `cargo check` (и `cargo test` там, где есть тесты). Работаем на новой ветке; `legacy/main` не трогаем.

| Шаг | Что делаем | Критерий готовности |
|---|---|---|
| **C0** | Ворота из раздела 1 закрыты | Бэкапы проверены, автодеплой выключен, тег и ветка на GitHub |
| **C1** | Ветка `beauty/main`. Вариант истории — см. раздел 6 (по умолчанию: **orphan-ветка**, чистая история) | Ветка создана |
| **C2** | Вырезать `routes.rs`, `main.rs` до минимума: `/health`, конфиг, пул БД, логи. Удалить всё, что перестало компилироваться/используется, слоями: (а) HTTP-ручки, (б) application, (в) domain, (г) infrastructure/persistence | `cargo check` зелёный после каждого слоя; сервер стартует, `/health` отвечает |
| **C3** | Удалить `migrations/*`, убрать из `main.rs` автоприменение миграций и странный `DELETE FROM _sqlx_migrations WHERE success = false` | В репозитории нет старых миграций; миграции вынесены в шаг деплоя/`admin` |
| **C4** | Зачистка зависимостей (`cargo machete` / `cargo udeps`), пересборка | Cargo.toml по разделу 3.3; время сборки с нуля падает в разы |
| **C5** | Переименование: crate `restaurant-backend` → `beauty-backend`, `restaurant_backend` в тестах, имена бинарников, `docker`/`koyeb` | Нигде нет слов `restaurant`, `chef`, `dish`, `recipe`, `church`, `icon`, `almabuild`, `gemini`, `groq` (проверка `grep -ri`) |
| **C6** | Config и `.env.example` под beauty; удалить старые переменные | `.env.example` содержит только используемые ключи |
| **C7** | Docs: новые `README.md`, `ARCHITECTURE.md`, правка `SECURITY.md` | Соответствуют коду и `PRODUCT_SPEC.md` |
| **C8** | CI (GitHub Actions): `fmt`, `clippy -D warnings`, `cargo test` на Postgres-сервисе, `cargo deny`, `sqlx prepare --check`; Dockerfile (закреплённый Rust, `cargo-chef`, non-root) | CI зелёный, образ собирается |
| **C9** | Деплой пустого каркаса в **новый** staging (EU) | `/health` доступен, метрики/логи идут |
| **C10** | Дальше — этапы 2–12 из `BACKEND_AUDIT_AND_PLAN.md` §8: схема + RLS, identity, каталог/расписание, availability + booking, CRM/GDPR, платежи, уведомления, отчёты, OpenAPI | По этапам |

**Оценка очистки (C0–C9):** 4–6 рабочих дней (включая бэкапы и решение по внешним сайтам). Общий срок backend MVP остаётся 11–15 недель (аудит §8), т.к. этап 1 «каркас» становится частью этой очистки.

---

## 5. Что переезжает в новую схему

Старые таблицы `tenants`, `users`, `refresh_tokens` не мигрируем — данных beauty ещё нет. Новая схема по `PRODUCT_SPEC.md` §10:
`user`, `auth_identity`, `device`, `consent`, `business`, `membership`, `staff_member`, `service_*`, `working_schedule`, `time_off`, `client`, `client_lash_profile`, `appointment` (+ exclusion constraint), `payment_account`, `payment`, `notification_outbox`, `review`, `portfolio_item`, `audit_log`, `daily_metrics`.
Старые данные ChefOS/церкви — только в бэкапе (1.2) и в `legacy/main`; **в новую базу не импортируются**.

---

## 6. Git-история: два варианта

| Вариант | Плюсы | Минусы |
|---|---|---|
| **Orphan-ветка (рекомендую):** новая `main` без истории, старая — в `legacy/main` и теге | Чистая история продукта, нет чужих коммитов (церковь, ключи в старых коммитах не найдены, но остаются в истории), проще публиковать/передавать | История «кто что менял» для beauty начинается заново |
| Удаления коммитами в текущей истории | Сохраняется история общих модулей (jwt, r2, stripe verify) | В репозитории навсегда остаются чужие данные/контент в истории; сложнее аудит GDPR |

Репозиторий на GitHub: после завершения — переименовать текущий в `assistant-legacy` (архив) и создать/переименовать новый `beauty-backend`, либо оставить имя и сменить описание. Ссылки на старый URL GitHub редиректит, но Koyeb-интеграцию нужно перепривязать.

---

## 7. Инфраструктура после очистки

- **Окружения:** dev (локально, Docker Postgres), staging, prod — разные БД и секреты.
- **Хостинг:** любой EU-регион (Koyeb Frankfurt/Neon EU или Fly/Hetzner + Postgres); минимум 2 инстанса api + 1 worker для prod; миграции — отдельным шагом деплоя.
- **Секреты:** только менеджер секретов; старые ключи (Stripe, Gemini, R2, Groq, GA4, Telegram) — ротировать/отозвать после отключения старых сервисов.
- **CORS:** только домены beauty (API-хост, страница записи), без `localhost` в prod и без личных сайтов.
- **Файлы:** новый R2-bucket beauty (EU), политика жизненного цикла для удалённых фото.
- **Dockerfile:** закреплённый toolchain, `cargo-chef`, non-root пользователь, healthcheck.
- **Наблюдаемость:** JSON-логи `tracing`, метрики, алерты на 5xx/задержку расчёта слотов.

---

## 8. Definition of Done для очистки

- [ ] Бэкапы и `legacy/main` + тег на GitHub, восстановление проверено
- [ ] Автодеплой старого сервиса не может выкатить beauty-код, и наоборот
- [ ] В репозитории нет кода и данных кухни/церкви/стройки/уборки/SEO/AI/Telegram
- [ ] `grep -ri` по запрещённым словам — пусто
- [ ] `cargo build --release`, `clippy -D warnings`, `cargo test` зелёные; `cargo deny` без блокеров
- [ ] Старых миграций нет; миграции не запускаются при старте процесса
- [ ] `README`, `ARCHITECTURE`, `SECURITY`, `.env.example` актуальны
- [ ] Staging в EU отвечает на `/health`
- [ ] Старые секреты ротированы

---

## 9. Риски

| Риск | Вероятность/влияние | Митигация |
|---|---|---|
| **Поломка внешних сайтов и ChefOS iOS**, ходящих в этот API | Высокая при неотключённом автодеплое | Ворота 1.1 и 1.5; отдельный legacy-сервис, если нужен |
| Потеря боевых данных | Средняя/критическая | Бэкапы 1.2–1.3, тег, ветка, отдельные БД |
| Тихие зависимости кода от кухонного домена в «сохраняемых» файлах | Средняя | Удаление слоями с `cargo check`; `shared/`, `r2_client`, `security`, `error.rs` не импортируют доменные слои (проверено) |
| Раздувание срока «до первого beauty-кода» | Средняя | Жёсткий лимит 4–6 дней на C0–C9 |
| Утечка старых секретов из локального `.env` и истории | Низкая | Ротация ключей, orphan-история |
| Смешение репозиториев в Koyeb/GitHub интеграциях | Средняя | Отдельный сервис и отдельные env; перепривязка после переименования |

---

## 10. Что нужно от вас

1. **Внешние потребители старого API (1.1):** какие из сайтов/приложений ещё живые и нужны? (отключаем / оставляем на legacy / уже не нужны)
2. **Вариант истории (раздел 6):** orphan-ветка (рекомендую) или удаление коммитами.
3. **Новая БД и хостинг:** Neon EU и Koyeb как сейчас, или другой EU-хостинг?
4. **Разрешение на git-страховку** (тег `legacy-final`, ветка `legacy/main`, push на GitHub) и на отключение автодеплоя — эти шаги трогают ваш удалённый репозиторий.
5. Название/домен/bundle id (Q7) — нужно для переименования crate и конфигов.

После ответов начинаю с C0–C1. До вашего «да» ничего в `assistant` не меняю.
