.PHONY: help setup db-create db-migrate db-drop test run clean lint format check

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-15s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

setup: ## Install dependencies and setup project
	@echo "Installing sqlx-cli..."
	cargo install sqlx-cli --no-default-features --features postgres
	@echo "Copying .env.example to .env..."
	cp -n .env.example .env || true
	@echo "Setup complete! Edit .env file with your database credentials."

db-create: ## Create database
	createdb restaurant_db || echo "Database may already exist"

db-migrate: ## Run database migrations
	sqlx migrate run

db-drop: ## Drop database (WARNING: destructive)
	dropdb restaurant_db

db-reset: db-drop db-create db-migrate ## Reset database (drop, create, migrate)

test: ## Run tests
	cargo test

test-verbose: ## Run tests with output
	RUST_LOG=debug cargo test -- --nocapture

run: ## Run the application
	cargo run

dev: ## Run with auto-reload (requires cargo-watch)
	cargo watch -x run

build: ## Build the application
	cargo build --release

clean: ## Clean build artifacts
	cargo clean

lint: ## Run clippy
	cargo clippy -- -D warnings

format: ## Format code
	cargo fmt

check: format lint test ## Format, lint, and test

install-tools: ## Install development tools
	cargo install cargo-watch sqlx-cli --no-default-features --features postgres

docker-db: ## Start PostgreSQL in Docker
	docker run --name restaurant-postgres \
		-e POSTGRES_PASSWORD=postgres \
		-e POSTGRES_DB=restaurant_db \
		-p 5432:5432 \
		-d postgres:15

docker-db-stop: ## Stop PostgreSQL Docker container
	docker stop restaurant-postgres
	docker rm restaurant-postgres
