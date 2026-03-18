//! Seasonality & recipe handlers:
//! seasonal_calendar, in_season_now, product_seasonality, best_in_season,
//! products_by_month, product_search, recipe_nutrition, recipe_cost,
//! best_right_now, list_regions.

use super::fish::month_name;
use super::shared::parse_lang;
use crate::domain::tools::catalog_row::CatalogNutritionRow;
use crate::domain::tools::unit_converter as uc;
use crate::shared::Language;
use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time;

// ── Shared season entry (re-used across multiple responses) ───────────────────

use super::fish::FishSeasonEntry;

// ── Shared DB row types ───────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct SeasonalProductRow {
    slug:      Option<String>,
    name_en:   String,
    name_ru:   String,
    name_pl:   String,
    name_uk:   String,
    image_url: Option<String>,
}

#[derive(sqlx::FromRow)]
struct SeasonalityRow {
    product_id: uuid::Uuid,
    month:      i16,
    status:     String,
}

// ── Shared BestInSeasonItem (reused by best_in_season + best_right_now) ───────

#[derive(Serialize)]
pub struct BestInSeasonItem {
    pub slug:        String,
    pub name:        String,
    pub image_url:   Option<String>,
    pub status:      String,
    pub water_type:  Option<String>,
    pub wild_farmed: Option<String>,
    pub sushi_grade: Option<bool>,
}

// ── 1. Seasonal Calendar ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SeasonalCalendarQuery {
    pub r#type: Option<String>,
    pub lang:   Option<String>,
    pub region: Option<String>,
}

#[derive(Serialize)]
pub struct SeasonalCalendarProduct {
    pub slug:      String,
    pub name:      String,
    pub image_url: Option<String>,
    pub season:    Vec<FishSeasonEntry>,
}

#[derive(Serialize)]
pub struct SeasonalCalendarResponse {
    pub product_type: String,
    pub lang:         String,
    pub region:       String,
    pub products:     Vec<SeasonalCalendarProduct>,
}

/// GET /public/tools/seasonal-calendar?type=seafood&lang=ru&region=GLOBAL
pub async fn seasonal_calendar(
    State(pool): State<PgPool>,
    Query(params): Query<SeasonalCalendarQuery>,
) -> Json<SeasonalCalendarResponse> {
    let lang         = parse_lang(&params.lang);
    let product_type = params.r#type.clone().unwrap_or_else(|| "seafood".to_string());
    let region       = params.region.clone().unwrap_or_else(|| "PL".to_string());
    let lang_code    = lang_code(lang);

    let products: Vec<SeasonalProductRow> = sqlx::query_as(
        r#"SELECT DISTINCT ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk, ci.image_url
           FROM catalog_ingredients ci
           JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
           WHERE ci.is_active = true AND ci.is_published = true AND ci.product_type = $1 AND cps.region_code = $2
           ORDER BY ci.name_en"#,
    )
    .bind(&product_type)
    .bind(&region)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let seasonality: Vec<SeasonalityRow> = sqlx::query_as(
        r#"SELECT cps.product_id, cps.month, cps.status
           FROM catalog_product_seasonality cps
           JOIN catalog_ingredients ci ON ci.id = cps.product_id
           WHERE ci.is_active = true AND ci.is_published = true AND ci.product_type = $1 AND cps.region_code = $2
           ORDER BY cps.product_id, cps.month"#,
    )
    .bind(&product_type)
    .bind(&region)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let product_ids: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        r#"SELECT DISTINCT ci.id, ci.slug
           FROM catalog_ingredients ci
           JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
           WHERE ci.is_active = true AND ci.is_published = true AND ci.product_type = $1 AND cps.region_code = $2"#,
    )
    .bind(&product_type)
    .bind(&region)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let result = products.iter().map(|p| {
        let slug       = p.slug.clone().unwrap_or_default();
        let product_id = product_ids.iter().find(|(_, s)| s.as_str() == slug.as_str()).map(|(id, _)| *id);
        let name       = localized(&p.name_en, &p.name_ru, &p.name_pl, &p.name_uk, lang);

        let season = build_season(product_id, &seasonality, lang);

        SeasonalCalendarProduct { slug, name, image_url: p.image_url.clone(), season }
    }).collect();

    Json(SeasonalCalendarResponse { product_type, lang: lang_code.to_string(), region, products: result })
}

