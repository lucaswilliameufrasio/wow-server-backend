# Dev setup (AzerothCore + backend + item seed)

## 1) Bootstrap AzerothCore (docker compose + DB import/migrations)

```bash
./scripts/bootstrap-azerothcore.sh
```

What it does:
- Clones/updates `azerothcore-wotlk` in `/tmp/azerothcore-wotlk` by default
- Runs its official `docker-compose.yml`
- Waits for `ac-database` MySQL readiness

Overrides:
- `ACORE_REPO_DIR`
- `ACORE_REPO_URL`
- `ACORE_REF`

## 2) Seed items from `thatsmybis/wotlk-item-db`

```bash
./scripts/seed-items.sh
```

What it does:
- Clones/updates `wotlk-item-db` in `/tmp/wotlk-item-db` by default
- Detects a `.sql` file (or uses `ITEM_SQL_FILE` if provided)
- Imports into MySQL DB `wotlk_items` by default

Overrides:
- `ACORE_REPO_DIR`
- `ITEM_REPO_DIR`
- `ITEM_REPO_URL`
- `ITEM_REPO_REF`
- `ITEM_DB_NAME`
- `ITEM_SQL_FILE`

## 3) Run this backend against AzerothCore DB network

Generate RSA keys for JWT (required):

```bash
mkdir -p .secrets
openssl genrsa -out .secrets/jwt_private.pem 2048
openssl rsa -in .secrets/jwt_private.pem -pubout -out .secrets/jwt_public.pem
```

Start API with compose:

```bash
docker compose -f docker-compose.dev.yml up --build
```

Or run the full local setup sequence:

```bash
make setup
```

Notes:
- `docker-compose.dev.yml` runs an app PostgreSQL container for app-only data (refresh/revocation tables).
- `docker-compose.dev.yml` expects the AzerothCore compose network `acore-docker_default` by default.
- Override network with `ACORE_DOCKER_NETWORK` if your project name differs.
- Core envs are explicit:
  - `AZEROTH_CORE_MYSQL_DATABASE_URL`
  - `AZEROTH_CORE_AUTH_DB`
  - `AZEROTH_CORE_CHARACTERS_DB`
  - `AZEROTH_CORE_WORLD_DB`
- App Postgres envs:
  - `APP_POSTGRES_DATABASE_URL`
- Schema is fixed to `public` for migration/runtime consistency.
- App schema is managed by SQLx migrations (not runtime auto-DDL).
