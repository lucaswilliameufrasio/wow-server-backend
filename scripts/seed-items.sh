#!/usr/bin/env bash
set -euo pipefail

ACORE_REPO_DIR="${ACORE_REPO_DIR:-/tmp/azerothcore-wotlk}"
ITEM_REPO_DIR="${ITEM_REPO_DIR:-/tmp/wotlk-item-db}"
ITEM_REPO_URL="${ITEM_REPO_URL:-https://github.com/thatsmybis/wotlk-item-db.git}"
ITEM_REPO_REF="${ITEM_REPO_REF:-master}"
ITEM_DB_NAME="${ITEM_DB_NAME:-wotlk_items}"
ITEM_SQL_FILE="${ITEM_SQL_FILE:-}"

if [ ! -f "$ACORE_REPO_DIR/docker-compose.yml" ]; then
  echo "AzerothCore docker-compose.yml not found at $ACORE_REPO_DIR" >&2
  echo "Run scripts/bootstrap-azerothcore.sh first or set ACORE_REPO_DIR" >&2
  exit 1
fi

if [ -z "$ITEM_SQL_FILE" ]; then
  if [ ! -d "$ITEM_REPO_DIR/.git" ]; then
    echo "Cloning item DB repo into $ITEM_REPO_DIR"
    git clone --depth 1 --branch "$ITEM_REPO_REF" "$ITEM_REPO_URL" "$ITEM_REPO_DIR"
  else
    echo "Updating item DB repo in $ITEM_REPO_DIR"
    git -C "$ITEM_REPO_DIR" fetch --depth 1 origin "$ITEM_REPO_REF"
    git -C "$ITEM_REPO_DIR" checkout "$ITEM_REPO_REF"
    git -C "$ITEM_REPO_DIR" pull --ff-only origin "$ITEM_REPO_REF"
  fi

  mapfile -t sql_files < <(find "$ITEM_REPO_DIR" -type f -name '*.sql' | sort)
  if [ "${#sql_files[@]}" -eq 0 ]; then
    echo "No .sql files found in $ITEM_REPO_DIR. Set ITEM_SQL_FILE explicitly." >&2
    exit 1
  fi

  ITEM_SQL_FILE="${sql_files[0]}"
  echo "Using detected SQL file: $ITEM_SQL_FILE"
else
  if [ ! -f "$ITEM_SQL_FILE" ]; then
    echo "ITEM_SQL_FILE does not exist: $ITEM_SQL_FILE" >&2
    exit 1
  fi
fi

echo "Creating target DB if needed: $ITEM_DB_NAME"
docker compose -f "$ACORE_REPO_DIR/docker-compose.yml" exec -T ac-database \
  sh -lc "mysql -uroot -ppassword -e \"CREATE DATABASE IF NOT EXISTS \\\`$ITEM_DB_NAME\\\`;\""

echo "Importing items SQL into $ITEM_DB_NAME from $ITEM_SQL_FILE"
docker compose -f "$ACORE_REPO_DIR/docker-compose.yml" exec -T ac-database \
  sh -lc "mysql -uroot -ppassword $ITEM_DB_NAME" < "$ITEM_SQL_FILE"

echo "Item seed import completed"
