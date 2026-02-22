#!/usr/bin/env bash
set -euo pipefail

# Weekend bootstrap for a fresh VPS:
# - installs k3s (optional)
# - installs CloudNativePG operator (optional)
# - builds/imports backend image into k3s containerd
# - creates required secrets
# - applies k8s manifests, runs migration job, and deploys backend

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
K8S_DIR="$ROOT_DIR/k8s"

NAMESPACE="${NAMESPACE:-wow-backend}"
INSTALL_K3S="${INSTALL_K3S:-true}"
INSTALL_CNPG="${INSTALL_CNPG:-true}"
BUILD_IMAGE="${BUILD_IMAGE:-true}"
IMPORT_IMAGE="${IMPORT_IMAGE:-true}"
APPLY_MANIFESTS="${APPLY_MANIFESTS:-true}"
WAIT_TIMEOUT="${WAIT_TIMEOUT:-600s}"

BACKEND_IMAGE="${BACKEND_IMAGE:-wow-server-backend:dev}"
CNPG_VERSION="${CNPG_VERSION:-1.24.2}"
CNPG_MANIFEST_URL="${CNPG_MANIFEST_URL:-https://raw.githubusercontent.com/cloudnative-pg/cloudnative-pg/release-1.24/releases/cnpg-${CNPG_VERSION}.yaml}"

AZEROTH_CORE_MYSQL_DATABASE_URL="${AZEROTH_CORE_MYSQL_DATABASE_URL:-}"
JWT_PRIVATE_KEY_FILE="${JWT_PRIVATE_KEY_FILE:-$ROOT_DIR/.secrets/jwt_private.pem}"
JWT_PUBLIC_KEY_FILE="${JWT_PUBLIC_KEY_FILE:-$ROOT_DIR/.secrets/jwt_public.pem}"
PG_APP_USER_PASSWORD="${PG_APP_USER_PASSWORD:-}"
PG_SUPERUSER_PASSWORD="${PG_SUPERUSER_PASSWORD:-}"

