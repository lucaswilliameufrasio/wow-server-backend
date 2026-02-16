#!/usr/bin/env bash
set -euo pipefail

ACORE_REPO_DIR="${ACORE_REPO_DIR:-/tmp/azerothcore-wotlk}"
ACORE_REPO_URL="${ACORE_REPO_URL:-https://github.com/azerothcore/azerothcore-wotlk.git}"
ACORE_REF="${ACORE_REF:-master}"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required" >&2
  exit 1
fi

if ! command -v git >/dev/null 2>&1; then
  echo "git is required" >&2
  exit 1
fi

if [ ! -d "$ACORE_REPO_DIR/.git" ]; then
  echo "Cloning AzerothCore into $ACORE_REPO_DIR"
  git clone --depth 1 --branch "$ACORE_REF" "$ACORE_REPO_URL" "$ACORE_REPO_DIR"
else
  echo "Updating AzerothCore in $ACORE_REPO_DIR"
  git -C "$ACORE_REPO_DIR" fetch --depth 1 origin "$ACORE_REF"
  git -C "$ACORE_REPO_DIR" checkout "$ACORE_REF"
  git -C "$ACORE_REPO_DIR" pull --ff-only origin "$ACORE_REF"
fi

echo "Starting AzerothCore docker compose (includes db import/migrations on first run)..."
docker compose -f "$ACORE_REPO_DIR/docker-compose.yml" up -d --build

echo "Waiting for MySQL in ac-database..."
for _ in $(seq 1 120); do
  if docker compose -f "$ACORE_REPO_DIR/docker-compose.yml" exec -T ac-database \
      mysqladmin ping -h127.0.0.1 -uroot -ppassword >/dev/null 2>&1; then
    echo "MySQL is ready"
    exit 0
  fi
  sleep 5
done

echo "MySQL did not become ready in time" >&2
exit 1
