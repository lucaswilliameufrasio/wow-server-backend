SHELL := /bin/bash

.PHONY: help dev test migrate-up migrate-down migration-create setup deps bootstrap seed seed-fast-raid-vendors seed-spec-bis-vendors jwt-keys app-postgres-up app-postgres-wait preflight-update backup-before-update weekend-go update-server update-azerothcore update-backend docker-build docker-publish docker-ship docker-run docker-compose-prod-up docker-compose-prod-down

AZEROTH_CORE_MYSQL_DATABASE_URL ?= mysql://root:password@127.0.0.1:3306
APP_POSTGRES_DATABASE_URL ?= postgres://postgres:password@127.0.0.1:5432/wow_app
APP_POSTGRES_USER ?= postgres
MIGRATIONS_DIR ?= migrations
MIGRATION_NAME ?= $(or $(NAME),$(name))
SQLX_CLI_VERSION ?= 0.8.6
IMAGE_NAME ?= wow-server-backend
TAG ?= latest
TARGET_PLATFORM ?= linux/amd64
SSH_USER ?= root
SSH_HOST ?= your-remote-host
SSH_ALIAS ?=
ENV ?= .env

ifeq ($(SSH_ALIAS),)
	SSH_TARGET := $(SSH_USER)@$(SSH_HOST)
else
	SSH_TARGET := $(SSH_ALIAS)
endif

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
	@echo "  make seed-fast-raid-vendors - Spawn class/misc raid prep vendors in AzerothCore world DB"
	@echo "  make seed-spec-bis-vendors - Spawn fixed ICC-style spec preset vendors"
	@echo "  make preflight-update   - Validate existing-server update prerequisites"
	@echo "  make backup-before-update - Backup CNPG Postgres + AzerothCore MySQL"
	@echo "  make weekend-go         - Preflight + backup + update + vendor seed + smoke checks"
	@echo "  make update-server      - Update AzerothCore + backend rollout/migrations on existing server"
	@echo "  make update-azerothcore - Update only AzerothCore (repo + compose + DB updates)"
	@echo "  make update-backend     - Update only backend (image import + app migrations + rollout)"
	@echo "  make docker-build       - Build production image locally"
	@echo "  make docker-publish     - Build and push image to registry"
	@echo "  make docker-ship        - Build image and load to remote Docker via SSH"
	@echo "  make docker-run         - Run container locally with env file"
	@echo "  make docker-compose-prod-up   - Start prod compose (app + postgres)"
	@echo "  make docker-compose-prod-down - Stop prod compose"

dev:
	cargo run

test:
	@if command -v cargo-nextest >/dev/null 2>&1; then \
		cargo nextest run --all-targets; \
	else \
		cargo test; \
	fi

coverage:
	@if command -v cargo-llvm-cov >/dev/null 2>&1; then \
		cargo llvm-cov --all-targets; \
	else \
		echo "cargo-llvm-cov not installed. Install with: cargo install cargo-llvm-cov"; \
		exit 1; \
	fi

coverage-html:
	@if command -v cargo-llvm-cov >/dev/null 2>&1; then \
		cargo llvm-cov --all-targets --open; \
	else \
		echo "cargo-llvm-cov not installed. Install with: cargo install cargo-llvm-cov"; \
		exit 1; \
	fi

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
	@command -v sqlx >/dev/null 2>&1 || cargo install --locked sqlx-cli --version $(SQLX_CLI_VERSION) --no-default-features --features rustls,postgres,mysql

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

seed-fast-raid-vendors:
	./scripts/seed-fast-raid-vendors.sh

seed-spec-bis-vendors:
	./scripts/seed-spec-bis-vendors.sh

setup: deps jwt-keys bootstrap app-postgres-up app-postgres-wait migrate-up seed
	@echo "Setup complete."

backup-before-update:
	./scripts/backup-before-update.sh

weekend-go:
	./scripts/weekend-go.sh

update-server:
	./scripts/preflight-update.sh
	./scripts/update-existing-server.sh

update-azerothcore:
	UPDATE_AZEROTHCORE=true UPDATE_BACKEND=false ./scripts/preflight-update.sh
	UPDATE_AZEROTHCORE=true UPDATE_BACKEND=false ./scripts/update-existing-server.sh

update-backend:
	UPDATE_AZEROTHCORE=false UPDATE_BACKEND=true ./scripts/preflight-update.sh
	UPDATE_AZEROTHCORE=false UPDATE_BACKEND=true ./scripts/update-existing-server.sh

preflight-update:
	./scripts/preflight-update.sh

docker-build:
	@docker buildx inspect container-builder >/dev/null 2>&1 || docker buildx create --name container-builder --driver docker-container --use
	docker buildx build --builder container-builder --platform $(TARGET_PLATFORM) \
		-t $(IMAGE_NAME):$(TAG) \
		-f Dockerfile.production \
		--load .

docker-publish:
	@docker buildx inspect container-builder >/dev/null 2>&1 || docker buildx create --name container-builder --driver docker-container --use
	docker buildx build --builder container-builder --platform $(TARGET_PLATFORM) \
		-t $(IMAGE_NAME):$(TAG) \
		-f Dockerfile.production \
		--push .

docker-ship:
	@docker buildx inspect container-builder >/dev/null 2>&1 || docker buildx create --name container-builder --driver docker-container --use
	docker buildx build --builder container-builder --platform $(TARGET_PLATFORM) \
		-t $(IMAGE_NAME):$(TAG) \
		-f Dockerfile.production \
		--output type=docker,dest=- . | ssh $(SSH_TARGET) "docker load"

docker-run:
	@docker run --rm -p 3000:3000 \
		$(shell grep -v '^#' $(ENV) | grep -v '^$$' | xargs -I {} echo "-e {}" | tr '\n' ' ') \
		$(IMAGE_NAME):$(TAG)

docker-compose-prod-up:
	docker compose -f docker-compose.prod.yml up -d

docker-compose-prod-down:
	docker compose -f docker-compose.prod.yml down
