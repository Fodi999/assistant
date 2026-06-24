use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{Datelike, Duration, NaiveDate};
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub image_urls: Vec<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar_date: Option<String>,
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
    #[serde(default)]
    pub gregorian_date: String,
    #[serde(default)]
    pub julian_day: String,
    #[serde(default)]
    pub julian_date: String,
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
    let day = |day: &str,
               label: &str,
               note: &str,
               kind: &str,
               icon_slug: &str,
               current: bool,
               feast: bool,
               text_only: bool,
               description: &str| CalendarDay {
        id: format!("calendar-jan-{day}"),
        day: day.into(),
        gregorian_date: String::new(),
        julian_day: String::new(),
        julian_date: String::new(),
        label: label.into(),
        note: note.into(),
        kind: kind.into(),
        image_url: String::new(),
        icon_slug: icon_slug.into(),
        prayer_slug: "molitva-kazanskoy-ikone".into(),
        gospel_slug: "today".into(),
        detail_href: if icon_slug.is_empty() {
            "/icons".into()
        } else {
            format!("/icons/{icon_slug}")
        },
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
            day("01", "", "", "quiet", "", false, false, true, ""),
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
            day("14", "Обрезание Господне", "Господский праздник", "feast", "obrezanie-gospodne", true, true, false, "Праздник Обрезания Господня: 1 января по церковному юлианскому календарю, 14 января по гражданскому календарю."),
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
            IconPage { id: "icon-kazan".into(), slug: "kazan-icon".into(), title: "Казанская икона Божией Матери".into(), short_description: "Перед Казанской иконой молятся о помощи семье, мире и укреплении в вере.".into(), full_description: "Казанская икона Божией Матери почитается как образ материнского заступничества и духовной поддержки.".into(), image_url: "/images/kazan-icon.svg".into(), image_urls: vec![], qr_code_url: "/images/qr-code.svg".into(), category: "Богородичные".into(), saint_name: "Пресвятая Богородица".into(), prayer_text: "Пресвятая Богородице, помоги нам обратиться к Богу с миром, покаянием и надеждой.".into(), gospel_text: "Евангелие дня представлено для внимательного чтения и размышления.".into(), life_text: "Почитание образа связано с молитвенной традицией Церкви.".into(), history_text: "История Казанского образа напоминает о бережном отношении к святыне и молитве.".into(), status: "published".into(), seo_title: Some("Казанская икона Божией Матери: молитва и история образа".into()), seo_description: Some("Молитва, история и духовные материалы к Казанской иконе Божией Матери.".into()), seo_keywords: Some("Казанская икона, молитва, Богородица".into()), calendar_date: None, created_at: now.clone(), updated_at: now.clone() },
            IconPage { id: "icon-nikolay".into(), slug: "nikolay-chudotvorets".into(), title: "Икона святителя Николая Чудотворца".into(), short_description: "Перед образом святителя Николая молятся о помощи в пути, семье и трудных обстоятельствах.".into(), full_description: "Страница собирает молитву, краткое житие, историю почитания и материалы для духовной поддержки.".into(), image_url: "/images/nikolay-icon.svg".into(), image_urls: vec![], qr_code_url: "/images/qr-code.svg".into(), category: "Святые".into(), saint_name: "Святитель Николай".into(), prayer_text: "Святителю отче Николае, моли Бога о нас и помоги укрепиться в добрых делах.".into(), gospel_text: "Чтение дня помогает соединить молитву у иконы с евангельским словом.".into(), life_text: "Святитель Николай известен милосердием и верностью Христу.".into(), history_text: "Почитание святителя Николая распространено во всем православном мире.".into(), status: "published".into(), seo_title: Some("Икона Николая Чудотворца: молитва, житие и помощь в чтении".into()), seo_description: Some("Православная страница иконы святителя Николая с молитвой, житием и QR-доступом.".into()), seo_keywords: None, calendar_date: None, created_at: now.clone(), updated_at: now.clone() },
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

struct FixedCalendarRule {
    julian_month: u32,
    julian_day: u32,
    label: &'static str,
    note: &'static str,
    kind: &'static str,
    feast: bool,
    priority: i32,
    aliases: &'static [&'static str],
    description: &'static str,
}

