#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
K8S_DIR="$ROOT_DIR/k8s"

NAMESPACE="${NAMESPACE:-wow-backend}"
WAIT_TIMEOUT="${WAIT_TIMEOUT:-900s}"

UPDATE_AZEROTHCORE="${UPDATE_AZEROTHCORE:-true}"
UPDATE_BACKEND="${UPDATE_BACKEND:-true}"
RUN_ITEM_SEED="${RUN_ITEM_SEED:-false}"

ACORE_REPO_DIR="${ACORE_REPO_DIR:-/tmp/azerothcore-wotlk}"
ACORE_REPO_URL="${ACORE_REPO_URL:-https://github.com/azerothcore/azerothcore-wotlk.git}"
ACORE_REF="${ACORE_REF:-master}"

BACKEND_IMAGE="${BACKEND_IMAGE:-wow-server-backend:dev}"
BUILD_IMAGE="${BUILD_IMAGE:-true}"
IMPORT_IMAGE="${IMPORT_IMAGE:-true}"
APPLY_BACKEND_MIGRATIONS="${APPLY_BACKEND_MIGRATIONS:-true}"
REAPPLY_K8S="${REAPPLY_K8S:-true}"

log() {
  echo "[update-existing-server] $*"
}

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

run_sudo() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  else
    sudo "$@"
  fi
}

configure_kube_env() {
  if [ -f "$HOME/.kube/config" ]; then
    export KUBECONFIG="$HOME/.kube/config"
  elif [ -f /etc/rancher/k3s/k3s.yaml ]; then
    export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
  fi
}

ensure_azerothcore_repo() {
  if [ ! -d "$ACORE_REPO_DIR/.git" ]; then
    log "Cloning AzerothCore into $ACORE_REPO_DIR"
    git clone --depth 1 --branch "$ACORE_REF" "$ACORE_REPO_URL" "$ACORE_REPO_DIR"
  else
    log "Updating AzerothCore repo in $ACORE_REPO_DIR ($ACORE_REF)"
    git -C "$ACORE_REPO_DIR" fetch --depth 1 origin "$ACORE_REF"
    git -C "$ACORE_REPO_DIR" checkout "$ACORE_REF"
    git -C "$ACORE_REPO_DIR" pull --ff-only origin "$ACORE_REF"
  fi
}

wait_acore_mysql() {
  log "Waiting for AzerothCore MySQL in ac-database"
  for _ in $(seq 1 120); do
    if docker compose -f "$ACORE_REPO_DIR/docker-compose.yml" exec -T ac-database \
      mysqladmin ping -h127.0.0.1 -uroot -ppassword >/dev/null 2>&1; then
      log "AzerothCore MySQL is ready"
      return
    fi
    sleep 5
  done
  echo "AzerothCore MySQL did not become ready in time" >&2
  exit 1
}

update_azerothcore() {
  need_cmd git
  need_cmd docker

  ensure_azerothcore_repo

  log "Pulling latest AzerothCore images and applying compose update"
  docker compose -f "$ACORE_REPO_DIR/docker-compose.yml" pull
  docker compose -f "$ACORE_REPO_DIR/docker-compose.yml" up -d --build

  wait_acore_mysql

  # Ensure worldserver restarts against latest schema/assets and applies pending DB updates.
  if docker compose -f "$ACORE_REPO_DIR/docker-compose.yml" ps ac-worldserver >/dev/null 2>&1; then
    log "Restarting ac-worldserver to apply pending DB updates if any"
    docker compose -f "$ACORE_REPO_DIR/docker-compose.yml" restart ac-worldserver
  fi

  if [ "$RUN_ITEM_SEED" = "true" ]; then
    log "Refreshing item seed database"
    "$ROOT_DIR/scripts/seed-items.sh"
  fi
}

build_and_import_backend_image() {
  need_cmd docker

  if [ "$BUILD_IMAGE" = "true" ]; then
    log "Building backend image: $BACKEND_IMAGE"
    docker build -t "$BACKEND_IMAGE" "$ROOT_DIR"
  fi

  if [ "$IMPORT_IMAGE" = "true" ]; then
    need_cmd k3s
    log "Importing backend image into k3s containerd: $BACKEND_IMAGE"
    docker save "$BACKEND_IMAGE" | run_sudo k3s ctr images import -
  fi
}

run_backend_migrations() {
  log "Applying migrations ConfigMap"
  kubectl apply -f "$K8S_DIR/migrations-configmap.yaml"

  if [ "$APPLY_BACKEND_MIGRATIONS" = "true" ]; then
    log "Running backend migration Job"
    kubectl delete job wow-backend-migrate -n "$NAMESPACE" --ignore-not-found
    kubectl apply -f "$K8S_DIR/migration-job.yaml"
    kubectl wait --for=condition=complete job/wow-backend-migrate -n "$NAMESPACE" --timeout="$WAIT_TIMEOUT"
  fi
}

rollout_backend() {
  if [ "$REAPPLY_K8S" = "true" ]; then
    log "Reapplying k8s manifests"
    kubectl apply -k "$K8S_DIR"
  fi

  log "Updating backend deployment image to $BACKEND_IMAGE"
  kubectl set image deployment/wow-backend wow-backend="$BACKEND_IMAGE" -n "$NAMESPACE"
  kubectl rollout status deployment/wow-backend -n "$NAMESPACE" --timeout="$WAIT_TIMEOUT"
}

update_backend() {
  need_cmd kubectl
  configure_kube_env

  build_and_import_backend_image
  run_backend_migrations
  rollout_backend
}

main() {
  if [ "$UPDATE_AZEROTHCORE" = "true" ]; then
    update_azerothcore
  fi

  if [ "$UPDATE_BACKEND" = "true" ]; then
    update_backend
  fi

  log "Update flow completed"
  log "Quick checks:"
  log "  kubectl get pods -n $NAMESPACE"
  log "  kubectl logs deployment/wow-backend -n $NAMESPACE --tail=200"
}

main "$@"