// ── 2. In season now ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct InSeasonItem {
    pub slug:      String,
    pub name:      String,
    pub image_url: Option<String>,
    pub status:    String,
}

#[derive(Serialize)]
pub struct InSeasonNowResponse {
    pub product_type: String,
    pub month:        u8,
    pub lang:         String,
    pub region:       String,
    pub items:        Vec<InSeasonItem>,
}

/// GET /public/tools/in-season-now?type=seafood&lang=ru
pub async fn in_season_now(
    State(pool): State<PgPool>,
    Query(params): Query<SeasonalCalendarQuery>,
) -> Json<InSeasonNowResponse> {
    let lang         = parse_lang(&params.lang);
    let product_type = params.r#type.clone().unwrap_or_else(|| "seafood".to_string());
    let region       = params.region.clone().unwrap_or_else(|| "PL".to_string());
    let month        = time::OffsetDateTime::now_utc().month() as u8;

    #[derive(sqlx::FromRow)]
    struct Row {
        slug:      Option<String>,
        name_en:   String,
        name_ru:   String,
        name_pl:   String,
        name_uk:   String,
        image_url: Option<String>,
        status:    String,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"SELECT ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk, ci.image_url, cps.status
           FROM catalog_ingredients ci
           JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
           WHERE ci.is_active = true AND ci.is_published = true AND ci.product_type = $1 AND cps.region_code = $2
             AND cps.month = $3 AND cps.status != 'off'
           ORDER BY CASE cps.status WHEN 'peak' THEN 1 WHEN 'good' THEN 2 WHEN 'limited' THEN 3 ELSE 4 END, ci.name_en"#,
    )
    .bind(&product_type)
    .bind(&region)
    .bind(month as i16)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let items = rows.iter().map(|r| InSeasonItem {
        slug:      r.slug.clone().unwrap_or_default(),
        name:      localized(&r.name_en, &r.name_ru, &r.name_pl, &r.name_uk, lang),
        image_url: r.image_url.clone(),
        status:    r.status.clone(),
    }).collect();

    Json(InSeasonNowResponse { product_type, month, lang: lang_code(lang).to_string(), region, items })
}

// ── 3. Product seasonality detail ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ProductSeasonalityQuery {
    pub slug:   String,
    pub lang:   Option<String>,
    pub region: Option<String>,
}

#[derive(Serialize)]
pub struct ProductSeasonalityEntry {
    pub month:      u8,
    pub month_name: String,
    pub status:     String,
    pub available:  bool,
    pub note:       Option<String>,
}

#[derive(Serialize)]
pub struct ProductSeasonalityResponse {
    pub slug:         String,
    pub name:         String,
    pub product_type: String,
    pub image_url:    Option<String>,
    pub region:       String,
    pub lang:         String,
    pub season:       Vec<ProductSeasonalityEntry>,
}

