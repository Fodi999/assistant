use crate::shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

// ══════════════════════════════════════════════════════
// SERVICE
// ══════════════════════════════════════════════════════

#[derive(Clone)]
pub struct AdminNutritionService {
    pub pool: PgPool,
}

impl AdminNutritionService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ══════════════════════════════════════════════════════
// DTOs
// ══════════════════════════════════════════════════════

/// One row in the products list
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct NutritionProductRow {
    pub id: Uuid,
    pub slug: String,
    pub name_en: Option<String>,
    pub name_ru: Option<String>,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub product_type: Option<String>,
    pub unit: Option<String>,
    pub image_url: Option<String>,
}

/// Full product detail (all joined nutrition tables)
#[derive(Debug, Serialize)]
pub struct NutritionProductDetail {
    // ── basic ──────────────────────────────────────────
    pub id: Uuid,
    pub slug: String,
    pub name_en: Option<String>,
    pub name_ru: Option<String>,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub product_type: Option<String>,
    pub unit: Option<String>,
    pub image_url: Option<String>,
    pub description_en: Option<String>,
    pub description_ru: Option<String>,
    pub description_pl: Option<String>,
    pub description_uk: Option<String>,
    pub density_g_per_ml: Option<f64>,
    pub typical_portion_g: Option<f64>,
    pub edible_yield_percent: Option<f64>,
    pub shelf_life_days: Option<i32>,
    pub wild_farmed: Option<String>,
    pub water_type: Option<String>,
    pub sushi_grade: Option<bool>,
    pub substitution_group: Option<String>,
    pub availability_months: Option<Vec<bool>>,
    // ── nutrition sub-tables ──────────────────────────
    pub macros: Option<MacrosDto>,
    pub vitamins: Option<VitaminsDto>,
    pub minerals: Option<MineralsDto>,
    pub fatty_acids: Option<FattyAcidsDto>,
    pub diet_flags: Option<DietFlagsDto>,
    pub allergens: Option<AllergensDto>,
    pub food_properties: Option<FoodPropertiesDto>,
    pub culinary: Option<CulinaryDto>,
}

// ── Macros ────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MacrosDto {
    pub calories_kcal: Option<f32>,
    pub protein_g: Option<f32>,
    pub fat_g: Option<f32>,
    pub carbs_g: Option<f32>,
    pub fiber_g: Option<f32>,
    pub sugar_g: Option<f32>,
    pub starch_g: Option<f32>,
    pub water_g: Option<f32>,
    pub alcohol_g: Option<f32>,
}

// ── Vitamins ──────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct VitaminsDto {
    pub vitamin_a: Option<f32>,
    pub vitamin_c: Option<f32>,
    pub vitamin_d: Option<f32>,
    pub vitamin_e: Option<f32>,
    pub vitamin_k: Option<f32>,
    pub vitamin_b1: Option<f32>,
    pub vitamin_b2: Option<f32>,
    pub vitamin_b3: Option<f32>,
    pub vitamin_b5: Option<f32>,
    pub vitamin_b6: Option<f32>,
    pub vitamin_b7: Option<f32>,
    pub vitamin_b9: Option<f32>,
    pub vitamin_b12: Option<f32>,
}

// ── Minerals ──────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MineralsDto {
    pub calcium: Option<f32>,
    pub iron: Option<f32>,
    pub magnesium: Option<f32>,
    pub phosphorus: Option<f32>,
    pub potassium: Option<f32>,
    pub sodium: Option<f32>,
    pub zinc: Option<f32>,
    pub copper: Option<f32>,
    pub manganese: Option<f32>,
    pub selenium: Option<f32>,
}

// ── Fatty Acids ───────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FattyAcidsDto {
    pub saturated_fat: Option<f32>,
    pub monounsaturated_fat: Option<f32>,
    pub polyunsaturated_fat: Option<f32>,
    pub omega3: Option<f32>,
    pub omega6: Option<f32>,
    pub epa: Option<f32>,
    pub dha: Option<f32>,
}

