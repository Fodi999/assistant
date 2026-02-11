// Utility to generate Argon2 password hash for Super Admin
// Usage: cargo run --bin generate_admin_hash

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

fn main() {
    let password = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Admin123!".to_string());

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();

    println!("Password: {}", password);
    println!("Argon2 Hash: {}", password_hash);
    println!("\nAdd to .env:");
    println!("ADMIN_PASSWORD_HASH='{}'", password_hash);
}
