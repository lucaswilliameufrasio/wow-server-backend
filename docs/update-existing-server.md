# Update existing server (AzerothCore + backend)

Use this when your server is already running and you want to apply:
- latest AzerothCore upstream changes/migrations
- latest backend image + backend SQL migrations

## Quick start (full update)

Run preflight first:

```bash
make preflight-update
```

Then run update:

```bash
make update-server
```

Equivalent script call:

```bash
./scripts/update-existing-server.sh
```

Note: `make update-server`, `make update-azerothcore`, and `make update-backend` already run preflight automatically.

## What it does

1. Updates AzerothCore git repo (`ACORE_REPO_DIR`) to `ACORE_REF`
2. Runs `docker compose pull && docker compose up -d --build`
3. Waits for MySQL readiness and restarts `ac-worldserver` (applies pending DB updates)
4. Builds backend image and imports it into k3s containerd
5. Re-applies backend migration ConfigMap and runs migration Job
6. Re-applies k8s manifests and rolls out backend Deployment

## Common modes

Only AzerothCore update:

```bash
make update-azerothcore
```

Only backend update:

```bash
make update-backend
```

Refresh item DB during update:

```bash
RUN_ITEM_SEED=true make update-azerothcore
```

## Useful env overrides

```bash
export ACORE_REPO_DIR=/opt/azerothcore-wotlk
export ACORE_REF=master
export BACKEND_IMAGE=wow-server-backend:dev
export NAMESPACE=wow-backend
export WAIT_TIMEOUT=900s
```

Skip backend image build/import (if image already present in k3s):

```bash
BUILD_IMAGE=false IMPORT_IMAGE=false make update-backend
```

## Post-update verification

```bash
kubectl get pods -n wow-backend
kubectl logs deployment/wow-backend -n wow-backend --tail=200
kubectl logs job/wow-backend-migrate -n wow-backend --tail=200
```

Check AzerothCore services:

```bash
docker compose -f /tmp/azerothcore-wotlk/docker-compose.yml ps
docker compose -f /tmp/azerothcore-wotlk/docker-compose.yml logs --tail=200 ac-worldserver
```

## Failure handling

- If backend migration job fails: inspect logs, fix SQL, re-run `make update-backend`.
- If backend rollout fails: inspect deployment logs/events and run `kubectl rollout undo deployment/wow-backend -n wow-backend` if needed.
- If AzerothCore fails after update: check `ac-worldserver` and `ac-authserver` logs and re-run `make update-azerothcore` after resolving compose/env issues.
