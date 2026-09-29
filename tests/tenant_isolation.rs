//! Row-level-security tests. They need a PostgreSQL server (DATABASE_URL) whose
//! user may create databases and roles (the CI/docker-compose superuser does).
//! Every check runs as the limited `beauty_app` role, exactly like production.

use beauty_backend::infrastructure::{apply_scope, DbScope};
use beauty_backend::shared::{BusinessId, UserId};
use sqlx::{PgPool, Postgres, Transaction};

struct World {
    ann: UserId,
    bob: UserId,
    biz_a: BusinessId,
    biz_b: BusinessId,
    biz_c: BusinessId,
}

/// Seeds two people and three businesses as the (RLS-exempt) test superuser:
///   ann: owner of A, employee of B      bob: owner of B      C: nobody
async fn seed(pool: &PgPool) -> World {
    let w = World {
        ann: UserId::new(),
        bob: UserId::new(),
        biz_a: BusinessId::new(),
        biz_b: BusinessId::new(),
        biz_c: BusinessId::new(),
    };

    for (id, email) in [(w.ann, "ann@example.pl"), (w.bob, "bob@example.pl")] {
        sqlx::query("INSERT INTO users (id, email) VALUES ($1, $2)")
            .bind(id.as_uuid())
            .bind(email)
            .execute(pool)
            .await
            .unwrap();
    }
    for (id, slug) in [(w.biz_a, "biz-a"), (w.biz_b, "biz-b"), (w.biz_c, "biz-c")] {
        sqlx::query("INSERT INTO business (id, name, slug) VALUES ($1, $2, $2)")
            .bind(id.as_uuid())
            .bind(slug)
            .execute(pool)
            .await
            .unwrap();
    }
    for (biz, user, role) in [
        (w.biz_a, w.ann, "owner"),
        (w.biz_b, w.bob, "owner"),
        (w.biz_b, w.ann, "employee"),
    ] {
        sqlx::query("INSERT INTO membership (business_id, user_id, role) VALUES ($1, $2, $3)")
            .bind(biz.as_uuid())
            .bind(user.as_uuid())
            .bind(role)
            .execute(pool)
            .await
            .unwrap();
    }
    for (biz, name) in [(w.biz_a, "Anna"), (w.biz_b, "Olga")] {
        sqlx::query("INSERT INTO staff_member (business_id, display_name) VALUES ($1, $2)")
            .bind(biz.as_uuid())
            .bind(name)
            .execute(pool)
            .await
            .unwrap();
    }
    w
}

/// Opens a transaction as the application role with the given scope.
async fn as_app(pool: &PgPool, scope: DbScope) -> Transaction<'static, Postgres> {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE beauty_app")
        .execute(&mut *tx)
        .await
        .unwrap();
    apply_scope(&mut tx, scope).await.unwrap();
    tx
}

fn sqlstate(err: &sqlx::Error) -> Option<String> {
    err.as_database_error()
        .and_then(|e| e.code())
        .map(|c| c.to_string())
}

#[sqlx::test(migrations = "./migrations")]
async fn business_scope_sees_only_its_own_staff(pool: PgPool) {
    let w = seed(&pool).await;
    let mut tx = as_app(&pool, DbScope::business(w.ann, w.biz_a)).await;

    let names: Vec<String> = sqlx::query_scalar("SELECT display_name FROM staff_member")
        .fetch_all(&mut *tx)
        .await
        .unwrap();

    assert_eq!(names, vec!["Anna".to_string()]);
}