// ── Diet Flags ────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct DietFlagsDto {
    pub vegan: Option<bool>,
    pub vegetarian: Option<bool>,
    pub keto: Option<bool>,
    pub paleo: Option<bool>,
    pub gluten_free: Option<bool>,
    pub mediterranean: Option<bool>,
    pub low_carb: Option<bool>,
}

// ── Allergens ─────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct AllergensDto {
    // original 7
    pub milk: Option<bool>,
    pub fish: Option<bool>,
    pub shellfish: Option<bool>,
    pub nuts: Option<bool>,
    pub soy: Option<bool>,
    pub gluten: Option<bool>,
    pub eggs: Option<bool>,
    // EU-14 additions
    pub peanuts: Option<bool>,
    pub sesame: Option<bool>,
    pub celery: Option<bool>,
    pub mustard: Option<bool>,
    pub sulfites: Option<bool>,
    pub lupin: Option<bool>,
    pub molluscs: Option<bool>,
}

// ── Food Properties ───────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FoodPropertiesDto {
    pub glycemic_index: Option<f32>,
    pub glycemic_load: Option<f32>,
    pub ph: Option<f32>,
    pub smoke_point: Option<f32>,
    pub water_activity: Option<f32>,
}

// ── Culinary ──────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct CulinaryDto {
    pub sweetness: Option<f32>,
    pub acidity: Option<f32>,
    pub bitterness: Option<f32>,
    pub umami: Option<f32>,
    pub aroma: Option<f32>,
    pub texture: Option<String>,
}

// ── Update requests ───────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateProductBasicRequest {
    pub name_en: Option<String>,
    pub name_ru: Option<String>,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub product_type: Option<String>,
    pub unit: Option<String>,
    pub image_url: Option<String>,
    pub description_en: Option<String>,
    pub description_ru: Option<String>,
    pub description_pl: Option<String>,
    pub description_uk: Option<String>,
    pub density_g_per_ml: Option<f64>,
    pub typical_portion_g: Option<f64>,
    pub edible_yield_percent: Option<f64>,
    pub shelf_life_days: Option<i32>,
    pub wild_farmed: Option<String>,
    pub water_type: Option<String>,
    pub sushi_grade: Option<bool>,
    pub substitution_group: Option<String>,
    pub availability_months: Option<Vec<bool>>,
}

// ══════════════════════════════════════════════════════
// SERVICE IMPL
// ══════════════════════════════════════════════════════

/// Raw row from the products table (used internally for get_product)
#[derive(sqlx::FromRow)]
struct ProductBasicRow {
    id: Uuid,
    slug: String,
    name_en: Option<String>,
    name_ru: Option<String>,
    name_pl: Option<String>,
    name_uk: Option<String>,
    product_type: Option<String>,
    unit: Option<String>,
    image_url: Option<String>,
    description_en: Option<String>,
    description_ru: Option<String>,
    description_pl: Option<String>,
    description_uk: Option<String>,
    density_g_per_ml: Option<f32>,
    typical_portion_g: Option<f32>,
    edible_yield_percent: Option<f32>,
    shelf_life_days: Option<i32>,
    wild_farmed: Option<String>,
    water_type: Option<String>,
    sushi_grade: Option<bool>,
    substitution_group: Option<String>,
    availability_months: Option<Vec<bool>>,
}

