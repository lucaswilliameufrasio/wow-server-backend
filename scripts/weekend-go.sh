#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NOTIFY_SCRIPT="$ROOT_DIR/scripts/notify-webhooks.sh"

SKIP_PREFLIGHT="${SKIP_PREFLIGHT:-false}"
SKIP_BACKUP="${SKIP_BACKUP:-false}"
SKIP_UPDATE="${SKIP_UPDATE:-false}"
SKIP_VENDOR_SEED="${SKIP_VENDOR_SEED:-false}"

notify() {
  local status="$1"
  local message="$2"
  if [ -x "$NOTIFY_SCRIPT" ]; then
    "$NOTIFY_SCRIPT" --source "weekend-go" --status "$status" --message "$message" || true
  fi
}

smoke_checks() {
  local ns="${NAMESPACE:-wow-backend}"
  kubectl get pods -n "$ns"
  kubectl rollout status deployment/wow-backend -n "$ns" --timeout="${WAIT_TIMEOUT:-600s}"
  kubectl logs deployment/wow-backend -n "$ns" --tail=120 || true
}

main() {
  trap 'code=$?; if [ "$code" -eq 0 ]; then notify success "weekend-go completed"; else notify failure "weekend-go failed (exit=$code)"; fi' EXIT

  if [ "$SKIP_PREFLIGHT" != "true" ]; then
    "$ROOT_DIR/scripts/preflight-update.sh"
  fi

  if [ "$SKIP_BACKUP" != "true" ]; then
    "$ROOT_DIR/scripts/backup-before-update.sh"
  fi

  if [ "$SKIP_UPDATE" != "true" ]; then
    "$ROOT_DIR/scripts/update-existing-server.sh"
  fi

  if [ "$SKIP_VENDOR_SEED" != "true" ]; then
    "$ROOT_DIR/scripts/seed-fast-raid-vendors.sh"
  fi

  smoke_checks
}

main "$@"
