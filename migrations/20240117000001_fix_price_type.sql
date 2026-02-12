-- Change price type to NUMERIC for proper Decimal handling in Rust
ALTER TABLE catalog_ingredients
ALTER COLUMN price TYPE NUMERIC(10, 2);
