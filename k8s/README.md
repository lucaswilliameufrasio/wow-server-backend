# Kubernetes manifests (k3s + CloudNativePG)

This folder contains a baseline deployment for:
- `wow-backend` API
- CloudNativePG PostgreSQL 18 cluster (`wow-app-pg`)

## Prerequisites

1. k3s cluster
2. CloudNativePG operator installed
3. Build backend image locally and import into k3s containerd (no external registry required)

## Local image workflow (recommended for dev)

The Deployment uses:
- `image: wow-server-backend:dev`
- `imagePullPolicy: Never`

Build and import:

```bash
docker build -t wow-server-backend:dev .
docker save wow-server-backend:dev | sudo k3s ctr images import -
```

If you rebuild, re-import and restart:

```bash
docker build -t wow-server-backend:dev .
docker save wow-server-backend:dev | sudo k3s ctr images import -
kubectl rollout restart deployment/wow-backend -n wow-backend
```

Verify image exists in k3s:

```bash
sudo k3s ctr images ls | grep wow-server-backend
```

## Apply order

```bash
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/secrets.example.yaml
kubectl apply -f k8s/migrations-configmap.yaml
kubectl apply -f k8s/cloudnativepg-cluster.yaml
kubectl apply -f k8s/migration-job.yaml
kubectl wait --for=condition=complete job/wow-backend-migrate -n wow-backend --timeout=300s
kubectl apply -k k8s
```

## Notes

- `APP_POSTGRES_DATABASE_URL` is read from CloudNativePG generated secret `wow-app-pg-app` key `uri`.
- Migration Job uses `sqlx-cli` and migration SQL from `migrations-configmap.yaml`.
- A daily cleanup CronJob (`wow-backend-token-cleanup`) removes expired rows from auth token tables.
- Replace placeholders in `secrets.example.yaml` before applying.
- `AZEROTH_CORE_MYSQL_DATABASE_URL` should point to your AzerothCore MySQL endpoint reachable from the cluster.
- For multi-node clusters, a local/private registry mirror is better than `imagePullPolicy: Never` (which requires importing the image on every node).
