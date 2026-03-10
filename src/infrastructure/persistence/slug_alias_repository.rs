use sqlx::PgPool;
use uuid::Uuid;

/// Repository for slug aliases (old_slug → current ingredient).
/// Used for 301 redirects when product names change.
#[derive(Clone)]
pub struct SlugAliasRepository {
    pool: PgPool,
}

/// Result of resolving a slug: either a direct ingredient or a redirect.
#[derive(Debug)]
pub enum SlugResolution {
    /// The slug matches the current ingredient directly — no redirect needed.
    Direct,
    /// The slug is an old alias. Frontend should 301 → new_slug.
    Redirect { new_slug: String },
    /// Slug not found at all.
    NotFound,
}

impl SlugAliasRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Resolve a slug: check if it's a current slug or an old alias.
    ///
    /// Returns:
    /// - `Direct` if the slug matches a current active ingredient
    /// - `Redirect { new_slug }` if it's an old alias pointing to a renamed ingredient
    /// - `NotFound` if neither
    pub async fn resolve(&self, slug: &str) -> Result<SlugResolution, sqlx::Error> {
        // 1. Check if it's a current active slug
        let current: Option<(String,)> = sqlx::query_as(
            "SELECT slug FROM catalog_ingredients WHERE slug = $1 AND is_active = true LIMIT 1",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        if current.is_some() {
            return Ok(SlugResolution::Direct);
        }

        // 2. Check if it's an old alias
        let alias: Option<(String,)> = sqlx::query_as(
            r#"SELECT ci.slug
               FROM slug_aliases sa
               JOIN catalog_ingredients ci ON ci.id = sa.ingredient_id
               WHERE sa.old_slug = $1 AND ci.is_active = true
               LIMIT 1"#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        match alias {
            Some((new_slug,)) => Ok(SlugResolution::Redirect { new_slug }),
            None => Ok(SlugResolution::NotFound),
        }
    }

    /// Save an old slug as an alias for an ingredient.
    pub async fn save_alias(
        &self,
        ingredient_id: Uuid,
        old_slug: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO slug_aliases (ingredient_id, old_slug)
               VALUES ($1, $2)
               ON CONFLICT (old_slug) DO NOTHING"#,
        )
        .bind(ingredient_id)
        .bind(old_slug)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// List all aliases for an ingredient (useful for admin panel).
    pub async fn list_aliases(&self, ingredient_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT old_slug FROM slug_aliases WHERE ingredient_id = $1 ORDER BY created_at DESC",
        )
        .bind(ingredient_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(s,)| s).collect())
    }

    /// Delete an alias (e.g., admin wants to free up an old slug).
    pub async fn delete_alias(&self, old_slug: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM slug_aliases WHERE old_slug = $1")
            .bind(old_slug)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