const FIXED_CALENDAR_RULES: &[FixedCalendarRule] = &[
    FixedCalendarRule {
        julian_month: 1,
        julian_day: 1,
        label: "Обрезание Господне",
        note: "Господский праздник",
        kind: "feast",
        feast: true,
        priority: 100,
        aliases: &["обрезание господне", "обрезанию господню", "circumcision of our lord"],
        description: "Праздник Обрезания Господня: 1 января по церковному юлианскому календарю, 14 января по гражданскому календарю. Источник: OCA Feasts & Saints; православный календарь 1/14 января.",
    },
    FixedCalendarRule {
        julian_month: 12,
        julian_day: 25,
        label: "Рождество Христово",
        note: "Двунадесятый праздник",
        kind: "feast",
        feast: true,
        priority: 100,
        aliases: &["рождество христово", "рождеству христову", "nativity of christ"],
        description: "Рождество Христово: 25 декабря по юлианскому календарю, 7 января по гражданскому календарю.",
    },
    FixedCalendarRule {
        julian_month: 1,
        julian_day: 6,
        label: "Крещение Господне",
        note: "Богоявление",
        kind: "feast",
        feast: true,
        priority: 100,
        aliases: &["крещение господне", "богоявление", "theophany", "baptism of the lord"],
        description: "Крещение Господне, или Богоявление: 6 января по юлианскому календарю, 19 января по гражданскому календарю.",
    },
    FixedCalendarRule {
        julian_month: 1,
        julian_day: 7,
        label: "Собор Предтечи и Крестителя Господня Иоанна",
        note: "Память святого",
        kind: "feast",
        feast: false,
        priority: 60,
        aliases: &["собор предтечи", "собор иоанна предтечи", "john the baptist synaxis"],
        description: "Собор Иоанна Предтечи: 7 января по юлианскому календарю, 20 января по гражданскому календарю.",
    },
    FixedCalendarRule {
        julian_month: 1,
        julian_day: 1,
        label: "Святитель Василий Великий",
        note: "Память святого",
        kind: "feast",
        feast: false,
        priority: 50,
        aliases: &["василий великий", "святитель василий", "basil the great"],
        description: "Память святителя Василия Великого совершается 1 января по юлианскому календарю, 14 января по гражданскому календарю.",
    },
];

#[derive(Debug, Deserialize)]
pub struct CalendarQuery {
    pub year: Option<i32>,
    pub month: Option<u32>,
}

fn normalize_lookup_text(value: &str) -> String {
    value
        .to_lowercase()
        .replace('ё', "е")
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn calendar_rule_for_text(value: &str) -> Option<&'static FixedCalendarRule> {
    let haystack = normalize_lookup_text(value);
    FIXED_CALENDAR_RULES.iter().find(|rule| {
        rule.aliases
            .iter()
            .any(|alias| haystack.contains(&normalize_lookup_text(alias)))
    })
}

fn calendar_rule_for_icon(icon: &IconPage) -> Option<&'static FixedCalendarRule> {
    calendar_rule_for_text(
        &[
            icon.title.as_str(),
            icon.slug.as_str(),
            icon.category.as_str(),
            icon.saint_name.as_str(),
            icon.short_description.as_str(),
            icon.full_description.as_str(),
            icon.seo_title.as_deref().unwrap_or_default(),
            icon.seo_description.as_deref().unwrap_or_default(),
            icon.seo_keywords.as_deref().unwrap_or_default(),
        ]
        .join(" "),
    )
}

fn empty_calendar_day(day: u32, year: i32, month: u32, current: bool) -> CalendarDay {
    let gregorian = NaiveDate::from_ymd_opt(year, month, day);
    let julian = gregorian.map(gregorian_to_julian);
    let gregorian_date = gregorian.map(format_gregorian_date).unwrap_or_default();
    let julian_day = julian
        .map(|date| format!("{:02}", date.day()))
        .unwrap_or_default();
    let julian_date = julian.map(format_julian_date).unwrap_or_default();
    let description = julian
        .map(|date| format!("{} по юлианскому календарю.", format_julian_date(date)))
        .unwrap_or_default();

    CalendarDay {
        id: format!("calendar-{year}-{month:02}-{day:02}"),
        day: format!("{day:02}"),
        gregorian_date,
        julian_day,
        julian_date,
        label: String::new(),
        note: String::new(),
        kind: "quiet".into(),
        image_url: String::new(),
        icon_slug: String::new(),
        prayer_slug: String::new(),
        gospel_slug: "today".into(),
        detail_href: "/icons".into(),
        current,
        feast: false,
        text_only: true,
        description,
    }
}