/// GET /public/tools/product-seasonality?slug=salmon&lang=ru
pub async fn product_seasonality(
    State(pool): State<PgPool>,
    Query(params): Query<ProductSeasonalityQuery>,
) -> Json<ProductSeasonalityResponse> {
    let lang   = parse_lang(&params.lang);
    let region = params.region.clone().unwrap_or_else(|| "PL".to_string());

    #[derive(sqlx::FromRow)]
    struct ProdRow {
        id:           uuid::Uuid,
        name_en:      String,
        name_ru:      String,
        name_pl:      String,
        name_uk:      String,
        image_url:    Option<String>,
        product_type: Option<String>,
    }

    let prod: Option<ProdRow> = sqlx::query_as(
        r#"SELECT id, name_en, name_ru, name_pl, name_uk, image_url, product_type
           FROM catalog_ingredients WHERE is_active = true AND is_published = true AND slug = $1"#,
    )
    .bind(&params.slug)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    let (prod_id, name, product_type, image_url) = if let Some(ref p) = prod {
        (
            Some(p.id),
            localized(&p.name_en, &p.name_ru, &p.name_pl, &p.name_uk, lang),
            p.product_type.clone().unwrap_or_else(|| "other".to_string()),
            p.image_url.clone(),
        )
    } else {
        (None, params.slug.clone(), "other".to_string(), None)
    };

    #[derive(sqlx::FromRow)]
    struct SRow { month: i16, status: String, note: Option<String> }

    let srows: Vec<SRow> = if let Some(pid) = prod_id {
        sqlx::query_as(
            r#"SELECT month, status, note FROM catalog_product_seasonality
               WHERE product_id = $1 AND region_code = $2 ORDER BY month"#,
        )
        .bind(pid)
        .bind(&region)
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    let season = (1u8..=12).map(|m| {
        let row    = srows.iter().find(|r| r.month == m as i16);
        let status = row.map(|r| r.status.clone()).unwrap_or_else(|| "off".to_string());
        let note   = row.and_then(|r| r.note.clone());
        ProductSeasonalityEntry {
            month:      m,
            month_name: month_name(m, lang).to_string(),
            available:  status != "off",
            status,
            note,
        }
    }).collect();

    Json(ProductSeasonalityResponse {
        slug: params.slug, name, product_type, image_url,
        region, lang: lang_code(lang).to_string(), season,
    })
}

// ── 4. Best in season ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BestInSeasonQuery {
    pub r#type:      Option<String>,
    pub month:       Option<u8>,
    pub lang:        Option<String>,
    pub region:      Option<String>,
    pub peak_only:   Option<bool>,
    pub water_type:  Option<String>,
    pub wild_farmed: Option<String>,
    pub sushi:       Option<bool>,
}

#[derive(Serialize)]
pub struct BestInSeasonResponse {
    pub product_type: String,
    pub month:        u8,
    pub lang:         String,
    pub region:       String,
    pub items:        Vec<BestInSeasonItem>,
}

/// GET /public/tools/best-in-season?type=seafood&month=8&lang=ru&peak_only=true&water_type=sea&sushi=true
pub async fn best_in_season(
    State(pool): State<PgPool>,
    Query(params): Query<BestInSeasonQuery>,
) -> Json<BestInSeasonResponse> {
    let lang         = parse_lang(&params.lang);
    let product_type = params.r#type.clone().unwrap_or_else(|| "seafood".to_string());
    let region       = params.region.clone().unwrap_or_else(|| "PL".to_string());
    let peak_only    = params.peak_only.unwrap_or(false);
    let month: u8    = params.month.unwrap_or_else(|| time::OffsetDateTime::now_utc().month() as u8);

    let status_filter = if peak_only { "cps.status = 'peak'" } else { "cps.status IN ('peak','good')" };

    let sql = format!(
        r#"SELECT ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk,
                  ci.image_url, cps.status, ci.water_type, ci.wild_farmed, ci.sushi_grade
           FROM catalog_ingredients ci
           JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
           WHERE ci.is_active = true AND ci.is_published = true AND ci.product_type = $1 AND cps.region_code = $2
             AND cps.month = $3 AND {status_filter}
             AND ($4::text IS NULL OR ci.water_type = $4)
             AND ($5::text IS NULL OR ci.wild_farmed = $5)
             AND ($6::boolean IS NULL OR ci.sushi_grade = $6)
           ORDER BY CASE cps.status WHEN 'peak' THEN 1 WHEN 'good' THEN 2 ELSE 3 END, ci.name_en"#
    );

    #[derive(sqlx::FromRow)]
    struct Row {
        slug: Option<String>, name_en: String, name_ru: String, name_pl: String, name_uk: String,
        image_url: Option<String>, status: String, water_type: Option<String>,
        wild_farmed: Option<String>, sushi_grade: Option<bool>,
    }

    let rows: Vec<Row> = sqlx::query_as(&sql)
        .bind(&product_type).bind(&region).bind(month as i16)
        .bind(params.water_type.as_deref()).bind(params.wild_farmed.as_deref()).bind(params.sushi)
        .fetch_all(&pool).await.unwrap_or_default();

    let items = rows.iter().map(|r| BestInSeasonItem {
        slug:        r.slug.clone().unwrap_or_default(),
        name:        localized(&r.name_en, &r.name_ru, &r.name_pl, &r.name_uk, lang),
        image_url:   r.image_url.clone(),
        status:      r.status.clone(),
        water_type:  r.water_type.clone(),
        wild_farmed: r.wild_farmed.clone(),
        sushi_grade: r.sushi_grade,
    }).collect();

    Json(BestInSeasonResponse { product_type, month, lang: lang_code(lang).to_string(), region, items })
}