log() {
  echo "[weekend-bootstrap] $*"
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

ensure_random_passwords() {
  if [ -z "$PG_APP_USER_PASSWORD" ]; then
    PG_APP_USER_PASSWORD="$(openssl rand -base64 32 | tr -d '\n')"
    log "Generated PG_APP_USER_PASSWORD"
  fi
  if [ -z "$PG_SUPERUSER_PASSWORD" ]; then
    PG_SUPERUSER_PASSWORD="$(openssl rand -base64 32 | tr -d '\n')"
    log "Generated PG_SUPERUSER_PASSWORD"
  fi
}

install_k3s() {
  if command -v kubectl >/dev/null 2>&1 && command -v k3s >/dev/null 2>&1; then
    log "k3s/kubectl already installed, skipping"
    return
  fi

  need_cmd curl
  log "Installing k3s"
  run_sudo sh -c 'curl -sfL https://get.k3s.io | sh -'

  if [ ! -f /etc/rancher/k3s/k3s.yaml ]; then
    echo "k3s kubeconfig not found at /etc/rancher/k3s/k3s.yaml" >&2
    exit 1
  fi

  mkdir -p "$HOME/.kube"
  run_sudo cp /etc/rancher/k3s/k3s.yaml "$HOME/.kube/config"
  run_sudo chown "$(id -u):$(id -g)" "$HOME/.kube/config"
  chmod 600 "$HOME/.kube/config"

  if ! command -v kubectl >/dev/null 2>&1; then
    run_sudo ln -sf /usr/local/bin/k3s /usr/local/bin/kubectl
  fi
}

configure_kube_env() {
  if [ -f "$HOME/.kube/config" ]; then
    export KUBECONFIG="$HOME/.kube/config"
  elif [ -f /etc/rancher/k3s/k3s.yaml ]; then
    export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
  fi
}

install_cnpg_operator() {
  need_cmd kubectl
  configure_kube_env

  if kubectl get deployment -n cnpg-system cnpg-controller-manager >/dev/null 2>&1; then
    log "CloudNativePG operator already installed, skipping"
  else
    log "Installing CloudNativePG operator from $CNPG_MANIFEST_URL"
    kubectl apply -f "$CNPG_MANIFEST_URL"
  fi

  log "Waiting for CloudNativePG operator"
  kubectl wait --for=condition=Available deployment/cnpg-controller-manager \
    -n cnpg-system --timeout="$WAIT_TIMEOUT"
}

build_and_import_image() {
  need_cmd docker
  if [ "$BUILD_IMAGE" = "true" ]; then
    log "Building backend image: $BACKEND_IMAGE"
    docker build -t "$BACKEND_IMAGE" "$ROOT_DIR"
  fi

  if [ "$IMPORT_IMAGE" = "true" ]; then
    if ! command -v k3s >/dev/null 2>&1; then
      echo "k3s CLI not found; cannot import image to k3s containerd" >&2
      exit 1
    fi
    log "Importing image into k3s containerd: $BACKEND_IMAGE"
    docker save "$BACKEND_IMAGE" | run_sudo k3s ctr images import -
  fi
}

create_secrets() {
  need_cmd kubectl
  configure_kube_env

  if [ -z "$AZEROTH_CORE_MYSQL_DATABASE_URL" ]; then
    echo "AZEROTH_CORE_MYSQL_DATABASE_URL is required" >&2
    exit 1
  fi
  if [ ! -f "$JWT_PRIVATE_KEY_FILE" ]; then
    echo "JWT_PRIVATE_KEY_FILE not found: $JWT_PRIVATE_KEY_FILE" >&2
    exit 1
  fi
  if [ ! -f "$JWT_PUBLIC_KEY_FILE" ]; then
    echo "JWT_PUBLIC_KEY_FILE not found: $JWT_PUBLIC_KEY_FILE" >&2
    exit 1
  fi

  ensure_random_passwords

  kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -

  log "Applying wow-backend-secrets"
  kubectl create secret generic wow-backend-secrets \
    -n "$NAMESPACE" \
    --from-literal=AZEROTH_CORE_MYSQL_DATABASE_URL="$AZEROTH_CORE_MYSQL_DATABASE_URL" \
    --from-file=jwt_private.pem="$JWT_PRIVATE_KEY_FILE" \
    --from-file=jwt_public.pem="$JWT_PUBLIC_KEY_FILE" \
    --dry-run=client -o yaml | kubectl apply -f -

  log "Applying wow-app-pg-app-user"
  kubectl create secret generic wow-app-pg-app-user \
    -n "$NAMESPACE" \
    --type=kubernetes.io/basic-auth \
    --from-literal=username=wow_app_user \
    --from-literal=password="$PG_APP_USER_PASSWORD" \
    --dry-run=client -o yaml | kubectl apply -f -

  log "Applying wow-app-pg-superuser"
  kubectl create secret generic wow-app-pg-superuser \
    -n "$NAMESPACE" \
    --type=kubernetes.io/basic-auth \
    --from-literal=username=postgres \
    --from-literal=password="$PG_SUPERUSER_PASSWORD" \
    --dry-run=client -o yaml | kubectl apply -f -
}

apply_manifests() {
  need_cmd kubectl
  configure_kube_env

  log "Applying base namespace/config"
  kubectl apply -f "$K8S_DIR/namespace.yaml"
  kubectl apply -f "$K8S_DIR/configmap.yaml"
  kubectl apply -f "$K8S_DIR/migrations-configmap.yaml"

  log "Applying CloudNativePG cluster"
  kubectl apply -f "$K8S_DIR/cloudnativepg-cluster.yaml"

  log "Waiting for CNPG app secret (wow-app-pg-app)"
  for _ in $(seq 1 120); do
    if kubectl get secret wow-app-pg-app -n "$NAMESPACE" >/dev/null 2>&1; then
      break
    fi
    sleep 2
  done

  log "Running migration job"
  kubectl delete job wow-backend-migrate -n "$NAMESPACE" --ignore-not-found
  kubectl apply -f "$K8S_DIR/migration-job.yaml"
  kubectl wait --for=condition=complete job/wow-backend-migrate -n "$NAMESPACE" --timeout="$WAIT_TIMEOUT"

  log "Applying app deployment/service/cronjob"
  kubectl apply -k "$K8S_DIR"

  log "Setting backend image to $BACKEND_IMAGE"
  kubectl set image deployment/wow-backend wow-backend="$BACKEND_IMAGE" -n "$NAMESPACE"

  log "Waiting for backend rollout"
  kubectl rollout status deployment/wow-backend -n "$NAMESPACE" --timeout="$WAIT_TIMEOUT"
}

print_summary() {
  cat <<SUMMARY

Weekend server bootstrap complete.

Namespace: $NAMESPACE
Backend image: $BACKEND_IMAGE

If passwords were auto-generated, capture them now from environment/session or rerun with explicit values.

Useful checks:
  kubectl get pods -n $NAMESPACE
  kubectl get svc -n $NAMESPACE
  kubectl logs job/wow-backend-migrate -n $NAMESPACE

SUMMARY
}

main() {
  need_cmd openssl

  if [ "$INSTALL_K3S" = "true" ]; then
    install_k3s
  fi

  configure_kube_env
  need_cmd kubectl

  if [ "$INSTALL_CNPG" = "true" ]; then
    install_cnpg_operator
  fi

  build_and_import_image
  create_secrets

  if [ "$APPLY_MANIFESTS" = "true" ]; then
    apply_manifests
  fi

  print_summary
}

main "$@"
