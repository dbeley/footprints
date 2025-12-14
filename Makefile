.PHONY: help build test run clean docker-build docker-up docker-down fmt clippy

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build the project in release mode
	cargo build --release

test: ## Run all tests
	cargo test

run: ## Run the application in development mode
	cargo run

clean: ## Clean build artifacts and database files
	cargo clean
	rm -f footprints.db*

docker-build: ## Build Docker image
	docker build -t footprints:latest .

docker-up: ## Start the application with docker-compose
	docker-compose up -d

docker-down: ## Stop the application
	docker-compose down

fmt: ## Format code with rustfmt
	cargo fmt

clippy: ## Run clippy linter
	cargo clippy -- -D warnings

check: fmt clippy test ## Run all checks (format, lint, test)