// ── 5. Products by month ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ProductsByMonthQuery {
    pub month:  Option<u8>,
    pub r#type: Option<String>,
    pub lang:   Option<String>,
    pub region: Option<String>,
}

#[derive(Serialize)]
pub struct ProductsByMonthItem {
    pub slug:      String,
    pub name:      String,
    pub image_url: Option<String>,
    pub status:    String,
}

#[derive(Serialize)]
pub struct ProductsByMonthResponse {
    pub month:        u8,
    pub month_name:   String,
    pub product_type: Option<String>,
    pub lang:         String,
    pub region:       String,
    pub items:        Vec<ProductsByMonthItem>,
}

/// GET /public/tools/products-by-month?month=7&type=vegetable&lang=ru
pub async fn products_by_month(
    State(pool): State<PgPool>,
    Query(params): Query<ProductsByMonthQuery>,
) -> Json<ProductsByMonthResponse> {
    let lang   = parse_lang(&params.lang);
    let region = params.region.clone().unwrap_or_else(|| "PL".to_string());
    let month: u8 = params.month.unwrap_or_else(|| time::OffsetDateTime::now_utc().month() as u8);

    let order = "ORDER BY CASE cps.status WHEN 'peak' THEN 1 WHEN 'good' THEN 2 WHEN 'limited' THEN 3 ELSE 4 END";

    #[derive(sqlx::FromRow)]
    struct Row {
        slug: Option<String>, name_en: String, name_ru: String,
        name_pl: String, name_uk: String, image_url: Option<String>, status: String,
    }

    let rows: Vec<Row> = match &params.r#type {
        Some(ptype) => sqlx::query_as(
            &format!(
                r#"SELECT ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk, ci.image_url, cps.status
                   FROM catalog_ingredients ci
                   JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
                   WHERE ci.is_active = true AND ci.is_published = true AND ci.product_type = $1 AND cps.region_code = $2
                     AND cps.month = $3 AND cps.status != 'off'
                   {order}, ci.name_en"#
            ),
        )
        .bind(ptype).bind(&region).bind(month as i16)
        .fetch_all(&pool).await.unwrap_or_default(),

        None => sqlx::query_as(
            &format!(
                r#"SELECT ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk, ci.image_url, cps.status
                   FROM catalog_ingredients ci
                   JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
                   WHERE ci.is_active = true AND ci.is_published = true AND cps.region_code = $1
                     AND cps.month = $2 AND cps.status != 'off'
                   {order}, ci.product_type, ci.name_en"#
            ),
        )
        .bind(&region).bind(month as i16)
        .fetch_all(&pool).await.unwrap_or_default(),
    };

    let items = rows.iter().map(|r| ProductsByMonthItem {
        slug:      r.slug.clone().unwrap_or_default(),
        name:      localized(&r.name_en, &r.name_ru, &r.name_pl, &r.name_uk, lang),
        image_url: r.image_url.clone(),
        status:    r.status.clone(),
    }).collect();

    Json(ProductsByMonthResponse {
        month,
        month_name:   month_name(month, lang).to_string(),
        product_type: params.r#type,
        lang:         lang_code(lang).to_string(),
        region,
        items,
    })
}

// ── 6. Product search ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ProductSearchQuery {
    pub q:     String,
    pub lang:  Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct ProductSearchItem {
    pub slug:               String,
    pub name:               String,
    pub name_en:            String,
    pub product_type:       Option<String>,
    pub image_url:          Option<String>,
    pub availability_model: Option<String>,
}

#[derive(Serialize)]
pub struct ProductSearchResponse {
    pub query:   String,
    pub results: Vec<ProductSearchItem>,
}

