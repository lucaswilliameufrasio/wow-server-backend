#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/wow-common.sh
source "$ROOT_DIR/scripts/wow-common.sh"

WORLD_DB_NAME="${WORLD_DB_NAME:-acore_world}"
VENDOR_SQL_FILE="${VENDOR_SQL_FILE:-$ROOT_DIR/sql/fast_raid_vendors.sql}"

if [ ! -f "$ACORE_REPO_DIR/docker-compose.yml" ]; then
  echo "AzerothCore docker-compose.yml not found at $ACORE_REPO_DIR" >&2
  echo "Run scripts/bootstrap-azerothcore.sh first or set ACORE_REPO_DIR" >&2
  exit 1
fi

if [ ! -f "$VENDOR_SQL_FILE" ]; then
  echo "Vendor SQL file not found: $VENDOR_SQL_FILE" >&2
  exit 1
fi

echo "Seeding fast raid-prep vendors into DB: $WORLD_DB_NAME"
docker compose -f "$ACORE_REPO_DIR/docker-compose.yml" exec -T ac-database \
  sh -lc "mysql -uroot -p\"$MYSQL_ROOT_PASSWORD\" $WORLD_DB_NAME" < "$VENDOR_SQL_FILE"

echo "Ensuring FULLTEXT index on item_template.name (API item search)"
docker compose -f "$ACORE_REPO_DIR/docker-compose.yml" exec -T ac-database \
  sh -lc "mysql -uroot -p\"$MYSQL_ROOT_PASSWORD\" $WORLD_DB_NAME" < "$ROOT_DIR/sql/fulltext_item_template_name.sql"

echo "Done. In-game GM commands (optional immediate refresh):"
echo "  .reload creature_template"
echo "  .reload creature"
echo "  .reload npc_vendor"
