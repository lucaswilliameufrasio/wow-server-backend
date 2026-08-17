#!/usr/bin/env bash
# wow-common.sh — shared env resolution for WoW operational scripts.
# Source this file (it must NOT be executed directly).
#
# Resolves:
#   ROOT_DIR         repo root
#   ACORE_REPO_DIR   AzerothCore checkout (explicit env > deploy/vps/.env > /opt > /tmp)
#   MYSQL_ROOT_PASSWORD  root password of the ac-database container
#                       (explicit env > deploy/vps/.env WOW_MYSQL_PASSWORD > "password")
#
# This keeps every script consistent with what `./wowctl install` produces,
# so `make seed-*` / `scripts/*` work on a VPS installed via wowctl.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VPS_ENV="${VPS_ENV:-$ROOT_DIR/deploy/vps/.env}"

# Pull values from the VPS .env when present (matches wowctl's load_env).
if [[ -f "$VPS_ENV" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$VPS_ENV"
  set +a
fi

# --- Resolve AzerothCore checkout dir ----------------------------------------
if [[ -z "${ACORE_REPO_DIR:-}" && -n "${WOW_ACORE_DIR:-}" ]]; then
  ACORE_REPO_DIR="$WOW_ACORE_DIR"
fi
if [[ -z "${ACORE_REPO_DIR:-}" && -d /opt/azerothcore-wotlk ]]; then
  ACORE_REPO_DIR="/opt/azerothcore-wotlk"
fi
ACORE_REPO_DIR="${ACORE_REPO_DIR:-/tmp/azerothcore-wotlk}"
export ACORE_REPO_DIR

# --- Resolve MySQL root password ---------------------------------------------
MYSQL_ROOT_PASSWORD="${MYSQL_ROOT_PASSWORD:-${WOW_MYSQL_PASSWORD:-password}}"
export MYSQL_ROOT_PASSWORD
