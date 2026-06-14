use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

const SITE_KEY: &str = "almabuild";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialCategory {
    pub index: String,
    pub slug: String,
    pub title: String,
    pub text: String,
    pub bullets: Vec<String>,
    pub photo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub category_slug: String,
    pub category: String,
    pub title: String,
    pub spec: String,
    pub photo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kit {
    pub title: String,
    pub text: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub title: String,
    pub meta: String,
    pub photo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlmabuildContent {
    pub material_categories: Vec<MaterialCategory>,
    pub products: Vec<Product>,
    pub kits: Vec<Kit>,
    pub projects: Vec<Project>,
}

fn default_content() -> AlmabuildContent {
    AlmabuildContent {
        material_categories: vec![
            MaterialCategory { index: "[0:1]".into(), slug: "gipsokarton-profili".into(), title: "Гипсокартон и профили".into(), text: "Листы ГКЛ, направляющие и стоечные профили, подвесы, крепёж и комплектующие для перегородок и потолков.".into(), bullets: vec!["Листы ГКЛ".into(), "Профили и направляющие".into(), "Подвесы и крепёж".into(), "Комплектующие".into()], photo: "material-drywall".into() },
            MaterialCategory { index: "[0:2]".into(), slug: "sukhie-smesi".into(), title: "Сухие смеси".into(), text: "Штукатурка, шпаклёвка, наливные полы, плиточный клей, грунтовки и расходные материалы.".into(), bullets: vec!["Штукатурки и шпаклёвки".into(), "Плиточный клей".into(), "Наливные полы".into(), "Грунтовки и добавки".into()], photo: "material-mixes".into() },
            MaterialCategory { index: "[0:3]".into(), slug: "poly-plitka".into(), title: "Полы и плитка".into(), text: "Керамогранит, плитка, кварцвинил, ламинат, плинтусы, затирка и материалы для укладки.".into(), bullets: vec!["Керамогранит и плитка".into(), "Кварцвинил и ламинат".into(), "Плинтусы и пороги".into(), "Затирки и клеи".into()], photo: "material-flooring".into() },
            MaterialCategory { index: "[0:4]".into(), slug: "elektrika-osveshchenie".into(), title: "Электрика и освещение".into(), text: "Кабель, автоматы, розетки, трековое освещение, светильники и LED-решения для магазинов.".into(), bullets: vec!["Кабель и провода".into(), "Автоматы и щиты".into(), "Розетки и выключатели".into(), "Светильники и LED-решения".into()], photo: "material-electric".into() },
            MaterialCategory { index: "[0:5]".into(), slug: "potolochnye-sistemy".into(), title: "Потолочные системы".into(), text: "Армстронг, грильято, гипсокартонные потолки, подвесные системы и комплектующие.".into(), bullets: vec!["Армстронг и грильято".into(), "Гипсокартонные потолки".into(), "Подвесные системы".into(), "Комплектующие".into()], photo: "material-ceiling".into() },
            MaterialCategory { index: "[0:6]".into(), slug: "osb-fanera-uteplitel".into(), title: "OSB, фанера и утеплитель".into(), text: "OSB, фанера, минеральная вата, гидроизоляция, мембраны и теплоизоляционные материалы.".into(), bullets: vec!["OSB и фанера".into(), "Минеральная вата".into(), "Гидроизоляция".into(), "Мембраны и плёнки".into()], photo: "material-osb".into() },
        ],
        products: vec![
            Product { category_slug: "gipsokarton-profili".into(), category: "ГКЛ".into(), title: "ГКЛ 12.5 мм стандартный".into(), spec: "2500x1200 мм · стены и потолки".into(), photo: "photo-plans".into() },
            Product { category_slug: "gipsokarton-profili".into(), category: "ГКЛ".into(), title: "ГКЛ влагостойкий 12.5 мм".into(), spec: "Для влажных зон и аптек".into(), photo: "photo-plans".into() },
            Product { category_slug: "gipsokarton-profili".into(), category: "Профили".into(), title: "Профиль стоечный CW".into(), spec: "50/75/100 мм · перегородки".into(), photo: "photo-building".into() },
            Product { category_slug: "sukhie-smesi".into(), category: "Сухие смеси".into(), title: "Плиточный клей усиленный".into(), spec: "Для керамогранита и плитки".into(), photo: "photo-retail".into() },
        ],
        kits: vec![
            Kit { title: "Комплект для перегородок".into(), text: "Каркас, листы, крепёж и расходники.".into(), items: vec!["ГКЛ".into(), "CW/UW профили".into(), "Подвесы и саморезы".into()] },
            Kit { title: "Комплект для потолка".into(), text: "Система под монтаж потолков.".into(), items: vec!["Профили".into(), "Подвесы".into(), "Плиты / ГКЛ".into()] },
        ],
        projects: vec![
            Project { title: "BUTIK KZ".into(), meta: "Магазин одежды · 320 м² · 28 дней".into(), photo: "photo-retail".into() },
            Project { title: "Green Mart".into(), meta: "Супермаркет · 1250 м² · 45 дней".into(), photo: "photo-office".into() },
            Project { title: "Europharma".into(), meta: "Аптека · 110 м² · 18 дней".into(), photo: "photo-building".into() },
        ],
    }
}

async fn load_content(pool: &PgPool) -> Result<AlmabuildContent, StatusCode> {
    let row: Option<Value> = sqlx::query_scalar("SELECT content FROM site_content WHERE site = $1")
        .bind(SITE_KEY)
        .fetch_optional(pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to load almabuild content");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match row {
        Some(value) => serde_json::from_value(value).map_err(|error| {
            tracing::error!(%error, "invalid almabuild content json");
            StatusCode::INTERNAL_SERVER_ERROR
        }),
        None => Ok(default_content()),
    }
}

pub async fn public_content(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?))
}

pub async fn admin_get_content(
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?))
}

pub async fn admin_put_content(
    State(pool): State<PgPool>,
    Json(content): Json<AlmabuildContent>,
) -> Result<impl IntoResponse, StatusCode> {
    let value = serde_json::to_value(&content).map_err(|error| {
        tracing::error!(%error, "failed to serialize almabuild content");
        StatusCode::BAD_REQUEST
    })?;

    sqlx::query(
        r#"
        INSERT INTO site_content (site, content, updated_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (site)
        DO UPDATE SET content = EXCLUDED.content, updated_at = NOW()
        "#,
    )
    .bind(SITE_KEY)
    .bind(value)
    .execute(&pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "failed to save almabuild content");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(content))
}
