use restaurant_backend::application::inventory::InventoryService;
use restaurant_backend::domain::catalog::CatalogIngredientId;
use restaurant_backend::shared::{TenantId, UserId};
use sqlx::postgres::PgPoolOptions;
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::test]
async fn test_loss_report_flow() {
    // 1. Setup
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    let inventory_service = InventoryService::new(pool.clone());
    let user_id = UserId::from_uuid(Uuid::new_v4());
    let tenant_id = TenantId::from_uuid(Uuid::new_v4());

    // 0. Setup User and Tenant
    sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, 'Test Tenant')")
        .bind(tenant_id.as_uuid())
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO users (id, tenant_id, email, password_hash, display_name, role) VALUES ($1, $2, $3, 'hash', 'Test User', 'owner')")
        .bind(user_id.as_uuid())
        .bind(tenant_id.as_uuid())
        .bind(format!("test-{}@example.com", Uuid::new_v4()))
        .execute(&pool)
        .await
        .unwrap();

    // 1. Setup inventory
    let category_id = Uuid::new_v4();
    sqlx::query("INSERT INTO catalog_categories (id, name_en, name_pl, name_uk, name_ru) VALUES ($1, 'Test', 'Test', 'Test', 'Test')")
        .bind(category_id)
        .execute(&pool)
        .await
        .unwrap();

    let suffix = Uuid::new_v4().to_string()[..8].to_string();
    let ing_id = Uuid::new_v4();
    let ing_name = format!("Shrimp {}", suffix);

    sqlx::query(
        "INSERT INTO catalog_ingredients (id, category_id, name_en, name_pl, name_uk, name_ru, default_unit) 
         VALUES ($1, $2, $3, $4, $5, $6, $7::unit_type)"
    )
    .bind(ing_id)
    .bind(category_id)
    .bind(&ing_name)
    .bind(&ing_name)
    .bind(&ing_name)
    .bind(&ing_name)
    .bind("kilogram")
    .execute(&pool)
    .await
    .unwrap();

    let now = OffsetDateTime::now_utc();
    
    // Batch 1: ALREADY EXPIRED (Expires yesterday)
    // 10 kg * 10.0 PLN = 100.0 PLN (10000 cents)
    let _b1_id = inventory_service.add_batch(
        user_id, tenant_id, CatalogIngredientId::from_uuid(ing_id), 1000, 10.0, 
        None, None, now - time::Duration::days(5), Some(now - time::Duration::days(1))
    ).await.unwrap();

    // Batch 2: NOT EXPIRED (Expires tomorrow)
    inventory_service.add_batch(
        user_id, tenant_id, CatalogIngredientId::from_uuid(ing_id), 1000, 5.0, 
        None, None, now, Some(now + time::Duration::days(1))
    ).await.unwrap();

    println!("✅ Running process_expirations...");
    let processed = inventory_service.process_expirations(tenant_id).await.unwrap();
    assert_eq!(processed, 1, "Only batch 1 should be processed");

    println!("✅ Verifying loss report...");
    let report = inventory_service.get_loss_report(tenant_id, 7).await.unwrap();
    
    assert!(report.items.iter().any(|i| i.ingredient_id == ing_id));
    let item = report.items.iter().find(|i| i.ingredient_id == ing_id).unwrap();
    
    assert_eq!(item.lost_quantity, 10.0);
    assert_eq!(item.loss_value_cents, 10000);
    assert_eq!(report.total_loss_cents, 10000);

    println!("🚀 LOSS REPORT TEST PASSED!");
}