impl AdminNutritionService {
    // ── List products ─────────────────────────────────
    pub async fn list_products(
        &self,
        page: i64,
        limit: i64,
        product_type: Option<String>,
        search: Option<String>,
    ) -> AppResult<Vec<NutritionProductRow>> {
        let offset = (page - 1) * limit;

        let rows = sqlx::query_as::<_, NutritionProductRow>(
            r#"
            SELECT id, slug, name_en, name_ru, name_pl, name_uk,
                   product_type, unit, image_url
            FROM products
            WHERE
                ($1::text IS NULL OR product_type = $1)
                AND ($2::text IS NULL OR
                     name_en ILIKE '%' || $2 || '%' OR
                     name_ru ILIKE '%' || $2 || '%' OR
                     slug    ILIKE '%' || $2 || '%')
            ORDER BY COALESCE(name_en, slug)
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(product_type)
        .bind(search)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(rows)
    }

    // ── Count products ────────────────────────────────
    pub async fn count_products(
        &self,
        product_type: Option<String>,
        search: Option<String>,
    ) -> AppResult<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM products
            WHERE
                ($1::text IS NULL OR product_type = $1)
                AND ($2::text IS NULL OR
                     name_en ILIKE '%' || $2 || '%' OR
                     name_ru ILIKE '%' || $2 || '%' OR
                     slug    ILIKE '%' || $2 || '%')
            "#,
        )
        .bind(product_type)
        .bind(search)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(count.0)
    }

    // ── Get full product detail ───────────────────────
    pub async fn get_product(&self, id: Uuid) -> AppResult<NutritionProductDetail> {
        // Basic row
        let row = sqlx::query_as::<_, ProductBasicRow>(
            r#"
            SELECT id, slug, name_en, name_ru, name_pl, name_uk,
                   product_type, unit, image_url, description_en,
                   description_ru, description_pl, description_uk,
                   density_g_per_ml, typical_portion_g, edible_yield_percent,
                   shelf_life_days, wild_farmed, water_type,
                   sushi_grade, substitution_group, availability_months
            FROM products
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound("Product not found".into()))?;

        let macros = sqlx::query_as::<_, MacrosDto>(
            "SELECT calories_kcal,protein_g,fat_g,carbs_g,fiber_g,sugar_g,starch_g,water_g,alcohol_g FROM nutrition_macros WHERE product_id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)?;

        let vitamins = sqlx::query_as::<_, VitaminsDto>(
            "SELECT vitamin_a,vitamin_c,vitamin_d,vitamin_e,vitamin_k,vitamin_b1,vitamin_b2,vitamin_b3,vitamin_b5,vitamin_b6,vitamin_b7,vitamin_b9,vitamin_b12 FROM nutrition_vitamins WHERE product_id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)?;

        let minerals = sqlx::query_as::<_, MineralsDto>(
            "SELECT calcium,iron,magnesium,phosphorus,potassium,sodium,zinc,copper,manganese,selenium FROM nutrition_minerals WHERE product_id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)?;

        let fatty_acids = sqlx::query_as::<_, FattyAcidsDto>(
            "SELECT saturated_fat,monounsaturated_fat,polyunsaturated_fat,omega3,omega6,epa,dha FROM nutrition_fatty_acids WHERE product_id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)?;

        let diet_flags = sqlx::query_as::<_, DietFlagsDto>(
            "SELECT vegan,vegetarian,keto,paleo,gluten_free,mediterranean,low_carb FROM diet_flags WHERE product_id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)?;

        let allergens = sqlx::query_as::<_, AllergensDto>(
            "SELECT milk,fish,shellfish,nuts,soy,gluten,eggs,peanuts,sesame,celery,mustard,sulfites,lupin,molluscs FROM product_allergens WHERE product_id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)?;

        let food_properties = sqlx::query_as::<_, FoodPropertiesDto>(
            "SELECT glycemic_index,glycemic_load,ph,smoke_point,water_activity FROM food_properties WHERE product_id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)?;

        let culinary = sqlx::query_as::<_, CulinaryDto>(
            "SELECT sweetness,acidity,bitterness,umami,aroma,texture FROM food_culinary_properties WHERE product_id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(NutritionProductDetail {
            id: row.id,
            slug: row.slug,
            name_en: row.name_en,
            name_ru: row.name_ru,
            name_pl: row.name_pl,
            name_uk: row.name_uk,
            product_type: row.product_type,
            unit: row.unit,
            image_url: row.image_url,
            description_en: row.description_en,
            description_ru: row.description_ru,
            description_pl: row.description_pl,
            description_uk: row.description_uk,
            density_g_per_ml: row.density_g_per_ml.map(|v| v as f64),
            typical_portion_g: row.typical_portion_g.map(|v| v as f64),
            edible_yield_percent: row.edible_yield_percent.map(|v| v as f64),
            shelf_life_days: row.shelf_life_days,
            wild_farmed: row.wild_farmed,
            water_type: row.water_type,
            sushi_grade: row.sushi_grade,
            substitution_group: row.substitution_group,
            availability_months: row.availability_months,
            macros,
            vitamins,
            minerals,
            fatty_acids,
            diet_flags,
            allergens,
            food_properties,
            culinary,
        })
    }

    // ── Update basic fields ───────────────────────────
    pub async fn update_basic(
        &self,
        id: Uuid,
        req: UpdateProductBasicRequest,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE products SET
                name_en              = COALESCE($2, name_en),
                name_ru              = COALESCE($3, name_ru),
                name_pl              = COALESCE($4, name_pl),
                name_uk              = COALESCE($5, name_uk),
                product_type         = COALESCE($6, product_type),
                unit                 = COALESCE($7, unit),
                image_url            = COALESCE($8, image_url),
                description_en       = COALESCE($9, description_en),
                description_ru       = COALESCE($10, description_ru),
                description_pl       = COALESCE($11, description_pl),
                description_uk       = COALESCE($12, description_uk),
                density_g_per_ml     = COALESCE($13, density_g_per_ml),
                typical_portion_g    = COALESCE($14, typical_portion_g),
                edible_yield_percent = COALESCE($15, edible_yield_percent),
                shelf_life_days      = COALESCE($16, shelf_life_days),
                wild_farmed          = COALESCE($17, wild_farmed),
                water_type           = COALESCE($18, water_type),
                sushi_grade          = COALESCE($19, sushi_grade),
                substitution_group   = COALESCE($20, substitution_group),
                availability_months  = COALESCE($21, availability_months),
                updated_at           = now()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(req.name_en)
        .bind(req.name_ru)
        .bind(req.name_pl)
        .bind(req.name_uk)
        .bind(req.product_type)
        .bind(req.unit)
        .bind(req.image_url)
        .bind(req.description_en)
        .bind(req.description_ru)
        .bind(req.description_pl)
        .bind(req.description_uk)
        .bind(req.density_g_per_ml.map(|v| v as f32))
        .bind(req.typical_portion_g.map(|v| v as f32))
        .bind(req.edible_yield_percent.map(|v| v as f32))
        .bind(req.shelf_life_days)
        .bind(req.wild_farmed)
        .bind(req.water_type)
        .bind(req.sushi_grade)
        .bind(req.substitution_group)
        .bind(req.availability_months)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(())
    }

    // ── Upsert macros ─────────────────────────────────
    pub async fn upsert_macros(&self, id: Uuid, dto: MacrosDto) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO nutrition_macros
                (product_id,calories_kcal,protein_g,fat_g,carbs_g,fiber_g,sugar_g,starch_g,water_g,alcohol_g)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            ON CONFLICT (product_id) DO UPDATE SET
                calories_kcal = EXCLUDED.calories_kcal,
                protein_g     = EXCLUDED.protein_g,
                fat_g         = EXCLUDED.fat_g,
                carbs_g       = EXCLUDED.carbs_g,
                fiber_g       = EXCLUDED.fiber_g,
                sugar_g       = EXCLUDED.sugar_g,
                starch_g      = EXCLUDED.starch_g,
                water_g       = EXCLUDED.water_g,
                alcohol_g     = EXCLUDED.alcohol_g
            "#,
        )
        .bind(id)
        .bind(dto.calories_kcal)
        .bind(dto.protein_g)
        .bind(dto.fat_g)
        .bind(dto.carbs_g)
        .bind(dto.fiber_g)
        .bind(dto.sugar_g)
        .bind(dto.starch_g)
        .bind(dto.water_g)
        .bind(dto.alcohol_g)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    // ── Upsert vitamins ───────────────────────────────
    pub async fn upsert_vitamins(&self, id: Uuid, dto: VitaminsDto) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO nutrition_vitamins
                (product_id,vitamin_a,vitamin_c,vitamin_d,vitamin_e,vitamin_k,
                 vitamin_b1,vitamin_b2,vitamin_b3,vitamin_b5,vitamin_b6,
                 vitamin_b7,vitamin_b9,vitamin_b12)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
            ON CONFLICT (product_id) DO UPDATE SET
                vitamin_a   = EXCLUDED.vitamin_a,
                vitamin_c   = EXCLUDED.vitamin_c,
                vitamin_d   = EXCLUDED.vitamin_d,
                vitamin_e   = EXCLUDED.vitamin_e,
                vitamin_k   = EXCLUDED.vitamin_k,
                vitamin_b1  = EXCLUDED.vitamin_b1,
                vitamin_b2  = EXCLUDED.vitamin_b2,
                vitamin_b3  = EXCLUDED.vitamin_b3,
                vitamin_b5  = EXCLUDED.vitamin_b5,
                vitamin_b6  = EXCLUDED.vitamin_b6,
                vitamin_b7  = EXCLUDED.vitamin_b7,
                vitamin_b9  = EXCLUDED.vitamin_b9,
                vitamin_b12 = EXCLUDED.vitamin_b12
            "#,
        )
        .bind(id)
        .bind(dto.vitamin_a)
        .bind(dto.vitamin_c)
        .bind(dto.vitamin_d)
        .bind(dto.vitamin_e)
        .bind(dto.vitamin_k)
        .bind(dto.vitamin_b1)
        .bind(dto.vitamin_b2)
        .bind(dto.vitamin_b3)
        .bind(dto.vitamin_b5)
        .bind(dto.vitamin_b6)
        .bind(dto.vitamin_b7)
        .bind(dto.vitamin_b9)
        .bind(dto.vitamin_b12)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    // ── Upsert minerals ───────────────────────────────
    pub async fn upsert_minerals(&self, id: Uuid, dto: MineralsDto) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO nutrition_minerals
                (product_id,calcium,iron,magnesium,phosphorus,potassium,
                 sodium,zinc,copper,manganese,selenium)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            ON CONFLICT (product_id) DO UPDATE SET
                calcium    = EXCLUDED.calcium,
                iron       = EXCLUDED.iron,
                magnesium  = EXCLUDED.magnesium,
                phosphorus = EXCLUDED.phosphorus,
                potassium  = EXCLUDED.potassium,
                sodium     = EXCLUDED.sodium,
                zinc       = EXCLUDED.zinc,
                copper     = EXCLUDED.copper,
                manganese  = EXCLUDED.manganese,
                selenium   = EXCLUDED.selenium
            "#,
        )
        .bind(id)
        .bind(dto.calcium)
        .bind(dto.iron)
        .bind(dto.magnesium)
        .bind(dto.phosphorus)
        .bind(dto.potassium)
        .bind(dto.sodium)
        .bind(dto.zinc)
        .bind(dto.copper)
        .bind(dto.manganese)
        .bind(dto.selenium)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    // ── Upsert fatty acids ────────────────────────────
    pub async fn upsert_fatty_acids(&self, id: Uuid, dto: FattyAcidsDto) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO nutrition_fatty_acids
                (product_id,saturated_fat,monounsaturated_fat,polyunsaturated_fat,omega3,omega6,epa,dha)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            ON CONFLICT (product_id) DO UPDATE SET
                saturated_fat       = EXCLUDED.saturated_fat,
                monounsaturated_fat = EXCLUDED.monounsaturated_fat,
                polyunsaturated_fat = EXCLUDED.polyunsaturated_fat,
                omega3              = EXCLUDED.omega3,
                omega6              = EXCLUDED.omega6,
                epa                 = EXCLUDED.epa,
                dha                 = EXCLUDED.dha
            "#,
        )
        .bind(id)
        .bind(dto.saturated_fat)
        .bind(dto.monounsaturated_fat)
        .bind(dto.polyunsaturated_fat)
        .bind(dto.omega3)
        .bind(dto.omega6)
        .bind(dto.epa)
        .bind(dto.dha)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    // ── Upsert diet flags ─────────────────────────────
    pub async fn upsert_diet_flags(&self, id: Uuid, dto: DietFlagsDto) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO diet_flags
                (product_id,vegan,vegetarian,keto,paleo,gluten_free,mediterranean,low_carb)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            ON CONFLICT (product_id) DO UPDATE SET
                vegan         = EXCLUDED.vegan,
                vegetarian    = EXCLUDED.vegetarian,
                keto          = EXCLUDED.keto,
                paleo         = EXCLUDED.paleo,
                gluten_free   = EXCLUDED.gluten_free,
                mediterranean = EXCLUDED.mediterranean,
                low_carb      = EXCLUDED.low_carb
            "#,
        )
        .bind(id)
        .bind(dto.vegan)
        .bind(dto.vegetarian)
        .bind(dto.keto)
        .bind(dto.paleo)
        .bind(dto.gluten_free)
        .bind(dto.mediterranean)
        .bind(dto.low_carb)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    // ── Upsert allergens ──────────────────────────────
    pub async fn upsert_allergens(&self, id: Uuid, dto: AllergensDto) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO product_allergens
                (product_id,milk,fish,shellfish,nuts,soy,gluten,eggs,
                 peanuts,sesame,celery,mustard,sulfites,lupin,molluscs)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            ON CONFLICT (product_id) DO UPDATE SET
                milk     = EXCLUDED.milk,
                fish     = EXCLUDED.fish,
                shellfish= EXCLUDED.shellfish,
                nuts     = EXCLUDED.nuts,
                soy      = EXCLUDED.soy,
                gluten   = EXCLUDED.gluten,
                eggs     = EXCLUDED.eggs,
                peanuts  = EXCLUDED.peanuts,
                sesame   = EXCLUDED.sesame,
                celery   = EXCLUDED.celery,
                mustard  = EXCLUDED.mustard,
                sulfites = EXCLUDED.sulfites,
                lupin    = EXCLUDED.lupin,
                molluscs = EXCLUDED.molluscs
            "#,
        )
        .bind(id)
        .bind(dto.milk)
        .bind(dto.fish)
        .bind(dto.shellfish)
        .bind(dto.nuts)
        .bind(dto.soy)
        .bind(dto.gluten)
        .bind(dto.eggs)
        .bind(dto.peanuts)
        .bind(dto.sesame)
        .bind(dto.celery)
        .bind(dto.mustard)
        .bind(dto.sulfites)
        .bind(dto.lupin)
        .bind(dto.molluscs)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    // ── Upsert food properties ────────────────────────
    pub async fn upsert_food_properties(&self, id: Uuid, dto: FoodPropertiesDto) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO food_properties
                (product_id,glycemic_index,glycemic_load,ph,smoke_point,water_activity)
            VALUES ($1,$2,$3,$4,$5,$6)
            ON CONFLICT (product_id) DO UPDATE SET
                glycemic_index = EXCLUDED.glycemic_index,
                glycemic_load  = EXCLUDED.glycemic_load,
                ph             = EXCLUDED.ph,
                smoke_point    = EXCLUDED.smoke_point,
                water_activity = EXCLUDED.water_activity
            "#,
        )
        .bind(id)
        .bind(dto.glycemic_index)
        .bind(dto.glycemic_load)
        .bind(dto.ph)
        .bind(dto.smoke_point)
        .bind(dto.water_activity)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    // ── Upsert culinary ───────────────────────────────
    pub async fn upsert_culinary(&self, id: Uuid, dto: CulinaryDto) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO food_culinary_properties
                (product_id,sweetness,acidity,bitterness,umami,aroma,texture)
            VALUES ($1,$2,$3,$4,$5,$6,$7)
            ON CONFLICT (product_id) DO UPDATE SET
                sweetness  = EXCLUDED.sweetness,
                acidity    = EXCLUDED.acidity,
                bitterness = EXCLUDED.bitterness,
                umami      = EXCLUDED.umami,
                aroma      = EXCLUDED.aroma,
                texture    = EXCLUDED.texture
            "#,
        )
        .bind(id)
        .bind(dto.sweetness)
        .bind(dto.acidity)
        .bind(dto.bitterness)
        .bind(dto.umami)
        .bind(dto.aroma)
        .bind(dto.texture)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(())
    }
}
