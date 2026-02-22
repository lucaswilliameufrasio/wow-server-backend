#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

NAMESPACE="${NAMESPACE:-wow-backend}"
CNPG_CLUSTER="${CNPG_CLUSTER:-wow-app-pg}"
PG_DATABASE="${PG_DATABASE:-wow_app}"
RUN_PG_BACKUP="${RUN_PG_BACKUP:-true}"
RUN_AZEROTHCORE_BACKUP="${RUN_AZEROTHCORE_BACKUP:-true}"

ACORE_REPO_DIR="${ACORE_REPO_DIR:-/tmp/azerothcore-wotlk}"
ACORE_DBS="${ACORE_DBS:-acore_auth acore_characters acore_world wotlk_items}"

BACKUP_ROOT="${BACKUP_ROOT:-$ROOT_DIR/backups}"
STAMP="$(date +%F_%H%M%S)"
OUT_DIR="${OUT_DIR:-$BACKUP_ROOT/$STAMP}"

log() {
  echo "[backup-before-update] $*"
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || { echo "Missing command: $1" >&2; exit 1; }
}

configure_kube_env() {
  if [ -f "$HOME/.kube/config" ]; then
    export KUBECONFIG="$HOME/.kube/config"
  elif [ -f /etc/rancher/k3s/k3s.yaml ]; then
    export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
  fi
}

backup_pg() {
  need_cmd kubectl
  configure_kube_env

  local pod
  pod="$(kubectl get pod -n "$NAMESPACE" -l "cnpg.io/cluster=${CNPG_CLUSTER},role=primary" -o jsonpath='{.items[0].metadata.name}')"
  if [ -z "$pod" ]; then
    echo "Could not find CNPG primary pod for cluster $CNPG_CLUSTER in namespace $NAMESPACE" >&2
    exit 1
  fi

  mkdir -p "$OUT_DIR/postgres"
  log "Backing up Postgres database $PG_DATABASE from pod $pod"
  kubectl exec -n "$NAMESPACE" "$pod" -- sh -lc "pg_dump -U postgres '$PG_DATABASE'" > "$OUT_DIR/postgres/${PG_DATABASE}.sql"
}

backup_azerothcore_mysql() {
  need_cmd docker
  if [ ! -f "$ACORE_REPO_DIR/docker-compose.yml" ]; then
    echo "AzerothCore docker-compose.yml not found at $ACORE_REPO_DIR" >&2
    exit 1
  fi

  mkdir -p "$OUT_DIR/mysql"
  for db in $ACORE_DBS; do
    log "Backing up MySQL database $db"
    docker compose -f "$ACORE_REPO_DIR/docker-compose.yml" exec -T ac-database \
      sh -lc "mysqldump -uroot -ppassword '$db'" > "$OUT_DIR/mysql/${db}.sql"
  done
}

main() {
  mkdir -p "$OUT_DIR"
  [ "$RUN_PG_BACKUP" = "true" ] && backup_pg
  [ "$RUN_AZEROTHCORE_BACKUP" = "true" ] && backup_azerothcore_mysql
  log "Backups created in: $OUT_DIR"
}

main "$@"