#[sqlx::test(migrations = "./migrations")]
async fn no_scope_fails_closed(pool: PgPool) {
    seed(&pool).await;
    let mut tx = as_app(&pool, DbScope::anonymous()).await;

    for table in [
        "staff_member",
        "business",
        "membership",
        "users",
        "audit_log",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(count, 0, "{table} must be empty without a scope");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn cannot_insert_into_another_business(pool: PgPool) {
    let w = seed(&pool).await;
    let mut tx = as_app(&pool, DbScope::business(w.ann, w.biz_a)).await;

    let err =
        sqlx::query("INSERT INTO staff_member (business_id, display_name) VALUES ($1, 'Evil')")
            .bind(w.biz_b.as_uuid())
            .execute(&mut *tx)
            .await
            .unwrap_err();

    assert_eq!(sqlstate(&err).as_deref(), Some("42501"), "{err}");
}

#[sqlx::test(migrations = "./migrations")]
async fn cannot_update_another_business(pool: PgPool) {
    let w = seed(&pool).await;
    let mut tx = as_app(&pool, DbScope::business(w.ann, w.biz_a)).await;

    let result = sqlx::query("UPDATE staff_member SET display_name = 'x' WHERE business_id = $1")
        .bind(w.biz_b.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();

    assert_eq!(result.rows_affected(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn user_scope_lists_only_own_memberships_and_businesses(pool: PgPool) {
    let w = seed(&pool).await;
    let mut tx = as_app(&pool, DbScope::user(w.ann)).await;

    let mut memberships: Vec<uuid::Uuid> = sqlx::query_scalar("SELECT business_id FROM membership")
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    memberships.sort();
    let mut expected = vec![*w.biz_a.as_uuid(), *w.biz_b.as_uuid()];
    expected.sort();
    assert_eq!(memberships, expected);

    let slugs: Vec<String> = sqlx::query_scalar("SELECT slug FROM business ORDER BY slug")
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    assert_eq!(slugs, vec!["biz-a".to_string(), "biz-b".to_string()]);
    assert!(!slugs.contains(&"biz-c".to_string()));
    let _ = w.biz_c;
}

#[sqlx::test(migrations = "./migrations")]
async fn users_are_visible_only_to_themselves_and_coworkers(pool: PgPool) {
    let w = seed(&pool).await;

    // In business A, ann works alone: she must not see bob.
    let mut tx = as_app(&pool, DbScope::business(w.ann, w.biz_a)).await;
    let emails: Vec<String> = sqlx::query_scalar("SELECT email FROM users ORDER BY email")
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    assert_eq!(emails, vec!["ann@example.pl".to_string()]);
    drop(tx);

    // In business B, bob is a coworker: both are visible.
    let mut tx = as_app(&pool, DbScope::business(w.ann, w.biz_b)).await;
    let emails: Vec<String> = sqlx::query_scalar("SELECT email FROM users ORDER BY email")
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    assert_eq!(
        emails,
        vec!["ann@example.pl".to_string(), "bob@example.pl".to_string()]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn audit_log_is_append_only(pool: PgPool) {
    let w = seed(&pool).await;
    let mut tx = as_app(&pool, DbScope::business(w.ann, w.biz_a)).await;

    sqlx::query(
        "INSERT INTO audit_log (business_id, actor_user_id, action, entity) VALUES ($1, $2, 'test', 'x')",
    )
    .bind(w.biz_a.as_uuid())
    .bind(w.ann.as_uuid())
    .execute(&mut *tx)
    .await
    .unwrap();

    let err = sqlx::query("UPDATE audit_log SET action = 'tamper'")
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(sqlstate(&err).as_deref(), Some("42501"), "{err}");
}

#[sqlx::test(migrations = "./migrations")]
async fn business_creation_flow_and_foreign_id_rejection(pool: PgPool) {
    let w = seed(&pool).await;
    let new_biz = BusinessId::new();

    // Creating a business: scope points at the new id, then business + owner.
    let mut tx = as_app(&pool, DbScope::business(w.bob, new_biz)).await;
    sqlx::query("INSERT INTO business (id, name, slug) VALUES ($1, 'New', 'new-biz')")
        .bind(new_biz.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO membership (business_id, user_id, role) VALUES ($1, $2, 'owner')")
        .bind(new_biz.as_uuid())
        .bind(w.bob.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // A scope for business A cannot create a business under a different id.
    let mut tx = as_app(&pool, DbScope::business(w.ann, w.biz_a)).await;
    let err = sqlx::query("INSERT INTO business (id, name, slug) VALUES ($1, 'Sneaky', 'sneaky')")
        .bind(BusinessId::new().as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(sqlstate(&err).as_deref(), Some("42501"), "{err}");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_user_can_only_register_their_own_row(pool: PgPool) {
    let me = UserId::new();
    let mut tx = as_app(&pool, DbScope::user(me)).await;

    sqlx::query("INSERT INTO users (id, email) VALUES ($1, 'me@example.pl')")
        .bind(me.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();

    let err = sqlx::query("INSERT INTO users (id, email) VALUES ($1, 'other@example.pl')")
        .bind(UserId::new().as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(sqlstate(&err).as_deref(), Some("42501"), "{err}");
}
