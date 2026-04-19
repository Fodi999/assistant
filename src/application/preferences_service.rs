use crate::domain::user_preferences::UserPreferences;
use crate::shared::{AppError, AppResult, UserId};
use sqlx::PgPool;

#[derive(Clone)]
pub struct PreferencesService {
    pool: PgPool,
}

impl PreferencesService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get preferences for user, returns defaults if none saved yet
    pub async fn get(&self, user_id: UserId) -> AppResult<UserPreferences> {
        let row = sqlx::query_as::<_, PrefRow>(
            r#"SELECT
                age, weight, target_weight,
                goal, calorie_target, protein_target, meals_per_day,
                diet, preferred_cuisine,
                cooking_level, cooking_time,
                likes, dislikes, allergies, intolerances, medical_conditions
            FROM user_preferences WHERE user_id = $1"#,
        )
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {e}")))?;

        Ok(match row {
            Some(r) => r.into_domain(),
            None => UserPreferences {
                goal: "eat_healthier".into(),
                calorie_target: 2200,
                protein_target: 120,
                meals_per_day: 3,
                diet: "no_restrictions".into(),
                preferred_cuisine: "any".into(),
                cooking_level: "home_cook".into(),
                cooking_time: "medium".into(),
                ..Default::default()
            },
        })
    }

    /// Upsert (INSERT ON CONFLICT UPDATE) user preferences
    pub async fn save(&self, user_id: UserId, prefs: &UserPreferences) -> AppResult<()> {
        sqlx::query(
            r#"INSERT INTO user_preferences (
                user_id, age, weight, target_weight,
                goal, calorie_target, protein_target, meals_per_day,
                diet, preferred_cuisine, cooking_level, cooking_time,
                likes, dislikes, allergies, intolerances, medical_conditions,
                updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17, now())
            ON CONFLICT (user_id) DO UPDATE SET
                age = EXCLUDED.age,
                weight = EXCLUDED.weight,
                target_weight = EXCLUDED.target_weight,
                goal = EXCLUDED.goal,
                calorie_target = EXCLUDED.calorie_target,
                protein_target = EXCLUDED.protein_target,
                meals_per_day = EXCLUDED.meals_per_day,
                diet = EXCLUDED.diet,
                preferred_cuisine = EXCLUDED.preferred_cuisine,
                cooking_level = EXCLUDED.cooking_level,
                cooking_time = EXCLUDED.cooking_time,
                likes = EXCLUDED.likes,
                dislikes = EXCLUDED.dislikes,
                allergies = EXCLUDED.allergies,
                intolerances = EXCLUDED.intolerances,
                medical_conditions = EXCLUDED.medical_conditions,
                updated_at = now()"#,
        )
        .bind(user_id.as_uuid())
        .bind(prefs.age)
        .bind(prefs.weight)
        .bind(prefs.target_weight)
        .bind(&prefs.goal)
        .bind(prefs.calorie_target)
        .bind(prefs.protein_target)
        .bind(prefs.meals_per_day)
        .bind(&prefs.diet)
        .bind(&prefs.preferred_cuisine)
        .bind(&prefs.cooking_level)
        .bind(&prefs.cooking_time)
        .bind(serde_json::to_value(&prefs.likes).unwrap_or_default())
        .bind(serde_json::to_value(&prefs.dislikes).unwrap_or_default())
        .bind(serde_json::to_value(&prefs.allergies).unwrap_or_default())
        .bind(serde_json::to_value(&prefs.intolerances).unwrap_or_default())
        .bind(serde_json::to_value(&prefs.medical_conditions).unwrap_or_default())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("DB error: {e}")))?;

        Ok(())
    }
}

// ── Internal row type ──────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct PrefRow {
    age: Option<i32>,
    weight: Option<f32>,
    target_weight: Option<f32>,
    goal: Option<String>,
    calorie_target: Option<i32>,
    protein_target: Option<i32>,
    meals_per_day: Option<i32>,
    diet: Option<String>,
    preferred_cuisine: Option<String>,
    cooking_level: Option<String>,
    cooking_time: Option<String>,
    likes: Option<serde_json::Value>,
    dislikes: Option<serde_json::Value>,
    allergies: Option<serde_json::Value>,
    intolerances: Option<serde_json::Value>,
    medical_conditions: Option<serde_json::Value>,
}

impl PrefRow {
    fn into_domain(self) -> UserPreferences {
        UserPreferences {
            age: self.age,
            weight: self.weight.map(|v| v as f64),
            target_weight: self.target_weight.map(|v| v as f64),
            goal: self.goal.unwrap_or_else(|| "eat_healthier".into()),
            calorie_target: self.calorie_target.unwrap_or(2200),
            protein_target: self.protein_target.unwrap_or(120),
            meals_per_day: self.meals_per_day.unwrap_or(3),
            diet: self.diet.unwrap_or_else(|| "no_restrictions".into()),
            preferred_cuisine: self.preferred_cuisine.unwrap_or_else(|| "any".into()),
            cooking_level: self.cooking_level.unwrap_or_else(|| "home_cook".into()),
            cooking_time: self.cooking_time.unwrap_or_else(|| "medium".into()),
            likes: json_to_strings(self.likes),
            dislikes: json_to_strings(self.dislikes),
            allergies: json_to_strings(self.allergies),
            intolerances: json_to_strings(self.intolerances),
            medical_conditions: json_to_strings(self.medical_conditions),
        }
    }
}

fn json_to_strings(val: Option<serde_json::Value>) -> Vec<String> {
    val.and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}
