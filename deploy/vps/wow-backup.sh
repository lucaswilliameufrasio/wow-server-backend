#!/usr/bin/env bash
# wow-backup.sh — scheduled backup of PostgreSQL (API) and MySQL (AzerothCore).
# Runs inside the wow-backup container (cron). Requires env vars set by compose.
set -euo pipefail

BACKUP_DIR="${BACKUP_DIR:-/backups}"
RETENTION_DAYS="${WOW_BACKUP_RETENTION_DAYS:-30}"
PG_HOST="${PG_HOST:-app-postgres}"
PG_USER="${WOW_PG_USER:-wow_api}"
PG_PASSWORD="${WOW_PG_PASSWORD:-}"
MYSQL_HOST="${MYSQL_HOST:-ac-database}"
MYSQL_ROOT_PASSWORD="${WOW_MYSQL_PASSWORD:-}"

mkdir -p "$BACKUP_DIR"
ts="$(date +%Y%m%d_%H%M%S)"
out="$BACKUP_DIR/$ts"
mkdir -p "$out"

echo "[backup] $ts — starting"

if [[ -n "$PG_PASSWORD" ]]; then
  PGPASSWORD="$PG_PASSWORD" pg_dumpall -h "$PG_HOST" -U "$PG_USER" \
    > "$out/postgres_all.sql" 2>/dev/null \
    && echo "[backup] postgres ok" || echo "[backup] postgres FAILED"
else
  echo "[backup] WOW_PG_PASSWORD not set — skipping postgres"
fi

if [[ -n "$MYSQL_ROOT_PASSWORD" ]]; then
  mysqldump -h "$MYSQL_HOST" -uroot -p"$MYSQL_ROOT_PASSWORD" \
    --all-databases --routines --triggers --single-transaction \
    > "$out/mysql_all.sql" 2>/dev/null \
    && echo "[backup] mysql ok" || echo "[backup] mysql FAILED"
else
  echo "[backup] WOW_MYSQL_PASSWORD not set — skipping mysql"
fi

(cd "$BACKUP_DIR" && tar czf "$ts.tar.gz" "$ts" && rm -rf "$ts")
echo "[backup] archive created: $ts.tar.gz"

echo "[backup] pruning older than $RETENTION_DAYS days"
find "$BACKUP_DIR" -name '*.tar.gz' -type f -mtime "+$RETENTION_DAYS" -delete

echo "[backup] done"