/// GET /public/tools/product-search?q=tom&lang=ru&limit=10
pub async fn product_search(
    State(pool): State<PgPool>,
    Query(params): Query<ProductSearchQuery>,
) -> Json<ProductSearchResponse> {
    let lang  = parse_lang(&params.lang);
    let q     = params.q.to_lowercase();
    let limit = params.limit.unwrap_or(10).min(50);

    #[derive(sqlx::FromRow)]
    struct Row {
        slug: Option<String>, name_en: String, name_ru: String, name_pl: String, name_uk: String,
        image_url: Option<String>, product_type: Option<String>, availability_model: Option<String>,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"SELECT slug, name_en, name_ru, name_pl, name_uk, image_url, product_type, availability_model
           FROM catalog_ingredients
           WHERE is_active = true AND is_published = true
             AND (LOWER(name_en) LIKE '%' || $1 || '%' OR LOWER(name_ru) LIKE '%' || $1 || '%'
                  OR LOWER(name_pl) LIKE '%' || $1 || '%' OR LOWER(name_uk) LIKE '%' || $1 || '%'
                  OR slug LIKE '%' || $1 || '%')
           ORDER BY CASE WHEN LOWER(name_en) = $1 OR slug = $1 THEN 0 WHEN LOWER(name_en) LIKE $1 || '%' THEN 1 ELSE 2 END, name_en
           LIMIT $2"#,
    )
    .bind(&q).bind(limit)
    .fetch_all(&pool).await.unwrap_or_default();

    let results = rows.iter().map(|r| ProductSearchItem {
        slug:               r.slug.clone().unwrap_or_default(),
        name:               localized(&r.name_en, &r.name_ru, &r.name_pl, &r.name_uk, lang),
        name_en:            r.name_en.clone(),
        product_type:       r.product_type.clone(),
        image_url:          r.image_url.clone(),
        availability_model: r.availability_model.clone(),
    }).collect();

    Json(ProductSearchResponse { query: params.q, results })
}

// ── 7. Recipe nutrition ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RecipeIngredientInput {
    pub name:   String,
    pub amount: f64,
    pub unit:   Option<String>,
}

#[derive(Deserialize)]
pub struct RecipeNutritionRequest {
    pub ingredients: Vec<RecipeIngredientInput>,
    pub lang:        Option<String>,
    pub portions:    Option<f64>,
}

#[derive(Serialize)]
pub struct RecipeIngredientNutrition {
    pub name:           String,
    pub localized_name: String,
    pub slug:           Option<String>,
    pub amount_g:       f64,
    pub calories:       f64,
    pub protein_g:      f64,
    pub fat_g:          f64,
    pub carbs_g:        f64,
    pub found:          bool,
}

#[derive(Serialize)]
pub struct RecipeTotals {
    pub calories:  f64,
    pub protein_g: f64,
    pub fat_g:     f64,
    pub carbs_g:   f64,
    pub weight_g:  f64,
}

#[derive(Serialize)]
pub struct RecipeNutritionResponse {
    pub ingredients: Vec<RecipeIngredientNutrition>,
    pub total:       RecipeTotals,
    pub per_portion: Option<RecipeTotals>,
    pub portions:    Option<f64>,
}

/// POST /public/tools/recipe-nutrition
pub async fn recipe_nutrition(
    State(pool): State<PgPool>,
    Json(body): Json<RecipeNutritionRequest>,
) -> Json<RecipeNutritionResponse> {
    let lang     = parse_lang(&body.lang);
    let portions = body.portions;

    let db_rows: Vec<CatalogNutritionRow> = load_all_catalog(&pool).await;

    let find = |name: &str| find_row(&db_rows, name);

    let mut items: Vec<RecipeIngredientNutrition> = Vec::new();
    let (mut tot_cal, mut tot_pro, mut tot_fat, mut tot_car, mut tot_wgt) =
        (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);

    for ing in &body.ingredients {
        let unit    = ing.unit.as_deref().unwrap_or("g");
        let db      = find(&ing.name);
        let density = db.map(|r| r.density()).unwrap_or(1.0);
        let amount_g = to_grams(ing.amount, unit, density);
        let f = amount_g / 100.0;
        let (cal, pro, fat, car) = db.map(|r| (r.cal(), r.prot(), r.fat(), r.carbs())).unwrap_or_default();

        tot_cal += cal * f; tot_pro += pro * f; tot_fat += fat * f;
        tot_car += car * f; tot_wgt += amount_g;

        items.push(RecipeIngredientNutrition {
            name:           ing.name.clone(),
            localized_name: db.map(|r| r.localized_name(lang).to_string()).unwrap_or_else(|| ing.name.clone()),
            slug:           db.and_then(|r| r.slug.clone()),
            amount_g:       uc::round_to(amount_g, 1),
            calories:       uc::round_to(cal * f, 1),
            protein_g:      uc::round_to(pro * f, 1),
            fat_g:          uc::round_to(fat * f, 1),
            carbs_g:        uc::round_to(car * f, 1),
            found:          db.is_some(),
        });
    }

    let total = RecipeTotals {
        calories: uc::round_to(tot_cal, 1), protein_g: uc::round_to(tot_pro, 1),
        fat_g:    uc::round_to(tot_fat, 1), carbs_g:   uc::round_to(tot_car, 1),
        weight_g: uc::round_to(tot_wgt, 1),
    };

    let per_portion = portions.filter(|&p| p > 0.0).map(|p| RecipeTotals {
        calories: uc::round_to(tot_cal / p, 1), protein_g: uc::round_to(tot_pro / p, 1),
        fat_g:    uc::round_to(tot_fat / p, 1), carbs_g:   uc::round_to(tot_car / p, 1),
        weight_g: uc::round_to(tot_wgt / p, 1),
    });

    Json(RecipeNutritionResponse { ingredients: items, total, per_portion, portions })
}