fn fixed_calendar_day(
    rule: &'static FixedCalendarRule,
    gregorian: NaiveDate,
    current: bool,
    icon_slug: Option<&str>,
) -> CalendarDay {
    let icon_slug = icon_slug.unwrap_or_default();
    let julian = gregorian_to_julian(gregorian);
    CalendarDay {
        id: format!("calendar-{}", gregorian.format("%Y-%m-%d")),
        day: format!("{:02}", gregorian.day()),
        gregorian_date: format_gregorian_date(gregorian),
        julian_day: format!("{:02}", julian.day()),
        julian_date: format_julian_date(julian),
        label: rule.label.into(),
        note: rule.note.into(),
        kind: rule.kind.into(),
        image_url: String::new(),
        icon_slug: icon_slug.into(),
        prayer_slug: icon_slug.into(),
        gospel_slug: "today".into(),
        detail_href: if icon_slug.is_empty() {
            "/icons".into()
        } else {
            format!("/icons/{icon_slug}")
        },
        current,
        feast: rule.feast,
        text_only: false,
        description: rule.description.into(),
    }
}

fn saint_calendar_day(saint: &SaintPage, gregorian: NaiveDate, current: bool) -> CalendarDay {
    let icon_slug = saint
        .related_icons
        .first()
        .map(String::as_str)
        .unwrap_or_default();
    let julian = gregorian_to_julian(gregorian);
    CalendarDay {
        id: format!(
            "calendar-saint-{}-{}",
            saint.slug,
            gregorian.format("%Y-%m-%d")
        ),
        day: format!("{:02}", gregorian.day()),
        gregorian_date: format_gregorian_date(gregorian),
        julian_day: format!("{:02}", julian.day()),
        julian_date: format_julian_date(julian),
        label: saint.name.clone(),
        note: "Память святого".into(),
        kind: "feast".into(),
        image_url: saint.image_url.clone(),
        icon_slug: icon_slug.into(),
        prayer_slug: saint.prayers.first().cloned().unwrap_or_default(),
        gospel_slug: "today".into(),
        detail_href: format!("/saints/{}", saint.slug),
        current,
        feast: false,
        text_only: false,
        description: format!(
            "{}: {}",
            format_julian_date(julian),
            saint.short_description
        ),
    }
}

fn icon_calendar_day(icon: &IconPage, gregorian: NaiveDate, current: bool) -> CalendarDay {
    let icon_slug = if icon.slug.is_empty() {
        icon.id.as_str()
    } else {
        icon.slug.as_str()
    };
    let julian = gregorian_to_julian(gregorian);
    let description = if !icon.short_description.trim().is_empty() {
        icon.short_description.clone()
    } else if let Some(seo_description) = icon.seo_description.as_deref() {
        seo_description.to_string()
    } else {
        icon.full_description.clone()
    };
    let kind = if normalize_lookup_text(&icon.category).contains("молит") {
        "prayer"
    } else {
        "feast"
    };

    CalendarDay {
        id: format!(
            "calendar-icon-{}-{}",
            icon_slug,
            gregorian.format("%Y-%m-%d")
        ),
        day: format!("{:02}", gregorian.day()),
        gregorian_date: format_gregorian_date(gregorian),
        julian_day: format!("{:02}", julian.day()),
        julian_date: format_julian_date(julian),
        label: icon.title.clone(),
        note: if icon.saint_name.trim().is_empty() {
            icon.category.clone()
        } else {
            icon.saint_name.clone()
        },
        kind: kind.into(),
        image_url: icon.image_url.clone(),
        icon_slug: icon_slug.into(),
        prayer_slug: icon_slug.into(),
        gospel_slug: "today".into(),
        detail_href: format!("/icons/{icon_slug}"),
        current,
        feast: false,
        text_only: false,
        description,
    }
}

