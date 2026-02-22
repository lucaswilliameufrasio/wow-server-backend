#!/usr/bin/env bash
set -euo pipefail

STATUS="info"
SOURCE_NAME="ops"
MESSAGE=""

while [ $# -gt 0 ]; do
  case "$1" in
    --status)
      STATUS="${2:-info}"
      shift 2
      ;;
    --source)
      SOURCE_NAME="${2:-ops}"
      shift 2
      ;;
    --message)
      MESSAGE="${2:-}"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done

NOTIFY_ENABLED="${NOTIFY_ENABLED:-false}"
DISCORD_WEBHOOK_URL="${DISCORD_WEBHOOK_URL:-}"
TELEGRAM_BOT_TOKEN="${TELEGRAM_BOT_TOKEN:-}"
TELEGRAM_CHAT_ID="${TELEGRAM_CHAT_ID:-}"

if [ "$NOTIFY_ENABLED" != "true" ]; then
  exit 0
fi

if ! command -v curl >/dev/null 2>&1; then
  exit 0
fi

TEXT="[$SOURCE_NAME][$STATUS] $MESSAGE"

if [ -n "$DISCORD_WEBHOOK_URL" ]; then
  ESCAPED="$(printf '%s' "$TEXT" | sed 's/\\/\\\\/g; s/"/\\"/g')"
  curl -fsS -X POST "$DISCORD_WEBHOOK_URL" \
    -H "Content-Type: application/json" \
    -d "{\"content\":\"$ESCAPED\"}" >/dev/null || true
fi

if [ -n "$TELEGRAM_BOT_TOKEN" ] && [ -n "$TELEGRAM_CHAT_ID" ]; then
  curl -fsS -X POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
    --data-urlencode "chat_id=${TELEGRAM_CHAT_ID}" \
    --data-urlencode "text=${TEXT}" >/dev/null || true
fi
