use restaurant_backend::application::InventoryService;
use restaurant_backend::shared::{TenantId, UserId};
use restaurant_backend::domain::catalog::CatalogIngredientId;
use restaurant_backend::domain::inventory::{BatchStatus};
use sqlx::postgres::PgPoolOptions;
use time::OffsetDateTime;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_inventory_fifo_full_suite() {
    // 1. Setup
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    let inventory_service = InventoryService::new(pool.clone());
    
    // Create Test Tenant and User
    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    
    sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, $2)")
        .bind(tenant_id.as_uuid())
        .bind("Test Tenant")
        .execute(&pool)
        .await
        .expect("Failed to insert tenant");

    sqlx::query("INSERT INTO users (id, tenant_id, email, password_hash, role) VALUES ($1, $2, $3, $4, $5)")
        .bind(user_id.as_uuid())
        .bind(tenant_id.as_uuid())
        .bind(format!("test-{}@example.com", Uuid::new_v4()))
        .bind("hash")
        .bind("owner")
        .execute(&pool)
        .await
        .expect("Failed to insert user");
    
    // Create category
    let category_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO catalog_categories (id, name_en, name_pl, name_uk, name_ru, sort_order) 
         VALUES ($1, 'Test Cat', 'Test Cat', 'Test Cat', 'Test Cat', 1)"
    )
    .bind(category_id)
    .execute(&pool)
    .await
    .expect("Failed to insert category");
    
    // Create ingredient
    let ingredient_id = Uuid::new_v4();
    let ingredient_name = format!("Test Ingredient {}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO catalog_ingredients (id, category_id, name_en, name_pl, name_uk, name_ru, default_unit) 
         VALUES ($1, $2, $3, $4, $5, $6, $7::unit_type)"
    )
    .bind(ingredient_id)
    .bind(category_id)
    .bind(&ingredient_name)
    .bind(&ingredient_name)
    .bind(&ingredient_name)
    .bind(&ingredient_name)
    .bind("kilogram")
    .execute(&pool)
    .await
    .expect("Failed to insert test ingredient");
    
    let catalog_id = CatalogIngredientId::from_uuid(ingredient_id);

    // --- TEST A: Batch Creation ---
    println!("✅ Running Test A: Batch Creation...");
    let batch1_id = inventory_service.add_batch(
        user_id, tenant_id, catalog_id, 1000, 10.0, 
        Some("Supplier A".to_string()), Some("INV-001".to_string()),
        OffsetDateTime::now_utc(), None
    ).await.expect("Failed to add batch 1");

    let batch2_id = inventory_service.add_batch(
        user_id, tenant_id, catalog_id, 1500, 5.0, 
        Some("Supplier B".to_string()), Some("INV-002".to_string()),
        OffsetDateTime::now_utc(), None
    ).await.expect("Failed to add batch 2");

    let batches = inventory_service.list_products(user_id, tenant_id).await.expect("Failed to list");
    // Filter by catalog_id to be safe
    let b1 = batches.iter().find(|b| b.id == batch1_id).expect("Batch 1 not found");
    let b2 = batches.iter().find(|b| b.id == batch2_id).expect("Batch 2 not found");
    
    assert_eq!(b1.remaining_quantity.value(), 10.0);
    assert_eq!(b1.quantity.value(), 10.0);
    assert_eq!(b1.status, BatchStatus::Active);
    
    assert_eq!(b2.remaining_quantity.value(), 5.0);
    assert_eq!(b2.quantity.value(), 5.0);
    assert_eq!(b2.status, BatchStatus::Active);

    // --- TEST 3: Partial Deduction ---
    println!("✅ Running Test 3: Partial Deduction...");
    inventory_service.deduct_fifo(tenant_id, catalog_id, 3.0, None, None, None).await.expect("Deduction failed");
    
    let b1_after = inventory_service.get_product(user_id, tenant_id, batch1_id).await.unwrap().unwrap();
    assert_eq!(b1_after.remaining_quantity.value(), 7.0);
    assert_eq!(b1_after.status, BatchStatus::Active);

    // --- TEST B: FIFO Deduction (Cross-batch) ---
    println!("✅ Running Test B: FIFO Deduction (Cross-batch)...");
    // Current: B1(7kg), B2(5kg). Deduct 9kg.
    // Result: B1(0kg), B2(3kg)
    inventory_service.deduct_fifo(tenant_id, catalog_id, 9.0, None, None, None).await.expect("Deduction failed");
    
    let b1_final = inventory_service.get_product(user_id, tenant_id, batch1_id).await.unwrap().unwrap();
    let b2_intermediate = inventory_service.get_product(user_id, tenant_id, batch2_id).await.unwrap().unwrap();
    
    assert_eq!(b1_final.remaining_quantity.value(), 0.0);
    assert_eq!(b1_final.status, BatchStatus::Exhausted);
    assert_eq!(b2_intermediate.remaining_quantity.value(), 3.0);
    assert_eq!(b2_intermediate.status, BatchStatus::Active);

    // --- TEST 4: Exhaustion Status ---
    println!("✅ Running Test 4: Exhaustion Status...");
    inventory_service.deduct_fifo(tenant_id, catalog_id, 3.0, None, None, None).await.expect("Deduction failed");
    let b2_final = inventory_service.get_product(user_id, tenant_id, batch2_id).await.unwrap().unwrap();
    assert_eq!(b2_final.remaining_quantity.value(), 0.0);
    assert_eq!(b2_final.status, BatchStatus::Exhausted);

    // --- TEST 6: Decimal Accuracy ---
    println!("✅ Running Test 6: Decimal Accuracy...");
    let batch3_id = inventory_service.add_batch(
        user_id, tenant_id, catalog_id, 1000, 1.333, 
        None, None, OffsetDateTime::now_utc(), None
    ).await.expect("Failed to add batch 3");
    
    let batch4_id = inventory_service.add_batch(
        user_id, tenant_id, catalog_id, 1000, 0.667, 
        None, None, OffsetDateTime::now_utc(), None
    ).await.expect("Failed to add batch 4");
    
    // Total is exactly 2.0
    inventory_service.deduct_fifo(tenant_id, catalog_id, 2.0, None, None, None).await.expect("Deduction failed");
    
    let b3 = inventory_service.get_product(user_id, tenant_id, batch3_id).await.unwrap().unwrap();
    let b4 = inventory_service.get_product(user_id, tenant_id, batch4_id).await.unwrap().unwrap();
    assert_eq!(b3.remaining_quantity.value(), 0.0);
    assert_eq!(b4.remaining_quantity.value(), 0.0);
    assert_eq!(b3.status, BatchStatus::Exhausted);
    assert_eq!(b4.status, BatchStatus::Exhausted);

    // --- TEST 2: Concurrency ---
    println!("✅ Running Test 2: Concurrency...");
    let batch5_id = inventory_service.add_batch(
        user_id, tenant_id, catalog_id, 1000, 10.0, 
        None, None, OffsetDateTime::now_utc(), None
    ).await.expect("Failed to add batch 5");
    
    let service = Arc::new(inventory_service.clone());
    let mut handles = vec![];
    
    for _ in 0..2 {
        let s = service.clone();
        let tid = tenant_id;
        let cid = catalog_id;
        handles.push(tokio::spawn(async move {
            s.deduct_fifo(tid, cid, 6.0, None, None, None).await
        }));
    }
    
    let mut successes = 0;
    let mut failures = 0;
    for h in handles {
        match h.await.unwrap() {
            Ok(_) => successes += 1,
            Err(_) => failures += 1,
        }
    }
    
    assert_eq!(successes, 1, "Exactly one deduction should succeed");
    assert_eq!(failures, 1, "One deduction should fail due to insufficient stock (10 - 6 - 6 = -2)");
    
    let b5 = inventory_service.get_product(user_id, tenant_id, batch5_id).await.unwrap().unwrap();
    assert_eq!(b5.remaining_quantity.value(), 4.0);

    // --- TEST 5: Multi-tenant Isolation ---
    println!("✅ Running Test 5: Multi-tenant Isolation...");
    let tenant_b_id = TenantId::new();
    sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, $2)")
        .bind(tenant_b_id.as_uuid())
        .bind("Tenant B")
        .execute(&pool)
        .await
        .expect("Failed to insert tenant B");
        
    let batch_b_id = inventory_service.add_batch(
        user_id, tenant_b_id, catalog_id, 1000, 20.0, 
        None, None, OffsetDateTime::now_utc(), None
    ).await.expect("Failed to add batch for Tenant B");
    
    // Attempt to deduct from Tenant A (which has 4kg left of B5)
    // Deduction of 5kg should fail for Tenant A but shouldn't affect Tenant B
    let result_a = inventory_service.deduct_fifo(tenant_id, catalog_id, 5.0, None, None, None).await;
    assert!(result_a.is_err(), "Tenant A should not have enough stock");
    
    // Tenant B should still have 20.0
    let b_stock = inventory_service.get_product(user_id, tenant_b_id, batch_b_id).await.unwrap().unwrap();
    assert_eq!(b_stock.remaining_quantity.value(), 20.0);

    println!("🚀 ALL INVENTORY TESTS PASSED!");
}