fn merge_calendar_days(
    mut existing: Vec<CalendarDay>,
    incoming: Vec<CalendarDay>,
) -> Vec<CalendarDay> {
    for next_day in incoming {
        if let Some(index) = existing
            .iter()
            .position(|day| calendar_day_key(day) == calendar_day_key(&next_day))
        {
            existing[index] = next_day;
        } else {
            existing.push(next_day);
        }
    }
    existing.sort_by(|a, b| calendar_day_key(a).cmp(&calendar_day_key(b)));
    existing
}

fn calendar_day_key(day: &CalendarDay) -> String {
    if !day.gregorian_date.trim().is_empty() {
        return day.gregorian_date.clone();
    }
    day.id.clone()
}

fn icon_slug_from_detail_href(href: &str) -> Option<&str> {
    href.strip_prefix("/icons/")
        .and_then(|slug| slug.split('/').next())
        .filter(|slug| !slug.trim().is_empty())
}

fn saved_day_is_managed_by_icon_date(saved_day: &CalendarDay, icons: &[IconPage]) -> bool {
    let icon_slug = if !saved_day.icon_slug.trim().is_empty() {
        Some(saved_day.icon_slug.as_str())
    } else {
        icon_slug_from_detail_href(&saved_day.detail_href)
    };

    let Some(icon_slug) = icon_slug else {
        return false;
    };

    icons.iter().any(|icon| {
        icon.calendar_date
            .as_deref()
            .is_some_and(|date| !date.trim().is_empty())
            && (icon.slug == icon_slug || icon.id == icon_slug)
    })
}

fn overlay_saved_calendar_days(
    calendar: &mut CalendarContent,
    saved_days: &[CalendarDay],
    icons: &[IconPage],
) {
    for saved_day in saved_days {
        if saved_day_is_managed_by_icon_date(saved_day, icons) {
            continue;
        }

        let Some(index) = calendar
            .days
            .iter()
            .position(|day| calendar_day_key(day) == calendar_day_key(saved_day))
        else {
            continue;
        };

        let mut merged = calendar.days[index].clone();
        merged.label = saved_day.label.clone();
        merged.note = saved_day.note.clone();
        merged.kind = saved_day.kind.clone();
        merged.image_url = saved_day.image_url.clone();
        merged.icon_slug = saved_day.icon_slug.clone();
        merged.prayer_slug = saved_day.prayer_slug.clone();
        merged.gospel_slug = saved_day.gospel_slug.clone();
        merged.detail_href = saved_day.detail_href.clone();
        merged.feast = saved_day.feast;
        merged.text_only = saved_day.text_only;
        merged.description = saved_day.description.clone();
        calendar.days[index] = merged;
    }
}

fn normalize_content_before_save_with_existing(
    mut content: IconsSiteContent,
    existing: Option<IconsSiteContent>,
) -> IconsSiteContent {
    if let Some(existing) = existing {
        content.calendar.days = merge_calendar_days(existing.calendar.days, content.calendar.days);
    }
    content
}

fn prepare_content_for_public(
    mut content: IconsSiteContent,
    year: Option<i32>,
    month: Option<u32>,
) -> IconsSiteContent {
    let today = chrono::Local::now().date_naive();
    let saved_days = content.calendar.days.clone();
    let selected_year = year
        .filter(|year| (1900..=2099).contains(year))
        .unwrap_or(today.year());
    let selected_month = month
        .filter(|month| (1..=12).contains(month))
        .unwrap_or(today.month());
    rebuild_calendar(
        &mut content.calendar,
        &content.icons,
        &content.saints,
        selected_year,
        selected_month,
        today,
    );
    overlay_saved_calendar_days(&mut content.calendar, &saved_days, &content.icons);
    overlay_icon_calendar_days(
        &mut content.calendar,
        &content.icons,
        selected_year,
        selected_month,
        today,
    );
    content
}

fn overlay_icon_calendar_days(
    calendar: &mut CalendarContent,
    icons: &[IconPage],
    year: i32,
    month: u32,
    today: NaiveDate,
) {
    for icon in icons {
        let Some(calendar_date) = icon.calendar_date.as_deref() else {
            continue;
        };
        let Ok(gregorian) = NaiveDate::parse_from_str(calendar_date.trim(), "%Y-%m-%d") else {
            continue;
        };
        if gregorian.year() == year && gregorian.month() == month {
            put_calendar_day(
                calendar,
                icon_calendar_day(icon, gregorian, gregorian == today),
                120,
            );
        }
    }
}

