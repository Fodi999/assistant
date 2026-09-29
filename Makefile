.PHONY: help db-up db-down run test lint fmt check

help:
	@echo "targets: db-up db-down run test lint fmt check"

db-up:
	docker compose up -d db

db-down:
	docker compose down

run:
	cargo run

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

check: fmt lint test