// ── 8. Recipe cost ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RecipeCostIngredient {
    pub name:         String,
    pub amount:       f64,
    pub unit:         Option<String>,
    pub price_per_kg: Option<f64>,
}

#[derive(Deserialize)]
pub struct RecipeCostRequest {
    pub ingredients:   Vec<RecipeCostIngredient>,
    pub portions:      Option<f64>,
    pub target_margin: Option<f64>,
    pub lang:          Option<String>,
}

#[derive(Serialize)]
pub struct RecipeCostItem {
    pub name:           String,
    pub localized_name: String,
    pub amount_g:       f64,
    pub price_per_kg:   Option<f64>,
    pub cost:           Option<f64>,
}

#[derive(Serialize)]
pub struct RecipeCostResponse {
    pub ingredients:          Vec<RecipeCostItem>,
    pub total_cost:           Option<f64>,
    pub cost_per_portion:     Option<f64>,
    pub portions:             Option<f64>,
    pub suggested_menu_price: Option<f64>,
    pub food_cost_percent:    Option<f64>,
    pub note:                 String,
}

/// POST /public/tools/recipe-cost
pub async fn recipe_cost(
    State(pool): State<PgPool>,
    Json(body): Json<RecipeCostRequest>,
) -> Json<RecipeCostResponse> {
    let lang          = parse_lang(&body.lang);
    let portions      = body.portions;
    let target_margin = body.target_margin.unwrap_or(70.0);

    let db_rows: Vec<CatalogNutritionRow> = load_all_catalog(&pool).await;
    let find = |name: &str| find_row(&db_rows, name);

    let mut items: Vec<RecipeCostItem>  = Vec::new();
    let mut total_cost_sum = 0.0_f64;
    let mut all_have_price = true;

    for ing in &body.ingredients {
        let unit     = ing.unit.as_deref().unwrap_or("g");
        let db       = find(&ing.name);
        let density  = db.map(|r| r.density()).unwrap_or(1.0);
        let amount_g = to_grams(ing.amount, unit, density);

        let cost = ing.price_per_kg.map(|ppkg| uc::round_to((amount_g / 1000.0) * ppkg, 4));
        if cost.is_none() { all_have_price = false; }
        if let Some(c) = cost { total_cost_sum += c; }

        items.push(RecipeCostItem {
            name:           ing.name.clone(),
            localized_name: db.map(|r| r.localized_name(lang).to_string()).unwrap_or_else(|| ing.name.clone()),
            amount_g:       uc::round_to(amount_g, 1),
            price_per_kg:   ing.price_per_kg,
            cost,
        });
    }

    let total_cost        = all_have_price.then(|| uc::round_to(total_cost_sum, 2));
    let cost_per_portion  = total_cost.zip(portions).filter(|(_, p)| *p > 0.0)
                                .map(|(tc, p)| uc::round_to(tc / p, 2));
    let suggested_menu_price = cost_per_portion.map(|cpp| {
        let m = target_margin.clamp(1.0, 99.0) / 100.0;
        uc::round_to(cpp / (1.0 - m), 2)
    });
    let food_cost_percent = cost_per_portion.zip(suggested_menu_price).map(|(cpp, smp)| {
        if smp > 0.0 { uc::round_to((cpp / smp) * 100.0, 1) } else { 0.0 }
    });

    let note = if !all_have_price {
        "Some ingredients are missing price_per_kg — provide prices for full cost calculation.".to_string()
    } else {
        format!("Cost calculated at {target_margin:.0}% target margin.")
    };

    Json(RecipeCostResponse { ingredients: items, total_cost, cost_per_portion, portions, suggested_menu_price, food_cost_percent, note })
}