fn rebuild_calendar(
    calendar: &mut CalendarContent,
    icons: &[IconPage],
    saints: &[SaintPage],
    year: i32,
    month: u32,
    today: NaiveDate,
) {
    let days_in_month = days_in_gregorian_month(year, month);
    calendar.days = (1..=days_in_month)
        .map(|day| {
            empty_calendar_day(
                day,
                year,
                month,
                today.year() == year && today.month() == month && today.day() == day,
            )
        })
        .collect();

    for rule in FIXED_CALENDAR_RULES {
        let Some(gregorian) =
            julian_dates_for_gregorian_year(year, rule.julian_month, rule.julian_day)
                .into_iter()
                .find(|date| date.year() == year && date.month() == month)
        else {
            continue;
        };

        let icon_slug = icons
            .iter()
            .find(|icon| {
                calendar_rule_for_icon(icon).is_some_and(|icon_rule| std::ptr::eq(icon_rule, rule))
            })
            .map(|icon| {
                if icon.slug.is_empty() {
                    icon.id.as_str()
                } else {
                    icon.slug.as_str()
                }
            });
        put_calendar_day(
            calendar,
            fixed_calendar_day(rule, gregorian, gregorian == today, icon_slug),
            rule.priority,
        );
    }

    for saint in saints {
        let Some((julian_day, julian_month)) = parse_day_month(&saint.feast_day) else {
            continue;
        };
        for gregorian in julian_dates_for_gregorian_year(year, julian_month, julian_day) {
            if gregorian.year() == year && gregorian.month() == month {
                put_calendar_day(
                    calendar,
                    saint_calendar_day(saint, gregorian, gregorian == today),
                    40,
                );
            }
        }
    }

    let today_julian = gregorian_to_julian(today);
    let selected_today = format!(
        "{} / {} по гражданскому календарю",
        format_julian_date(today_julian),
        format_gregorian_date(today)
    );

    calendar.hero.year = year.to_string();
    calendar.hero.month_title = format!("{} {}", month_name_nominative(month), year);
    calendar.hero.prev_label = "Предыдущий год".into();
    calendar.hero.prev_href = format!("?year={}", year - 1);
    calendar.hero.next_label = "Следующий год".into();
    calendar.hero.next_href = format!("?year={}", year + 1);
    calendar.hero.today_date = selected_today;
    calendar.hero.info_primary = "Юлианский календарь".into();
    calendar.hero.info_secondary = "Даты показаны по старому стилю с гражданской привязкой.".into();

    if let Some(current_day) = calendar
        .days
        .iter()
        .find(|day| day.current && !day.text_only)
        .cloned()
        .or_else(|| calendar.days.iter().find(|day| !day.text_only).cloned())
    {
        calendar.hero.feature_title = current_day.label.clone();
        calendar.hero.feature_note = current_day.note.clone();
        calendar.hero.feature_date = current_day.description.clone();
        calendar.hero.feature_href = current_day.detail_href.clone();
        calendar.hero.icon_day_title = current_day.label;
        calendar.hero.icon_day_icon_slug = current_day.icon_slug;
        calendar.hero.icon_day_date = calendar.hero.today_date.clone();
        calendar.hero.icon_day_prayer_slug = current_day.prayer_slug;
    }
}

fn put_calendar_day(calendar: &mut CalendarContent, next_day: CalendarDay, priority: i32) {
    if let Some(index) = calendar.days.iter().position(|day| day.day == next_day.day) {
        let existing_priority = calendar_rule_for_text(&calendar.days[index].label)
            .map(|rule| rule.priority)
            .unwrap_or(if calendar.days[index].text_only {
                0
            } else {
                40
            });
        if existing_priority <= priority {
            calendar.days[index] = next_day;
        }
    }
}

fn julian_to_gregorian(year: i32, month: u32, day: u32) -> Option<NaiveDate> {
    let approximate = NaiveDate::from_ymd_opt(year, month, day)?;
    Some(approximate + Duration::days(julian_offset_days(approximate)))
}

fn julian_dates_for_gregorian_year(year: i32, month: u32, day: u32) -> Vec<NaiveDate> {
    [year - 1, year, year + 1]
        .into_iter()
        .filter_map(|julian_year| julian_to_gregorian(julian_year, month, day))
        .collect()
}

