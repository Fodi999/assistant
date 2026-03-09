//! Fish seasonality handlers: fish_season, fish_season_table.

use super::shared::parse_lang;
use crate::shared::Language;
use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time;

// ── Fish & Seafood category UUID (stable in our catalog) ──────────────────────
const FISH_CATEGORY_ID: &str = "503794cf-37e0-48c1-a6d8-b5c3f21e03a1";

// ── Shared month helpers ──────────────────────────────────────────────────────

pub fn month_name(m: u8, lang: Language) -> &'static str {
    match lang {
        Language::Ru => match m {
            1=>"Январь",2=>"Февраль",3=>"Март",4=>"Апрель",5=>"Май",6=>"Июнь",
            7=>"Июль",8=>"Август",9=>"Сентябрь",10=>"Октябрь",11=>"Ноябрь",12=>"Декабрь",_=>"—",
        },
        Language::Pl => match m {
            1=>"Styczeń",2=>"Luty",3=>"Marzec",4=>"Kwiecień",5=>"Maj",6=>"Czerwiec",
            7=>"Lipiec",8=>"Sierpień",9=>"Wrzesień",10=>"Październik",11=>"Listopad",12=>"Grudzień",_=>"—",
        },
        Language::Uk => match m {
            1=>"Січень",2=>"Лютий",3=>"Березень",4=>"Квітень",5=>"Травень",6=>"Червень",
            7=>"Липень",8=>"Серпень",9=>"Вересень",10=>"Жовтень",11=>"Листопад",12=>"Грудень",_=>"—",
        },
        Language::En => match m {
            1=>"January",2=>"February",3=>"March",4=>"April",5=>"May",6=>"June",
            7=>"July",8=>"August",9=>"September",10=>"October",11=>"November",12=>"December",_=>"—",
        },
    }
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct FishQuery {
    #[serde(default = "default_fish")]
    pub fish: String,
    pub lang: Option<String>,
}
fn default_fish() -> String { "salmon".to_string() }

#[derive(Serialize)]
pub struct FishSeasonEntry {
    pub month:      u8,
    pub month_name: String,
    pub available:  bool,
}

#[derive(Serialize)]
pub struct FishSeasonResponse {
    pub fish:   String,
    pub season: Vec<FishSeasonEntry>,
}

#[derive(Serialize)]
pub struct FishSeasonTableItem {
    pub slug:        String,
    pub name:        String,
    pub name_en:     String,
    pub name_ru:     String,
    pub name_pl:     String,
    pub name_uk:     String,
    pub image_url:   Option<String>,
    pub season:      Vec<FishSeasonEntry>,
    pub status:      String,
    pub water_type:  Option<String>,
    pub wild_farmed: Option<String>,
    pub sushi_grade: Option<bool>,
}

#[derive(Serialize)]
pub struct AllYearItem {
    pub slug:      String,
    pub name:      String,
    pub image_url: Option<String>,
}

#[derive(Serialize)]
pub struct FishSeasonTableResponse {
    pub fish:          Vec<FishSeasonTableItem>,
    pub all_year:      Vec<AllYearItem>,
    pub lang:          String,
    pub region:        String,
    pub note_all_year: String,
}