// ── 9. Best right now ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BestRightNowQuery {
    pub r#type:      Option<String>,
    pub lang:        Option<String>,
    pub region:      Option<String>,
    pub water_type:  Option<String>,
    pub wild_farmed: Option<String>,
    pub sushi:       Option<bool>,
}

#[derive(Serialize)]
pub struct BestRightNowResponse {
    pub headline:     String,
    pub month:        u8,
    pub month_name:   String,
    pub product_type: String,
    pub region:       String,
    pub lang:         String,
    pub peak:         Vec<BestInSeasonItem>,
    pub also_good:    Vec<BestInSeasonItem>,
}

/// GET /public/tools/best-right-now?type=seafood&lang=ru&region=PL
pub async fn best_right_now(
    State(pool): State<PgPool>,
    Query(params): Query<BestRightNowQuery>,
) -> Json<BestRightNowResponse> {
    let lang         = parse_lang(&params.lang);
    let product_type = params.r#type.clone().unwrap_or_else(|| "seafood".to_string());
    let region       = params.region.clone().unwrap_or_else(|| "PL".to_string());
    let now          = time::OffsetDateTime::now_utc();
    let month        = now.month() as u8;
    let mn           = month_name(month, lang);

    let type_label = match lang {
        Language::Ru => match product_type.as_str() {
            "seafood"=>"рыба и морепродукты","vegetable"=>"овощи","fruit"=>"фрукты","meat"=>"мясо",_=>product_type.as_str(),
        },
        Language::Pl => match product_type.as_str() {
            "seafood"=>"ryby i owoce morza","vegetable"=>"warzywa","fruit"=>"owoce","meat"=>"mięso",_=>product_type.as_str(),
        },
        Language::Uk => match product_type.as_str() {
            "seafood"=>"риба та морепродукти","vegetable"=>"овочі","fruit"=>"фрукти","meat"=>"мʼясо",_=>product_type.as_str(),
        },
        Language::En => match product_type.as_str() {
            "seafood"=>"fish & seafood","vegetable"=>"vegetables","fruit"=>"fruits","meat"=>"meat",_=>product_type.as_str(),
        },
    };

    let headline = match lang {
        Language::Ru => format!("🔥 Лучшие {} в {}", type_label, mn),
        Language::Pl => format!("🔥 Najlepsze {} w {}", type_label, mn),
        Language::Uk => format!("🔥 Найкращі {} у {}", type_label, mn),
        Language::En => format!("🔥 Best {} in {}", type_label, mn),
    };

    #[derive(sqlx::FromRow)]
    struct Row {
        slug: Option<String>, name_en: String, name_ru: String, name_pl: String, name_uk: String,
        image_url: Option<String>, status: String, water_type: Option<String>,
        wild_farmed: Option<String>, sushi_grade: Option<bool>,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"SELECT ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk, ci.image_url,
                  cps.status, ci.water_type, ci.wild_farmed, ci.sushi_grade
           FROM catalog_ingredients ci
           JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
           WHERE ci.is_active = true AND ci.is_published = true AND ci.product_type = $1 AND cps.region_code = $2
             AND cps.month = $3 AND cps.status IN ('peak','good')
             AND ($4::text IS NULL OR ci.water_type = $4)
             AND ($5::text IS NULL OR ci.wild_farmed = $5)
             AND ($6::boolean IS NULL OR ci.sushi_grade = $6)
           ORDER BY CASE cps.status WHEN 'peak' THEN 1 ELSE 2 END, ci.name_en"#,
    )
    .bind(&product_type).bind(&region).bind(month as i16)
    .bind(params.water_type.as_deref()).bind(params.wild_farmed.as_deref()).bind(params.sushi)
    .fetch_all(&pool).await.unwrap_or_default();

    let map_row = |r: &Row| BestInSeasonItem {
        slug:        r.slug.clone().unwrap_or_default(),
        name:        localized(&r.name_en, &r.name_ru, &r.name_pl, &r.name_uk, lang),
        image_url:   r.image_url.clone(),
        status:      r.status.clone(),
        water_type:  r.water_type.clone(),
        wild_farmed: r.wild_farmed.clone(),
        sushi_grade: r.sushi_grade,
    };

    let peak:      Vec<BestInSeasonItem> = rows.iter().filter(|r| r.status == "peak").map(map_row).collect();
    let also_good: Vec<BestInSeasonItem> = rows.iter().filter(|r| r.status == "good").map(map_row).collect();

    Json(BestRightNowResponse {
        headline, month, month_name: mn.to_string(), product_type,
        region, lang: lang_code(lang).to_string(), peak, also_good,
    })
}