fn gregorian_to_julian(date: NaiveDate) -> NaiveDate {
    date - Duration::days(julian_offset_days(date))
}

fn julian_offset_days(date: NaiveDate) -> i64 {
    let mut year = date.year();
    if date.month() <= 2 {
        year -= 1;
    }
    (year / 100 - year / 400 - 2) as i64
}

fn days_in_gregorian_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .expect("valid month");
    (next - Duration::days(1)).day()
}

fn parse_day_month(value: &str) -> Option<(u32, u32)> {
    let normalized = normalize_lookup_text(value);
    let mut day = None;
    for token in normalized.split_whitespace() {
        if let Ok(parsed) = token.parse::<u32>() {
            day = Some(parsed);
            break;
        }
    }
    let month = MONTHS_GENITIVE
        .iter()
        .position(|month| normalized.contains(month))
        .map(|index| index as u32 + 1)?;
    day.map(|day| (day, month))
}

const MONTHS_NOMINATIVE: [&str; 12] = [
    "Январь",
    "Февраль",
    "Март",
    "Апрель",
    "Май",
    "Июнь",
    "Июль",
    "Август",
    "Сентябрь",
    "Октябрь",
    "Ноябрь",
    "Декабрь",
];

const MONTHS_GENITIVE: [&str; 12] = [
    "января",
    "февраля",
    "марта",
    "апреля",
    "мая",
    "июня",
    "июля",
    "августа",
    "сентября",
    "октября",
    "ноября",
    "декабря",
];

fn month_name_nominative(month: u32) -> &'static str {
    MONTHS_NOMINATIVE
        .get(month.saturating_sub(1) as usize)
        .copied()
        .unwrap_or("Месяц")
}

fn month_name_genitive(month: u32) -> &'static str {
    MONTHS_GENITIVE
        .get(month.saturating_sub(1) as usize)
        .copied()
        .unwrap_or("месяца")
}

fn format_julian_date(date: NaiveDate) -> String {
    format!(
        "{} {} {} (ст. ст.)",
        date.day(),
        month_name_genitive(date.month()),
        date.year()
    )
}

fn format_gregorian_date(date: NaiveDate) -> String {
    format!(
        "{} {} {}",
        date.day(),
        month_name_genitive(date.month()),
        date.year()
    )
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

pub async fn public_content(
    Query(query): Query<CalendarQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(prepare_content_for_public(
        load_content(&pool).await?,
        query.year,
        query.month,
    )))
}

pub async fn public_icons(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?.icons))
}

pub async fn public_icon(
    Path(slug): Path<String>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    load_content(&pool)
        .await?
        .icons
        .into_iter()
        .find(|item| item.slug == slug)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn public_prayers(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?.prayers))
}

pub async fn public_prayer(
    Path(slug): Path<String>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    load_content(&pool)
        .await?
        .prayers
        .into_iter()
        .find(|item| item.slug == slug)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn public_gospel_today(
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(
        load_content(&pool)
            .await?
            .gospel
            .into_iter()
            .next()
            .unwrap_or_else(|| {
                let mut content = default_content();
                content.gospel.remove(0)
            }),
    ))
}

pub async fn public_saints(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?.saints))
}

pub async fn public_saint(
    Path(slug): Path<String>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    load_content(&pool)
        .await?
        .saints
        .into_iter()
        .find(|item| item.slug == slug)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn public_churches(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?.churches))
}

pub async fn public_qr(
    Path(qr_id): Path<String>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    load_content(&pool)
        .await?
        .qr_pages
        .into_iter()
        .find(|item| item.qr_id == qr_id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn public_seo_page(
    Path(slug): Path<String>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    load_content(&pool)
        .await?
        .pages
        .into_iter()
        .find(|item| item.slug == slug)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn admin_get_content(
    Query(query): Query<CalendarQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(prepare_content_for_public(
        load_content(&pool).await?,
        query.year,
        query.month,
    )))
}

pub async fn admin_put_content(
    State(pool): State<PgPool>,
    Json(content): Json<IconsSiteContent>,
) -> Result<impl IntoResponse, StatusCode> {
    let existing = load_content(&pool).await.ok();
    let content = normalize_content_before_save_with_existing(content, existing);
    save_content(&pool, &content).await?;
    Ok(Json(prepare_content_for_public(content, None, None)))
}