#[derive(Deserialize)]
pub struct FishTableQuery {
    pub lang:   Option<String>,
    pub region: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /public/tools/fish-season?fish=salmon&lang=ru
pub async fn fish_season(
    State(pool): State<PgPool>,
    Query(params): Query<FishQuery>,
) -> Json<FishSeasonResponse> {
    let lang       = parse_lang(&params.lang);
    let fish_lower = params.fish.to_lowercase();

    let row: Option<(Vec<bool>,)> = sqlx::query_as(
        r#"SELECT availability_months
           FROM catalog_ingredients
           WHERE is_active = true AND slug = $1
             AND availability_months IS NOT NULL"#,
    )
    .bind(&fish_lower)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    let months = row.map(|(m,)| m).unwrap_or_else(|| vec![true; 12]);

    let season = (1u8..=12)
        .map(|m| FishSeasonEntry {
            month: m,
            month_name: month_name(m, lang).to_string(),
            available: months.get((m - 1) as usize).copied().unwrap_or(true),
        })
        .collect();

    Json(FishSeasonResponse { fish: params.fish, season })
}

/// GET /public/tools/fish-season-table?lang=ru&region=PL
pub async fn fish_season_table(
    State(pool): State<PgPool>,
    Query(params): Query<FishTableQuery>,
) -> Json<FishSeasonTableResponse> {
    let lang       = parse_lang(&params.lang);
    let region     = params.region.clone().unwrap_or_else(|| "PL".to_string());
    let lang_code  = match lang {
        Language::Ru => "ru", Language::Pl => "pl",
        Language::Uk => "uk", Language::En => "en",
    };

    let now       = time::OffsetDateTime::now_utc();
    let cur_month = now.month() as i16;

    // ── All-year products ─────────────────────────────────────────────────────
    #[derive(sqlx::FromRow)]
    struct AllYearRow {
        slug:      Option<String>,
        name_en:   String,
        name_ru:   String,
        name_pl:   String,
        name_uk:   String,
        image_url: Option<String>,
    }

    let all_year_rows: Vec<AllYearRow> = sqlx::query_as(
        r#"SELECT slug, name_en, name_ru, name_pl, name_uk, image_url
           FROM catalog_ingredients
           WHERE is_active = true
             AND category_id = $1::uuid
             AND availability_model = 'all_year'
           ORDER BY name_en"#,
    )
    .bind(FISH_CATEGORY_ID)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let all_year: Vec<AllYearItem> = all_year_rows.iter().map(|r| {
        let name = match lang {
            Language::Ru => r.name_ru.clone(), Language::Pl => r.name_pl.clone(),
            Language::Uk => r.name_uk.clone(), Language::En => r.name_en.clone(),
        };
        AllYearItem { slug: r.slug.clone().unwrap_or_default(), name, image_url: r.image_url.clone() }
    }).collect();

    // ── Seasonal products ─────────────────────────────────────────────────────
    #[derive(sqlx::FromRow)]
    struct SeasonRow {
        slug:        Option<String>,
        name_en:     String,
        name_ru:     String,
        name_pl:     String,
        name_uk:     String,
        image_url:   Option<String>,
        water_type:  Option<String>,
        wild_farmed: Option<String>,
        sushi_grade: Option<bool>,
    }

    #[derive(sqlx::FromRow)]
    struct MonthRow { product_id: uuid::Uuid, month: i16, status: String }

    let season_rows: Vec<SeasonRow> = sqlx::query_as(
        r#"SELECT DISTINCT ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk,
                  ci.image_url, ci.water_type, ci.wild_farmed, ci.sushi_grade
           FROM catalog_ingredients ci
           JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
           WHERE ci.is_active = true
             AND ci.category_id = $1::uuid
             AND ci.availability_model != 'all_year'
             AND cps.region_code = $2
           ORDER BY ci.name_en"#,
    )
    .bind(FISH_CATEGORY_ID)
    .bind(&region)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let month_rows: Vec<MonthRow> = sqlx::query_as(
        r#"SELECT cps.product_id, cps.month, cps.status
           FROM catalog_product_seasonality cps
           JOIN catalog_ingredients ci ON ci.id = cps.product_id
           WHERE ci.is_active = true
             AND ci.category_id = $1::uuid
             AND ci.availability_model != 'all_year'
             AND cps.region_code = $2
           ORDER BY cps.product_id, cps.month"#,
    )
    .bind(FISH_CATEGORY_ID)
    .bind(&region)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let prod_ids: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        r#"SELECT DISTINCT ci.id, ci.slug
           FROM catalog_ingredients ci
           JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
           WHERE ci.is_active = true AND ci.category_id = $1::uuid
             AND ci.availability_model != 'all_year' AND cps.region_code = $2"#,
    )
    .bind(FISH_CATEGORY_ID)
    .bind(&region)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let fish: Vec<FishSeasonTableItem> = season_rows.iter().map(|r| {
        let slug = r.slug.clone().unwrap_or_default();
        let pid  = prod_ids.iter().find(|(_, s)| s == &slug).map(|(id, _)| *id);

        let name = match lang {
            Language::Ru => r.name_ru.clone(), Language::Pl => r.name_pl.clone(),
            Language::Uk => r.name_uk.clone(), Language::En => r.name_en.clone(),
        };

        let cur_status = pid
            .and_then(|id| month_rows.iter().find(|m| m.product_id == id && m.month == cur_month))
            .map(|m| m.status.clone())
            .unwrap_or_else(|| "off".to_string());

        let season: Vec<FishSeasonEntry> = (1u8..=12).map(|m| {
            let st = pid
                .and_then(|id| month_rows.iter().find(|r| r.product_id == id && r.month == m as i16))
                .map(|r| r.status.clone())
                .unwrap_or_else(|| "off".to_string());
            FishSeasonEntry {
                month: m,
                month_name: month_name(m, lang).to_string(),
                available: st != "off",
            }
        }).collect();

        FishSeasonTableItem {
            slug, name,
            name_en:     r.name_en.clone(),
            name_ru:     r.name_ru.clone(),
            name_pl:     r.name_pl.clone(),
            name_uk:     r.name_uk.clone(),
            image_url:   r.image_url.clone(),
            season,
            status:      cur_status,
            water_type:  r.water_type.clone(),
            wild_farmed: r.wild_farmed.clone(),
            sushi_grade: r.sushi_grade,
        }
    }).collect();

    let note_all_year = match lang {
        Language::Ru => "Доступны круглый год — не привязаны к сезону",
        Language::Pl => "Dostępne przez cały rok — nie sezonowe",
        Language::Uk => "Доступні цілий рік — не сезонні",
        Language::En => "Available all year — not seasonal",
    };

    Json(FishSeasonTableResponse {
        fish,
        all_year,
        lang:          lang_code.to_string(),
        region,
        note_all_year: note_all_year.to_string(),
    })
}
