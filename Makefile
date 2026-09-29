.PHONY: help db-up db-down db-reset migrate run test lint fmt check

SUPERUSER_URL ?= postgres://beauty:beauty@localhost:5432/beauty

help:
	@echo "targets: db-up db-down db-reset migrate run test lint fmt check"

db-up:
	docker compose up -d db

db-down:
	docker compose down

# Destroys local data and recreates the roles from docker/initdb.
db-reset:
	docker compose down -v
	docker compose up -d db

# Applies migrations as the schema owner (never at process start).
migrate:
	MIGRATION_DATABASE_URL=$(SUPERUSER_URL) cargo run --bin migrate

run:
	cargo run

# The RLS tests create throw-away databases, so they need the superuser URL.
test:
	DATABASE_URL=$(SUPERUSER_URL) cargo test

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

check: fmt lint test
