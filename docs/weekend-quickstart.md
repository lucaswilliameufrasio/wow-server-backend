# Weekend VPS quickstart — k3s + CloudNativePG

> ⚠️ **Atenção:** este guia usa Kubernetes (k3s + CloudNativePG), que é mais complexo.
> Para uma stack mais simples e rápida, use o guia **[Docker weekend VPS](docker-weekend-vps.md)**.

This guide is for spinning up the full stack on a new VPS with k3s + CloudNativePG + local container image (no external registry).

## 0) Requirements

- Fresh Linux VPS with sudo access
- Docker installed
- Repo cloned on the VPS
- JWT keys available (or generated locally)
- Reachable AzerothCore MySQL endpoint

## 1) Prepare environment

Create an env file (example below):

```bash
cat > .env.weekend <<'ENV'
AZEROTH_CORE_MYSQL_DATABASE_URL=mysql://root:password@YOUR_AZEROTHCORE_MYSQL_HOST:3306
JWT_PRIVATE_KEY_FILE=/opt/wow-server-backend/.secrets/jwt_private.pem
JWT_PUBLIC_KEY_FILE=/opt/wow-server-backend/.secrets/jwt_public.pem

# Optional (if omitted, random passwords are generated)
PG_APP_USER_PASSWORD=change-me-app-user
PG_SUPERUSER_PASSWORD=change-me-superuser

# Optional behavior flags
INSTALL_K3S=true
INSTALL_CNPG=true
BUILD_IMAGE=true
IMPORT_IMAGE=true
APPLY_MANIFESTS=true
BACKEND_IMAGE=wow-server-backend:dev
WAIT_TIMEOUT=600s
ENV
```

## 2) Generate JWT keys (if needed)

```bash
mkdir -p .secrets
openssl genrsa -out .secrets/jwt_private.pem 2048
openssl rsa -in .secrets/jwt_private.pem -pubout -out .secrets/jwt_public.pem
```

## 3) Run one-command bootstrap

```bash
set -a
source .env.weekend
set +a
./scripts/provision-weekend-server.sh
```

What this does:
- Installs k3s (optional)
- Installs CloudNativePG operator (optional)
- Builds backend image and imports it to k3s containerd
- Creates required secrets
- Applies PostgreSQL cluster and waits
- Runs migration job and waits
- Deploys backend and waits for rollout

## 4) Verify

```bash
kubectl get pods -n wow-backend
kubectl get svc -n wow-backend
kubectl logs job/wow-backend-migrate -n wow-backend
```

Health check from inside cluster:

```bash
kubectl run curl --rm -it --restart=Never --image=curlimages/curl:8.10.1 -n wow-backend -- \
  curl -sS http://wow-backend/health-check
```

## 5) Re-deploy updated backend code

```bash
docker build -t wow-server-backend:dev .
docker save wow-server-backend:dev | sudo k3s ctr images import -
kubectl rollout restart deployment/wow-backend -n wow-backend
kubectl rollout status deployment/wow-backend -n wow-backend --timeout=600s
```

## Notes

- For single-node k3s this local-image workflow is fastest.
- For multi-node k3s, switch to an internal registry mirror.
- The migration job is idempotent and can be re-run safely.
- For recurring updates on an existing server, use `make update-server` (see `docs/update-existing-server.md`).
