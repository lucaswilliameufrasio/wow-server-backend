# Deployment and maintenance runbook

This runbook covers production-like deployment and day-2 maintenance for:
- k3s cluster
- CloudNativePG PostgreSQL 18
- wow-backend Kubernetes manifests in `k8s/`

## Scope

- Backend API deployment
- Postgres cluster operations
- DB migrations and token cleanup jobs
- Backups, restore, upgrades, rollback, and troubleshooting

## 1) Initial deployment

### 1.1 Prerequisites

- k3s installed and reachable via `kubectl`
- CloudNativePG operator installed
- Backend image available to cluster nodes
  - local image import for single-node k3s, or
  - internal registry for multi-node
- AzerothCore MySQL reachable from cluster
- JWT key pair generated

### 1.2 One-command weekend bootstrap (recommended)

Use:

```bash
./scripts/provision-weekend-server.sh
```

Reference env-driven setup:
- `docs/weekend-quickstart.md`

### 1.3 Manual apply order

```bash
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/secrets.example.yaml
kubectl apply -f k8s/migrations-configmap.yaml
kubectl apply -f k8s/cloudnativepg-cluster.yaml
kubectl apply -f k8s/migration-job.yaml
kubectl wait --for=condition=complete job/wow-backend-migrate -n wow-backend --timeout=300s
kubectl apply -k k8s
```

## 2) Daily operations

### 2.1 Health checks

```bash
kubectl get pods -n wow-backend
kubectl get svc -n wow-backend
kubectl logs deployment/wow-backend -n wow-backend --tail=200
kubectl logs job/wow-backend-migrate -n wow-backend --tail=200
```

API health from in-cluster:

```bash
kubectl run curl --rm -it --restart=Never --image=curlimages/curl:8.10.1 -n wow-backend -- \
  curl -sS http://wow-backend/health-check
```

### 2.2 Verify scheduled cleanup

```bash
kubectl get cronjob wow-backend-token-cleanup -n wow-backend
kubectl get jobs -n wow-backend --sort-by=.metadata.creationTimestamp | tail
```

## 3) Deploying a new backend version

### 3.1 Single-node local-image flow

```bash
docker build -t wow-server-backend:dev .
docker save wow-server-backend:dev | sudo k3s ctr images import -
kubectl rollout restart deployment/wow-backend -n wow-backend
kubectl rollout status deployment/wow-backend -n wow-backend --timeout=600s
```

### 3.2 Multi-node recommendation

- Push image to internal/private registry.
- Update `k8s/backend-deployment.yaml` image tag.
- Apply and monitor rollout:

```bash
kubectl apply -f k8s/backend-deployment.yaml
kubectl rollout status deployment/wow-backend -n wow-backend --timeout=600s
```

## 4) Database migrations

### 4.1 Run migrations

```bash
kubectl delete job wow-backend-migrate -n wow-backend --ignore-not-found
kubectl apply -f k8s/migration-job.yaml
kubectl wait --for=condition=complete job/wow-backend-migrate -n wow-backend --timeout=600s
kubectl logs job/wow-backend-migrate -n wow-backend
```

### 4.2 Update migration SQL in cluster

If migration files changed in repo, re-apply configmap before migration job:

```bash
kubectl apply -f k8s/migrations-configmap.yaml
```

## 5) Backup and restore (CloudNativePG)

Important: keep this aligned with your retention/compliance policy.

### 5.1 Backup strategy

Use CloudNativePG backup features (object store recommended).

At minimum, define:
- backup destination (S3-compatible bucket)
- schedule
- retention policy

### 5.2 Manual logical backup (emergency fallback)

```bash
POD=$(kubectl get pod -n wow-backend -l cnpg.io/cluster=wow-app-pg,role=primary -o jsonpath='{.items[0].metadata.name}')
kubectl exec -n wow-backend "$POD" -- pg_dump -U wow_app_user -d wow_app > wow_app_$(date +%F_%H%M).sql
```

### 5.3 Restore note

Prefer native CloudNativePG recovery workflows for point-in-time or base backups.
For logical dump restore, create a temporary recovery DB and import with `psql`.

## 6) Upgrade procedures

### 6.1 Backend upgrade

1. Build/import new image
2. Restart deployment or update image
3. Watch rollout and logs
4. Run smoke tests

Repeatable command for existing servers:

```bash
make preflight-update
make update-backend
```

### 6.2 CloudNativePG/operator upgrade

1. Read CNPG release notes
2. Upgrade operator
3. Confirm cluster status
4. Validate app connectivity and migration/job health

### 6.3 AzerothCore upstream upgrade

Run:

```bash
make preflight-update
make update-azerothcore
```

This updates AzerothCore repo + compose services and restarts `ac-worldserver` to apply pending DB updates.
For full-stack update flow, use:

```bash
make update-server
```

Detailed guide:
- `docs/update-existing-server.md`
- `docs/weekend-go.md`

## 7) Rollback procedures

### 7.1 Backend rollback

If using Deployment rollout history:

```bash
kubectl rollout undo deployment/wow-backend -n wow-backend
kubectl rollout status deployment/wow-backend -n wow-backend --timeout=600s
```

Or set previous known-good image explicitly.

### 7.2 Migration rollback

- Prefer forward fixes for production incidents.
- If required, apply controlled rollback SQL from migration down scripts.
- Validate data impact before rollback.

## 8) Security and secrets

- Never commit real secrets to git.
- Rotate regularly:
  - JWT keys
  - Postgres credentials
  - MySQL credentials
- Use external secret manager in production.
- Scope RBAC/service accounts to least privilege.

## 9) Observability

Minimum recommended:
- pod restart alerts
- error-rate and latency alerts
- Postgres availability and storage alerts
- migration job failure alerts

## 10) Incident quick triage

```bash
kubectl get events -n wow-backend --sort-by=.lastTimestamp | tail -n 50
kubectl describe pod -n wow-backend -l app=wow-backend
kubectl logs deployment/wow-backend -n wow-backend --tail=500
kubectl get cluster -n wow-backend wow-app-pg -o yaml
kubectl describe job wow-backend-migrate -n wow-backend
```

## 11) Operational checklist

Before game weekend:
- Secrets validated
- MySQL connectivity verified
- CNPG cluster healthy
- Migration job completed
- Backend rollout healthy
- Token cleanup cronjob running
- Smoke test (`/health-check`, sign-in, character load) passes

One-command execution:

```bash
make weekend-go
```