// ── 10. List regions ──────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct RegionInfo {
    pub code:        &'static str,
    pub name_en:     &'static str,
    pub description: &'static str,
}

#[derive(Serialize)]
pub struct RegionsResponse {
    pub regions: Vec<RegionInfo>,
}

/// GET /public/tools/regions
pub async fn list_regions() -> Json<RegionsResponse> {
    Json(RegionsResponse {
        regions: vec![
            RegionInfo { code: "GLOBAL", name_en: "Global",         description: "Generic global average seasonality" },
            RegionInfo { code: "PL",     name_en: "Poland",         description: "Central European seasonality (Poland)" },
            RegionInfo { code: "EU",     name_en: "European Union", description: "Average Western/Central Europe" },
            RegionInfo { code: "ES",     name_en: "Spain",          description: "Mediterranean / Southern Europe" },
            RegionInfo { code: "UA",     name_en: "Ukraine",        description: "Eastern European seasonality" },
        ],
    })
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn lang_code(lang: Language) -> &'static str {
    match lang {
        Language::Ru => "ru", Language::Pl => "pl",
        Language::Uk => "uk", Language::En => "en",
    }
}

fn localized(en: &str, ru: &str, pl: &str, uk: &str, lang: Language) -> String {
    match lang {
        Language::Ru => ru.to_string(),
        Language::Pl => pl.to_string(),
        Language::Uk => uk.to_string(),
        Language::En => en.to_string(),
    }
}

fn build_season(
    product_id: Option<uuid::Uuid>,
    seasonality: &[SeasonalityRow],
    lang: Language,
) -> Vec<FishSeasonEntry> {
    (1u8..=12).map(|m| {
        let status = product_id
            .and_then(|pid| seasonality.iter().find(|r| r.product_id == pid && r.month == m as i16))
            .map(|r| r.status.clone())
            .unwrap_or_else(|| "off".to_string());
        FishSeasonEntry {
            month: m,
            month_name: month_name(m, lang).to_string(),
            available: status != "off",
            status,
        }
    }).collect()
}

async fn load_all_catalog(pool: &PgPool) -> Vec<CatalogNutritionRow> {
    sqlx::query_as(
        &format!(
            r#"SELECT {} FROM catalog_ingredients WHERE is_active = true"#,
            crate::domain::tools::catalog_row::CATALOG_NUTRITION_COLS
        ),
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

fn find_row<'a>(rows: &'a [CatalogNutritionRow], name: &str) -> Option<&'a CatalogNutritionRow> {
    let n = name.to_lowercase();
    rows.iter()
        .find(|r| r.name_en.to_lowercase() == n || r.slug.as_deref() == Some(name))
        .or_else(|| rows.iter().find(|r| r.name_ru.to_lowercase() == n || r.name_pl.to_lowercase() == n || r.name_uk.to_lowercase() == n))
        .or_else(|| rows.iter().find(|r| r.name_en.to_lowercase().contains(&n)))
}

fn to_grams(amount: f64, unit: &str, density: f64) -> f64 {
    if unit == "g" {
        amount
    } else if let Some(g) = uc::mass_to_grams(unit) {
        amount * g
    } else if let Some(ml) = uc::volume_to_ml(unit) {
        amount * ml * density
    } else {
        amount
    }
}
