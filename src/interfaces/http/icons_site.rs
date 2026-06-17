use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

const SITE_KEY: &str = "svet-ikony";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconPage {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub short_description: String,
    pub full_description: String,
    pub image_url: String,
    pub qr_code_url: String,
    pub category: String,
    pub saint_name: String,
    pub prayer_text: String,
    pub gospel_text: String,
    pub life_text: String,
    pub history_text: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_keywords: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrayerPage {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub text: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_icon: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GospelReading {
    pub id: String,
    pub date: String,
    pub title: String,
    pub reference: String,
    pub text: String,
    pub explanation: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaintPage {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub short_description: String,
    pub biography: String,
    pub feast_day: String,
    pub image_url: String,
    pub related_icons: Vec<String>,
    pub prayers: Vec<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeoPage {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub h1: String,
    pub content: String,
    pub page_type: String,
    pub target_keyword: String,
    pub language: String,
    pub blocks: Vec<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrPage {
    pub id: String,
    pub qr_id: String,
    pub icon_id: String,
    pub slug: String,
    pub title: String,
    pub active: bool,
    pub scan_count: i64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_prayer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchPage {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub city: String,
    pub address: String,
    pub description: String,
    pub schedule: String,
    pub related_icons: Vec<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub donation_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarHero {
    pub year: String,
    pub title: String,
    pub month_title: String,
    pub prev_label: String,
    pub prev_href: String,
    pub next_label: String,
    pub next_href: String,
    pub feature_title: String,
    pub feature_note: String,
    pub feature_date: String,
    pub feature_href: String,
    pub icon_day_title: String,
    pub icon_day_icon_slug: String,
    pub icon_day_date: String,
    pub icon_day_prayer_slug: String,
    pub info_primary: String,
    pub info_secondary: String,
    pub today_date: String,
    pub today_gospel: String,
    pub today_prayer_title: String,
    pub today_href: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDay {
    pub id: String,
    pub day: String,
    pub label: String,
    pub note: String,
    pub kind: String,
    #[serde(default)]
    pub image_url: String,
    pub icon_slug: String,
    pub prayer_slug: String,
    pub gospel_slug: String,
    pub detail_href: String,
    pub current: bool,
    pub feast: bool,
    pub text_only: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarServiceCard {
    pub id: String,
    pub index: String,
    pub title: String,
    pub description: String,
    pub href: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarContent {
    pub hero: CalendarHero,
    pub days: Vec<CalendarDay>,
    pub services: Vec<CalendarServiceCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub published_pages: i64,
    pub icons: i64,
    pub prayers: i64,
    pub qr_pages: i64,
    pub qr_scans: i64,
    pub latest_pages: Vec<Value>,
    pub seo: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconsSiteContent {
    pub icons: Vec<IconPage>,
    pub prayers: Vec<PrayerPage>,
    pub gospel: Vec<GospelReading>,
    pub saints: Vec<SaintPage>,
    pub pages: Vec<SeoPage>,
    pub qr_pages: Vec<QrPage>,
    pub churches: Vec<ChurchPage>,
    #[serde(default = "default_calendar")]
    pub calendar: CalendarContent,
    pub dashboard: Dashboard,
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn default_calendar() -> CalendarContent {
    let day = |day: &str, label: &str, note: &str, kind: &str, icon_slug: &str, current: bool, feast: bool, text_only: bool, description: &str| CalendarDay {
        id: format!("calendar-jan-{day}"),
        day: day.into(),
        label: label.into(),
        note: note.into(),
        kind: kind.into(),
        image_url: String::new(),
        icon_slug: icon_slug.into(),
        prayer_slug: "molitva-kazanskoy-ikone".into(),
        gospel_slug: "today".into(),
        detail_href: if icon_slug.is_empty() { "/icons".into() } else { format!("/icons/{icon_slug}") },
        current,
        feast,
        text_only,
        description: description.into(),
    };

    CalendarContent {
        hero: CalendarHero {
            year: "2026".into(),
            title: "Свет Иконы".into(),
            month_title: "Январь 2026".into(),
            prev_label: "← Декабрь".into(),
            prev_href: "#".into(),
            next_label: "Февраль →".into(),
            next_href: "#".into(),
            feature_title: "Святитель Василий Великий".into(),
            feature_note: "Память святого".into(),
            feature_date: "14 января (ст. ст.)".into(),
            feature_href: "/saints/nikolay-chudotvorets".into(),
            icon_day_title: "Икона святителя Николая Чудотворца".into(),
            icon_day_icon_slug: "nikolay-chudotvorets".into(),
            icon_day_date: "14 января 2026".into(),
            icon_day_prayer_slug: "molitva-kazanskoy-ikone".into(),
            info_primary: "Сегодняшний праздник".into(),
            info_secondary: "Важный день".into(),
            today_date: "14 января 2026".into(),
            today_gospel: "Мф. 5:14-16".into(),
            today_prayer_title: "Молитва перед Казанской иконой Божией Матери".into(),
            today_href: "/gospel".into(),
        },
        days: vec![
            day("01", "Обрезание Господне", "Праздник", "feast", "kazan-icon", true, false, false, "Память события и начало годового молитвенного круга."),
            day("02", "", "", "quiet", "", false, false, true, ""),
            day("03", "Икона Божией Матери «Казанская»", "Праздничная икона", "feast", "kazan-icon", false, true, false, "Молитва о семье, мире и укреплении в вере."),
            day("04", "Святитель Николай Чудотворец", "Память святого", "feast", "nikolay-chudotvorets", false, false, false, "Почитание святого, помощника в пути и нужде."),
            day("05", "", "", "quiet", "", false, false, true, ""),
            day("06", "Крещение Господне", "Праздник", "feast", "kazan-icon", true, false, false, "Воспоминание Богоявления и освящения вод."),
            day("07", "Рождество Христово", "Празднество", "fast", "nikolay-chudotvorets", false, true, false, "Праздничное чтение и домашняя молитва."),
            day("08", "", "", "quiet", "", false, false, true, ""),
            day("09", "Блаженная Матрона Московская", "Память святой", "prayer", "kazan-icon", false, false, false, "Молитва о помощи в житейских обстоятельствах."),
            day("10", "", "", "quiet", "", false, false, true, ""),
            day("11", "Великомученик Пантелеимон", "Память святого", "prayer", "nikolay-chudotvorets", false, false, false, "Молитвенное обращение о болящих."),
            day("12", "", "", "quiet", "", false, false, true, ""),
            day("13", "Собор Предтечи и Крестителя Господня Иоанна", "Память святого", "feast", "nikolay-chudotvorets", false, false, false, "День молитвенного почитания Предтечи."),
            day("14", "Святитель Василий Великий", "Память святого", "feast", "nikolay-chudotvorets", true, false, false, "Память святителя и учителя Церкви."),
            day("15", "", "", "quiet", "", false, false, true, ""),
            day("16", "Икона Божией Матери «Умиление»", "Праздничная икона", "feast", "kazan-icon", false, false, false, "Молитва о мире сердца и покаянии."),
            day("17", "", "", "quiet", "", false, false, true, ""),
            day("18", "Неделя 32-я по Пятидесятнице", "Евангельское чтение", "gospel", "kazan-icon", false, false, false, "Чтение напоминает о тихом свидетельстве веры через добрые дела."),
        ],
        services: vec![
            CalendarServiceCard { id: "service-prayers".into(), index: "01".into(), title: "Молитвы на каждый день".into(), description: "Краткое правило и молитвы перед иконой.".into(), href: "/prayers".into() },
            CalendarServiceCard { id: "service-gospel".into(), index: "02".into(), title: "Евангелие дня".into(), description: "Чтение, ссылка и спокойное объяснение.".into(), href: "/gospel".into() },
            CalendarServiceCard { id: "service-feasts".into(), index: "03".into(), title: "Праздники и посты".into(), description: "Церковные даты, важные дни и отметки.".into(), href: "/p/pravoslavnaya-ikona-s-qr-kodom".into() },
            CalendarServiceCard { id: "service-icons".into(), index: "04".into(), title: "Иконы святых".into(), description: "История образов, жития и QR-страницы.".into(), href: "/icons".into() },
        ],
    }
}

fn default_content() -> IconsSiteContent {
    let now = now();
    IconsSiteContent {
        icons: vec![
            IconPage { id: "icon-kazan".into(), slug: "kazan-icon".into(), title: "Казанская икона Божией Матери".into(), short_description: "Перед Казанской иконой молятся о помощи семье, мире и укреплении в вере.".into(), full_description: "Казанская икона Божией Матери почитается как образ материнского заступничества и духовной поддержки.".into(), image_url: "/images/kazan-icon.svg".into(), qr_code_url: "/images/qr-code.svg".into(), category: "Богородичные".into(), saint_name: "Пресвятая Богородица".into(), prayer_text: "Пресвятая Богородице, помоги нам обратиться к Богу с миром, покаянием и надеждой.".into(), gospel_text: "Евангелие дня представлено для внимательного чтения и размышления.".into(), life_text: "Почитание образа связано с молитвенной традицией Церкви.".into(), history_text: "История Казанского образа напоминает о бережном отношении к святыне и молитве.".into(), status: "published".into(), seo_title: Some("Казанская икона Божией Матери: молитва и история образа".into()), seo_description: Some("Молитва, история и духовные материалы к Казанской иконе Божией Матери.".into()), seo_keywords: Some("Казанская икона, молитва, Богородица".into()), created_at: now.clone(), updated_at: now.clone() },
            IconPage { id: "icon-nikolay".into(), slug: "nikolay-chudotvorets".into(), title: "Икона святителя Николая Чудотворца".into(), short_description: "Перед образом святителя Николая молятся о помощи в пути, семье и трудных обстоятельствах.".into(), full_description: "Страница собирает молитву, краткое житие, историю почитания и материалы для духовной поддержки.".into(), image_url: "/images/nikolay-icon.svg".into(), qr_code_url: "/images/qr-code.svg".into(), category: "Святые".into(), saint_name: "Святитель Николай".into(), prayer_text: "Святителю отче Николае, моли Бога о нас и помоги укрепиться в добрых делах.".into(), gospel_text: "Чтение дня помогает соединить молитву у иконы с евангельским словом.".into(), life_text: "Святитель Николай известен милосердием и верностью Христу.".into(), history_text: "Почитание святителя Николая распространено во всем православном мире.".into(), status: "published".into(), seo_title: Some("Икона Николая Чудотворца: молитва, житие и помощь в чтении".into()), seo_description: Some("Православная страница иконы святителя Николая с молитвой, житием и QR-доступом.".into()), seo_keywords: None, created_at: now.clone(), updated_at: now.clone() },
        ],
        prayers: vec![PrayerPage { id: "prayer-kazan".into(), slug: "molitva-kazanskoy-ikone".into(), title: "Молитва перед Казанской иконой Божией Матери".into(), text: "Пресвятая Богородице, помоги нам обратиться к Богу с миром, покаянием и надеждой.".into(), category: "Богородичные молитвы".into(), related_icon: Some("kazan-icon".into()), status: "published".into(), seo_title: Some("Молитва перед Казанской иконой Божией Матери".into()), seo_description: Some("Текст молитвы перед Казанской иконой и спокойное объяснение для чтения.".into()) }],
        gospel: vec![GospelReading { id: "gospel-today".into(), date: chrono::Utc::now().date_naive().to_string(), title: "Евангелие дня".into(), reference: "Мф. 5:14-16".into(), text: "Вы свет мира. Не может укрыться город, стоящий на верху горы.".into(), explanation: "Чтение напоминает о тихом свидетельстве веры через добрые дела.".into(), status: "published".into(), seo_title: Some("Евангелие дня: чтение и краткое толкование".into()), seo_description: Some("Евангельское чтение дня с кратким объяснением.".into()) }],
        saints: vec![SaintPage { id: "saint-nikolay".into(), slug: "nikolay-chudotvorets".into(), name: "Святитель Николай Чудотворец".into(), short_description: "Святой, почитаемый за милосердие и помощь нуждающимся.".into(), biography: "Святитель Николай был архипастырем, заботившимся о людях и направлявшим верующих к жизни во Христе.".into(), feast_day: "19 декабря".into(), image_url: "/images/nikolay-icon.svg".into(), related_icons: vec!["nikolay-chudotvorets".into()], prayers: vec!["molitva-nikolayu".into()], status: "published".into(), seo_title: Some("Святитель Николай Чудотворец: житие и молитвы".into()), seo_description: Some("Краткое житие святителя Николая, день памяти и молитвы.".into()) }],
        pages: vec![],
        qr_pages: vec![QrPage { id: "qr-home-001".into(), qr_id: "home-001".into(), icon_id: "icon-kazan".into(), slug: "home-001".into(), title: "Домашняя Казанская икона".into(), owner_name: Some("Семейная икона".into()), location: Some("Домашний киот".into()), custom_prayer: Some("Помяни, Господи, нашу семью и помоги нам жить в мире.".into()), active: true, scan_count: 128, created_at: now.clone(), updated_at: now.clone() }],
        churches: vec![],
        calendar: default_calendar(),
        dashboard: Dashboard { published_pages: 12, icons: 2, prayers: 1, qr_pages: 1, qr_scans: 128, latest_pages: vec![], seo: vec![] },
    }
}

async fn load_content(pool: &PgPool) -> Result<IconsSiteContent, StatusCode> {
    let row: Option<Value> = sqlx::query_scalar("SELECT content FROM site_content WHERE site = $1")
        .bind(SITE_KEY)
        .fetch_optional(pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to load icons site content");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match row {
        Some(value) => serde_json::from_value(value).map_err(|error| {
            tracing::error!(%error, "invalid icons site content json");
            StatusCode::INTERNAL_SERVER_ERROR
        }),
        None => Ok(default_content()),
    }
}

async fn save_content(pool: &PgPool, content: &IconsSiteContent) -> Result<(), StatusCode> {
    let value = serde_json::to_value(content).map_err(|error| {
        tracing::error!(%error, "failed to serialize icons site content");
        StatusCode::BAD_REQUEST
    })?;
    sqlx::query(
        r#"INSERT INTO site_content (site, content, updated_at)
           VALUES ($1, $2, NOW())
           ON CONFLICT (site) DO UPDATE SET content = EXCLUDED.content, updated_at = NOW()"#,
    )
    .bind(SITE_KEY)
    .bind(value)
    .execute(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "failed to save icons site content");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(())
}

pub async fn public_content(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?))
}

pub async fn public_icons(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?.icons))
}

pub async fn public_icon(Path(slug): Path<String>, State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    load_content(&pool).await?.icons.into_iter().find(|item| item.slug == slug).map(Json).ok_or(StatusCode::NOT_FOUND)
}

pub async fn public_prayers(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?.prayers))
}

pub async fn public_prayer(Path(slug): Path<String>, State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    load_content(&pool).await?.prayers.into_iter().find(|item| item.slug == slug).map(Json).ok_or(StatusCode::NOT_FOUND)
}

pub async fn public_gospel_today(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?.gospel.into_iter().next().unwrap_or_else(|| {
        let mut content = default_content();
        content.gospel.remove(0)
    })))
}

pub async fn public_saints(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?.saints))
}

pub async fn public_saint(Path(slug): Path<String>, State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    load_content(&pool).await?.saints.into_iter().find(|item| item.slug == slug).map(Json).ok_or(StatusCode::NOT_FOUND)
}

pub async fn public_churches(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?.churches))
}

pub async fn public_qr(Path(qr_id): Path<String>, State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    load_content(&pool).await?.qr_pages.into_iter().find(|item| item.qr_id == qr_id).map(Json).ok_or(StatusCode::NOT_FOUND)
}

pub async fn public_seo_page(Path(slug): Path<String>, State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    load_content(&pool).await?.pages.into_iter().find(|item| item.slug == slug).map(Json).ok_or(StatusCode::NOT_FOUND)
}

pub async fn admin_get_content(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?))
}

pub async fn admin_put_content(State(pool): State<PgPool>, Json(content): Json<IconsSiteContent>) -> Result<impl IntoResponse, StatusCode> {
    save_content(&pool, &content).await?;
    Ok(Json(content))
}
