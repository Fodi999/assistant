use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use restaurant_backend::{
    domain::AdminClaims,
    interfaces::http::{
        admin_panel::{create_affiliate_product, AffiliateProductPayload},
        site_context::SiteQuery,
    },
};
use sqlx::PgPool;
use uuid::Uuid;

const CHURCH_SITE_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000101);
const CONSTRUCTION_SITE_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000102);
const KITCHEN_SITE_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000103);

fn slug(prefix: &str) -> String {
    format!("{}-{}", prefix, Uuid::new_v4())
}

#[sqlx::test(migrations = "./migrations")]
async fn scoped_leads_never_return_for_another_site(pool: PgPool) -> sqlx::Result<()> {
    let church_lead_id = Uuid::new_v4();
    let construction_lead_id = Uuid::new_v4();
    let global_lead_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO site_leads (id, site_id, is_global, site, name, phone, object_type, area, comment)
        VALUES
            ($1, $2, false, 'church', 'Church lead', '+10000000001', 'icon', '', ''),
            ($3, $4, false, 'construction', 'Construction lead', '+10000000002', 'repair', '', ''),
            ($5, $2, true, 'church', 'Global lead', '+10000000003', 'global', '', '')
        "#,
    )
    .bind(church_lead_id)
    .bind(CHURCH_SITE_ID)
    .bind(construction_lead_id)
    .bind(CONSTRUCTION_SITE_ID)
    .bind(global_lead_id)
    .execute(&pool)
    .await?;

    let ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM site_leads WHERE site_id = $1 OR is_global = true ORDER BY created_at DESC",
    )
    .bind(CONSTRUCTION_SITE_ID)
    .fetch_all(&pool)
    .await?;

    assert!(ids.contains(&construction_lead_id));
    assert!(ids.contains(&global_lead_id));
    assert!(!ids.contains(&church_lead_id));

    let changed =
        sqlx::query("UPDATE site_leads SET status = 'won' WHERE id = $1 AND site_id = $2")
            .bind(church_lead_id)
            .bind(CONSTRUCTION_SITE_ID)
            .execute(&pool)
            .await?
            .rows_affected();

    assert_eq!(changed, 0);
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn catalog_cms_and_shop_are_site_scoped_with_global_reads(pool: PgPool) -> sqlx::Result<()> {
    let church_catalog = Uuid::new_v4();
    let construction_catalog = Uuid::new_v4();
    let global_catalog = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO catalog_ingredients
            (id, site_id, is_global, slug, name_pl, name_en, name_uk, name_ru, default_unit)
        VALUES
            ($1, $2, false, $3, 'Church PL', 'Church EN', 'Church UK', 'Church RU', 'piece'),
            ($4, $5, false, $6, 'Construction PL', 'Construction EN', 'Construction UK', 'Construction RU', 'piece'),
            ($7, $2, true, $8, 'Global PL', 'Global EN', 'Global UK', 'Global RU', 'piece')
        "#,
    )
    .bind(church_catalog)
    .bind(CHURCH_SITE_ID)
    .bind(slug("church-catalog"))
    .bind(construction_catalog)
    .bind(CONSTRUCTION_SITE_ID)
    .bind(slug("construction-catalog"))
    .bind(global_catalog)
    .bind(slug("global-catalog"))
    .execute(&pool)
    .await?;

    let catalog_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM catalog_ingredients WHERE (site_id = $1 OR is_global = true) AND is_active = true",
    )
    .bind(CONSTRUCTION_SITE_ID)
    .fetch_all(&pool)
    .await?;
    assert!(catalog_ids.contains(&construction_catalog));
    assert!(catalog_ids.contains(&global_catalog));
    assert!(!catalog_ids.contains(&church_catalog));

    let church_catalog_visible = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM catalog_ingredients WHERE id = $1 AND (site_id = $2 OR is_global = true)",
    )
    .bind(church_catalog)
    .bind(CONSTRUCTION_SITE_ID)
    .fetch_optional(&pool)
    .await?;
    assert!(church_catalog_visible.is_none());

    let catalog_updates = sqlx::query(
        "UPDATE catalog_ingredients SET name_en = 'Wrong site update' WHERE id = $1 AND site_id = $2",
    )
    .bind(church_catalog)
    .bind(CONSTRUCTION_SITE_ID)
    .execute(&pool)
    .await?
    .rows_affected();
    assert_eq!(catalog_updates, 0);

    let catalog_deletes = sqlx::query(
        "UPDATE catalog_ingredients SET is_active = false WHERE id = $1 AND site_id = $2 AND is_active = true",
    )
    .bind(church_catalog)
    .bind(CONSTRUCTION_SITE_ID)
    .execute(&pool)
    .await?
    .rows_affected();
    assert_eq!(catalog_deletes, 0);

    let church_article = Uuid::new_v4();
    let construction_article = Uuid::new_v4();
    let global_article = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO knowledge_articles (id, site_id, is_global, slug, category, title_en)
        VALUES
            ($1, $2, false, $3, 'icons', 'Church article'),
            ($4, $5, false, $6, 'build', 'Construction article'),
            ($7, $2, true, $8, 'global', 'Global article')
        "#,
    )
    .bind(church_article)
    .bind(CHURCH_SITE_ID)
    .bind(slug("church-article"))
    .bind(construction_article)
    .bind(CONSTRUCTION_SITE_ID)
    .bind(slug("construction-article"))
    .bind(global_article)
    .bind(slug("global-article"))
    .execute(&pool)
    .await?;

    let article_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM knowledge_articles WHERE site_id = $1 OR is_global = true",
    )
    .bind(CONSTRUCTION_SITE_ID)
    .fetch_all(&pool)
    .await?;
    assert!(article_ids.contains(&construction_article));
    assert!(article_ids.contains(&global_article));
    assert!(!article_ids.contains(&church_article));

    let article_updates =
        sqlx::query("UPDATE knowledge_articles SET title_en = 'Wrong site update' WHERE id = $1 AND site_id = $2")
            .bind(church_article)
            .bind(CONSTRUCTION_SITE_ID)
            .execute(&pool)
            .await?
            .rows_affected();
    assert_eq!(article_updates, 0);

    let article_deletes =
        sqlx::query("DELETE FROM knowledge_articles WHERE id = $1 AND site_id = $2")
            .bind(church_article)
            .bind(CONSTRUCTION_SITE_ID)
            .execute(&pool)
            .await?
            .rows_affected();
    assert_eq!(article_deletes, 0);

    let church_shop = Uuid::new_v4();
    let construction_shop = Uuid::new_v4();
    let global_shop = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO shop_products (id, site_id, is_global, slug, name_en)
        VALUES
            ($1, $2, false, $3, 'Church product'),
            ($4, $5, false, $6, 'Construction product'),
            ($7, $2, true, $8, 'Global product')
        "#,
    )
    .bind(church_shop)
    .bind(CHURCH_SITE_ID)
    .bind(slug("church-shop"))
    .bind(construction_shop)
    .bind(CONSTRUCTION_SITE_ID)
    .bind(slug("construction-shop"))
    .bind(global_shop)
    .bind(slug("global-shop"))
    .execute(&pool)
    .await?;

    let shop_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM shop_products WHERE site_id = $1 OR is_global = true",
    )
    .bind(CONSTRUCTION_SITE_ID)
    .fetch_all(&pool)
    .await?;
    assert!(shop_ids.contains(&construction_shop));
    assert!(shop_ids.contains(&global_shop));
    assert!(!shop_ids.contains(&church_shop));

    let shop_updates =
        sqlx::query("UPDATE shop_products SET status = 'active' WHERE id = $1 AND site_id = $2")
            .bind(church_shop)
            .bind(CONSTRUCTION_SITE_ID)
            .execute(&pool)
            .await?
            .rows_affected();
    assert_eq!(shop_updates, 0);

    let shop_deletes = sqlx::query("DELETE FROM shop_products WHERE id = $1 AND site_id = $2")
        .bind(church_shop)
        .bind(CONSTRUCTION_SITE_ID)
        .execute(&pool)
        .await?
        .rows_affected();
    assert_eq!(shop_deletes, 0);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn suppliers_settings_and_analytics_respect_site_scope_and_global_reads(
    pool: PgPool,
) -> sqlx::Result<()> {
    let church_supplier = Uuid::new_v4();
    let construction_supplier = Uuid::new_v4();
    let global_supplier = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO admin_suppliers (id, site_id, is_global, name)
        VALUES
            ($1, $2, false, 'Church supplier'),
            ($3, $4, false, 'Construction supplier'),
            ($5, $2, true, 'Global supplier')
        "#,
    )
    .bind(church_supplier)
    .bind(CHURCH_SITE_ID)
    .bind(construction_supplier)
    .bind(CONSTRUCTION_SITE_ID)
    .bind(global_supplier)
    .execute(&pool)
    .await?;

    let supplier_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM admin_suppliers WHERE site_id = $1 OR is_global = true",
    )
    .bind(CONSTRUCTION_SITE_ID)
    .fetch_all(&pool)
    .await?;
    assert!(supplier_ids.contains(&construction_supplier));
    assert!(supplier_ids.contains(&global_supplier));
    assert!(!supplier_ids.contains(&church_supplier));

    let supplier_updates = sqlx::query(
        "UPDATE admin_suppliers SET name = 'Wrong site update' WHERE id = $1 AND site_id = $2",
    )
    .bind(church_supplier)
    .bind(CONSTRUCTION_SITE_ID)
    .execute(&pool)
    .await?
    .rows_affected();
    assert_eq!(supplier_updates, 0);

    let settings_key = format!("site-isolation-{}", Uuid::new_v4());
    let global_settings_key = format!("global-site-isolation-{}", Uuid::new_v4());
    sqlx::query("INSERT INTO seo_settings (key, value, site_id, is_global) VALUES ($1, 'church', $2, false), ($3, 'global', $2, true)")
        .bind(&settings_key)
        .bind(CHURCH_SITE_ID)
        .bind(&global_settings_key)
        .execute(&pool)
        .await?;

    let setting_keys = sqlx::query_scalar::<_, String>(
        "SELECT key FROM seo_settings WHERE site_id = $1 OR is_global = true",
    )
    .bind(CONSTRUCTION_SITE_ID)
    .fetch_all(&pool)
    .await?;
    assert!(setting_keys.contains(&global_settings_key));
    assert!(!setting_keys.contains(&settings_key));

    let church_stat = Uuid::new_v4();
    let construction_stat = Uuid::new_v4();
    let global_stat = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO ai_usage_stats (id, site_id, is_global, endpoint)
        VALUES
            ($1, $2, false, 'church-test'),
            ($3, $4, false, 'construction-test'),
            ($5, $2, true, 'global-test')
        "#,
    )
    .bind(church_stat)
    .bind(CHURCH_SITE_ID)
    .bind(construction_stat)
    .bind(CONSTRUCTION_SITE_ID)
    .bind(global_stat)
    .execute(&pool)
    .await?;

    let stat_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM ai_usage_stats WHERE site_id = $1 OR is_global = true",
    )
    .bind(CONSTRUCTION_SITE_ID)
    .fetch_all(&pool)
    .await?;
    assert!(stat_ids.contains(&construction_stat));
    assert!(stat_ids.contains(&global_stat));
    assert!(!stat_ids.contains(&church_stat));

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn create_does_not_trust_spoofed_site_from_payload(pool: PgPool) -> sqlx::Result<()> {
    let claims = AdminClaims {
        sub: "admin@example.test".to_string(),
        role: "super_admin".to_string(),
        exp: usize::MAX,
        iat: 0,
    };
    let slug = slug("spoofed-affiliate");
    let payload = AffiliateProductPayload {
        site: Some("church".to_string()),
        title: None,
        slug: Some(slug.clone()),
        category: Some("test".to_string()),
        network: None,
        merchant: None,
        affiliate_url: Some("https://example.test/product".to_string()),
        image_url: None,
        detail_image_url: None,
        price: None,
        currency: None,
        commission_percent: None,
        cookie_days: None,
        status: None,
        languages: None,
        seo_title: None,
        seo_description: None,
    };

    let (status, Json(created)) = create_affiliate_product(
        claims,
        Query(SiteQuery {
            site_id: Some(CONSTRUCTION_SITE_ID),
            site: None,
        }),
        State(pool.clone()),
        Json(payload),
    )
    .await
    .expect("affiliate product should be created");

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created.site, "construction");

    let stored_site_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT site_id FROM admin_affiliate_products WHERE slug = $1",
    )
    .bind(slug)
    .fetch_one(&pool)
    .await?;
    assert_eq!(stored_site_id, CONSTRUCTION_SITE_ID);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn migrated_tables_have_no_null_site_ids(pool: PgPool) -> sqlx::Result<()> {
    let tables = [
        "catalog_ingredients",
        "knowledge_articles",
        "shop_products",
        "site_leads",
        "admin_affiliate_products",
        "admin_suppliers",
        "seo_settings",
        "ai_usage_stats",
    ];

    for table in tables {
        let sql = format!("SELECT COUNT(*) FROM {table} WHERE site_id IS NULL");
        let null_count: i64 = sqlx::query_scalar(&sql).fetch_one(&pool).await?;
        assert_eq!(null_count, 0, "{table} contains NULL site_id rows");
    }

    let site_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sites")
        .fetch_one(&pool)
        .await?;
    assert!(site_count >= 3);
    assert_ne!(KITCHEN_SITE_ID, CONSTRUCTION_SITE_ID);

    Ok(())
}
