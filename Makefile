.PHONY: help db-up db-down db-reset migrate run test lint fmt check

SUPERUSER_URL ?= postgres://beauty:beauty@localhost:5433/beauty

help:
	@echo "targets: db-up db-down db-reset migrate run test lint fmt check"

db-up:
	docker compose up -d db
	@echo "waiting for Postgres (roles are created on first start)..."
	@i=0; until docker compose exec -T db pg_isready -q -h 127.0.0.1 -U beauty -d beauty; do \
		i=$$((i+1)); [ $$i -gt 60 ] && { echo "Postgres did not become ready"; exit 1; }; sleep 1; done
	@echo "Postgres is ready on localhost:5433"

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
test: db-up
	DATABASE_URL=$(SUPERUSER_URL) cargo test

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

check: fmt lint test
