SHELL := /bin/bash

.PHONY: help dev test migrate-up migrate-down migration-create setup deps bootstrap seed jwt-keys app-postgres-up app-postgres-wait

AZEROTH_CORE_MYSQL_DATABASE_URL ?= mysql://root:password@127.0.0.1:3306
APP_POSTGRES_DATABASE_URL ?= postgres://postgres:password@127.0.0.1:5432/wow_app
APP_POSTGRES_USER ?= postgres
MIGRATIONS_DIR ?= migrations
MIGRATION_NAME ?= $(or $(NAME),$(name))

help:
	@echo "Available targets:"
	@echo "  make dev                - Run API locally with cargo run"
	@echo "  make test               - Run tests with cargo nextest"
	@echo "  make migrate-up         - Apply sqlx migrations"
	@echo "  make migrate-down       - Revert last sqlx migration"
	@echo "  make migration-create NAME=<name> - Create a new sqlx migration"
	@echo "  make app-postgres-up    - Start app Postgres container"
	@echo "  make app-postgres-wait  - Wait for app Postgres readiness"
	@echo "  make setup              - Install tools, bootstrap AzerothCore, migrations, and item seed"

dev:
	cargo run

test:
	cargo nextest run --all-targets

migrate-up:
	@[ -d "$(MIGRATIONS_DIR)" ] || mkdir -p "$(MIGRATIONS_DIR)"
	DATABASE_URL="$(APP_POSTGRES_DATABASE_URL)" sqlx migrate run --source "$(MIGRATIONS_DIR)"

migrate-down:
	@[ -d "$(MIGRATIONS_DIR)" ] || mkdir -p "$(MIGRATIONS_DIR)"
	DATABASE_URL="$(APP_POSTGRES_DATABASE_URL)" sqlx migrate revert --source "$(MIGRATIONS_DIR)"

migration-create:
	@if [ -z "$(MIGRATION_NAME)" ]; then \
		echo "Usage: make migration-create NAME=create_refresh_tables"; \
		exit 1; \
	fi
	@[ -d "$(MIGRATIONS_DIR)" ] || mkdir -p "$(MIGRATIONS_DIR)"
	sqlx migrate add "$(MIGRATION_NAME)" --source "$(MIGRATIONS_DIR)"

deps:
	@command -v cargo >/dev/null 2>&1 || { echo "cargo is required"; exit 1; }
	@command -v docker >/dev/null 2>&1 || { echo "docker is required"; exit 1; }
	@command -v openssl >/dev/null 2>&1 || { echo "openssl is required"; exit 1; }
	@command -v cargo-nextest >/dev/null 2>&1 || cargo install --locked cargo-nextest
	@command -v sqlx >/dev/null 2>&1 || cargo install --locked sqlx-cli --no-default-features --features rustls,postgres,mysql

jwt-keys:
	@mkdir -p .secrets
	@test -f .secrets/jwt_private.pem || openssl genrsa -out .secrets/jwt_private.pem 2048
	@test -f .secrets/jwt_public.pem || openssl rsa -in .secrets/jwt_private.pem -pubout -out .secrets/jwt_public.pem

bootstrap:
	./scripts/bootstrap-azerothcore.sh

app-postgres-up:
	docker compose -f docker-compose.dev.yml up -d app-postgres

app-postgres-wait:
	@echo "Waiting for app-postgres..."
	@for i in $$(seq 1 60); do \
		if docker compose -f docker-compose.dev.yml exec -T app-postgres pg_isready -U "$(APP_POSTGRES_USER)" >/dev/null 2>&1; then \
			echo "app-postgres is ready"; \
			exit 0; \
		fi; \
		sleep 2; \
	done; \
	echo "app-postgres did not become ready in time" >&2; \
	exit 1

seed:
	./scripts/seed-items.sh

setup: deps jwt-keys bootstrap app-postgres-up app-postgres-wait migrate-up seed
	@echo "Setup complete."
