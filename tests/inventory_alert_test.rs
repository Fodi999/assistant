use restaurant_backend::application::{InventoryService, InventoryAlertService};
use restaurant_backend::shared::{TenantId, UserId};
use restaurant_backend::domain::catalog::CatalogIngredientId;
use restaurant_backend::domain::inventory::{AlertSeverity, InventoryAlertType};
use sqlx::postgres::PgPoolOptions;
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::test]
async fn test_inventory_alerts_suite() {
    // 1. Setup
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    let inventory_service = InventoryService::new(pool.clone());
    let alert_service = InventoryAlertService::new(pool.clone());
    
    // Create Test Tenant and User
    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    
    sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, $2)")
        .bind(tenant_id.as_uuid())
        .bind("Alert Test Tenant")
        .execute(&pool)
        .await
        .expect("Failed to insert tenant");

    sqlx::query("INSERT INTO users (id, tenant_id, email, password_hash, role) VALUES ($1, $2, $3, $4, $5)")
        .bind(user_id.as_uuid())
        .bind(tenant_id.as_uuid())
        .bind(format!("alert-test-{}@example.com", Uuid::new_v4()))
        .bind("hash")
        .bind("owner")
        .execute(&pool)
        .await
        .expect("Failed to insert user");
    
    // Create category
    let category_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO catalog_categories (id, name_en, name_pl, name_uk, name_ru, sort_order) 
         VALUES ($1, 'Alert Cat', 'Alert Cat', 'Alert Cat', 'Alert Cat', 1)"
    )
    .bind(category_id)
    .execute(&pool)
    .await
    .expect("Failed to insert category");

    // --- SCENARIO 1: Expiration Alerts ---
    println!("✅ Testing Expiration Alerts...");
    
    let suffix = Uuid::new_v4().to_string()[..8].to_string();
    let ing_exp_name = format!("Expiring Fish {}", suffix);
    let ing_exp_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO catalog_ingredients (id, category_id, name_en, name_pl, name_uk, name_ru, default_unit) 
         VALUES ($1, $2, $3, $4, $5, $6, $7::unit_type)"
    )
    .bind(ing_exp_id)
    .bind(category_id)
    .bind(&ing_exp_name)
    .bind(&ing_exp_name)
    .bind(&ing_exp_name)
    .bind(&ing_exp_name)
    .bind("kilogram")
    .execute(&pool)
    .await
    .unwrap();

    let now = OffsetDateTime::now_utc();
    
    // Batch 1: Expiring in < 1 day (Critical)
    inventory_service.add_batch(
        user_id, tenant_id, CatalogIngredientId::from_uuid(ing_exp_id), 1000, 10.0, 
        None, None, now, Some(now + time::Duration::hours(12))
    ).await.unwrap();

    // Batch 2: Expiring in 2 days (Warning)
    inventory_service.add_batch(
        user_id, tenant_id, CatalogIngredientId::from_uuid(ing_exp_id), 1000, 5.0, 
        None, None, now, Some(now + time::Duration::days(2))
    ).await.unwrap();

    let alerts = alert_service.get_alerts(tenant_id).await.unwrap();
    let exp_alerts: Vec<_> = alerts.iter().filter(|a| a.alert_type == InventoryAlertType::ExpiringBatch).collect();
    
    for a in &exp_alerts {
        println!("Found aggregated expiration alert: {} ({:?})", a.message, a.severity);
    }
    
    // With aggregation, we expect 1 alert for this ingredient (Critical takes precedence over Warning)
    assert!(exp_alerts.len() >= 1);
    assert!(exp_alerts.iter().any(|a| a.severity == AlertSeverity::Critical && a.ingredient_id == ing_exp_id));

    // --- SCENARIO 2: Low Stock Alerts ---
    println!("✅ Testing Low Stock Alerts...");
    
    let ing_low_id = Uuid::new_v4();
    let ing_low_name = format!("Low Milk {}", suffix);
    sqlx::query(
        "INSERT INTO catalog_ingredients (id, category_id, name_en, name_pl, name_uk, name_ru, default_unit, min_stock_threshold) 
         VALUES ($1, $2, $3, $4, $5, $6, $7::unit_type, 10.0)"
    )
    .bind(ing_low_id)
    .bind(category_id)
    .bind(&ing_low_name)
    .bind(&ing_low_name)
    .bind(&ing_low_name)
    .bind(&ing_low_name)
    .bind("liter")
    .execute(&pool)
    .await
    .unwrap();

    // 1. Check alert when stock is 0 (Critical)
    let alerts_before = alert_service.get_alerts(tenant_id).await.unwrap();
    let milk_alert_0 = alerts_before.iter().find(|a| a.ingredient_id == ing_low_id).expect("Should have out of stock alert");
    assert_eq!(milk_alert_0.severity, AlertSeverity::Critical);
    assert!(milk_alert_0.message.contains("OUT OF STOCK"));

    // 2. Add some stock, but below threshold (Warning)
    inventory_service.add_batch(
        user_id, tenant_id, CatalogIngredientId::from_uuid(ing_low_id), 500, 3.0, 
        None, None, now, None
    ).await.unwrap();

    let alerts_after = alert_service.get_alerts(tenant_id).await.unwrap();
    let milk_alert_3 = alerts_after.iter().find(|a| a.ingredient_id == ing_low_id).expect("Should have low stock alert");
    assert_eq!(milk_alert_3.severity, AlertSeverity::Warning);
    assert_eq!(milk_alert_3.current_value, 3.0);
    assert_eq!(milk_alert_3.threshold_value, Some(10.0));

    // 3. Add more stock to exceed threshold (No alert)
    inventory_service.add_batch(
        user_id, tenant_id, CatalogIngredientId::from_uuid(ing_low_id), 500, 10.0, 
        None, None, now, None
    ).await.unwrap();

    let alerts_final = alert_service.get_alerts(tenant_id).await.unwrap();
    let has_milk_alert = alerts_final.iter().any(|a| a.ingredient_id == ing_low_id);
    assert!(!has_milk_alert, "Should not have low stock alert when above threshold");

    println!("🚀 ALL ALERT ENGINE TESTS PASSED!");
}
