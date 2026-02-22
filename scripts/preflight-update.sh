#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
K8S_DIR="$ROOT_DIR/k8s"

NAMESPACE="${NAMESPACE:-wow-backend}"

UPDATE_AZEROTHCORE="${UPDATE_AZEROTHCORE:-true}"
UPDATE_BACKEND="${UPDATE_BACKEND:-true}"
RUN_ITEM_SEED="${RUN_ITEM_SEED:-false}"

ACORE_REPO_DIR="${ACORE_REPO_DIR:-/tmp/azerothcore-wotlk}"
BACKEND_IMAGE="${BACKEND_IMAGE:-wow-server-backend:dev}"
BUILD_IMAGE="${BUILD_IMAGE:-true}"
IMPORT_IMAGE="${IMPORT_IMAGE:-true}"
APPLY_BACKEND_MIGRATIONS="${APPLY_BACKEND_MIGRATIONS:-true}"

log() {
  echo "[preflight-update] $*"
}

fail() {
  echo "[preflight-update] ERROR: $*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing command: $1"
}

configure_kube_env() {
  if [ -f "$HOME/.kube/config" ]; then
    export KUBECONFIG="$HOME/.kube/config"
  elif [ -f /etc/rancher/k3s/k3s.yaml ]; then
    export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
  fi
}

check_azerothcore() {
  log "Checking AzerothCore update prerequisites"
  need_cmd git
  need_cmd docker

  if [ -d "$ACORE_REPO_DIR/.git" ]; then
    [ -f "$ACORE_REPO_DIR/docker-compose.yml" ] || fail "missing $ACORE_REPO_DIR/docker-compose.yml"
  else
    log "AzerothCore repo not found at $ACORE_REPO_DIR (it will be cloned during update)"
  fi

  if [ "$RUN_ITEM_SEED" = "true" ]; then
    [ -x "$ROOT_DIR/scripts/seed-items.sh" ] || fail "seed script not executable: $ROOT_DIR/scripts/seed-items.sh"
  fi
}

check_backend() {
  log "Checking backend update prerequisites"
  need_cmd kubectl
  need_cmd docker
  configure_kube_env

  kubectl version --client >/dev/null 2>&1 || fail "kubectl client is not working"
  kubectl cluster-info >/dev/null 2>&1 || fail "cannot reach kubernetes cluster"

  kubectl get namespace "$NAMESPACE" >/dev/null 2>&1 || \
    log "namespace $NAMESPACE does not exist yet (will fail update until created)"

  [ -d "$K8S_DIR" ] || fail "missing k8s dir: $K8S_DIR"
  [ -f "$K8S_DIR/migrations-configmap.yaml" ] || fail "missing $K8S_DIR/migrations-configmap.yaml"
  [ -f "$K8S_DIR/migration-job.yaml" ] || fail "missing $K8S_DIR/migration-job.yaml"
  [ -f "$K8S_DIR/kustomization.yaml" ] || fail "missing $K8S_DIR/kustomization.yaml"

  if [ "$APPLY_BACKEND_MIGRATIONS" = "true" ]; then
    [ -d "$ROOT_DIR/migrations" ] || fail "missing migrations dir: $ROOT_DIR/migrations"
    find "$ROOT_DIR/migrations" -maxdepth 1 -type f -name "*.sql" | grep -q . || \
      fail "no .sql files found in $ROOT_DIR/migrations"
  fi

  if [ "$BUILD_IMAGE" = "true" ]; then
    log "backend image will be built: $BACKEND_IMAGE"
  fi
  if [ "$IMPORT_IMAGE" = "true" ]; then
    need_cmd k3s
  fi
}

main() {
  [ "$UPDATE_AZEROTHCORE" = "true" ] || [ "$UPDATE_BACKEND" = "true" ] || \
    fail "both UPDATE_AZEROTHCORE and UPDATE_BACKEND are false; nothing to validate"

  [ "$UPDATE_AZEROTHCORE" = "true" ] && check_azerothcore
  [ "$UPDATE_BACKEND" = "true" ] && check_backend

  log "Preflight checks passed"
}

main "$@"
