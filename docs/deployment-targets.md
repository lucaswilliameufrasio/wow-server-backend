# Deployment targets

This doc covers publishing images and deploying to:
- k3s (CloudNativePG 18)
- Railway
- Docker (direct or Portainer)
- Docker + Postgres

## 1) Build/publish image

Set your image name:

```bash
export IMAGE_NAME=ghcr.io/your-org/wow-server-backend
export TAG=latest
make docker-publish
```

Local build (no push):

```bash
make docker-build
```

Ship to remote Docker host over SSH (Portainer-friendly):

```bash
export SSH_USER=root
export SSH_HOST=your-host
make docker-ship
```

## 2) k3s (CloudNativePG 18)

Use local image import for single-node k3s:

```bash
make docker-build
docker save wow-server-backend:latest | sudo k3s ctr images import -
kubectl rollout restart deployment/wow-backend -n wow-backend
```

Or use a registry image:
- set `image:` in `k8s/backend-deployment.yaml` to your registry tag
- apply: `kubectl apply -f k8s/backend-deployment.yaml`

Full deployment steps:
- `docs/deployment-maintenance.md`
- `docs/weekend-quickstart.md`

## 3) Railway

Railway can build from `Dockerfile.production`.

Steps:
1. Create a new service and point it at this repo.
2. Set build file: `Dockerfile.production`
3. Expose port `3000`
4. Configure env:
   - `AZEROTH_CORE_MYSQL_DATABASE_URL`
   - `APP_POSTGRES_DATABASE_URL`
   - `JWT_PRIVATE_KEY_PEM` or `JWT_PRIVATE_KEY_PATH`
   - `JWT_PUBLIC_KEY_PEM` or `JWT_PUBLIC_KEY_PATH`
   - `JWT_ISSUER`, `JWT_AUDIENCE`
   - `SRP6_CORE5_MODE` (true/false)
5. Add a Postgres service (if not external) and use its `DATABASE_URL` for `APP_POSTGRES_DATABASE_URL`.

Note: ensure Railway can reach your AzerothCore MySQL host.

## 4) Docker (direct)

```bash
make docker-build

docker run -d --name wow-backend \
  --restart unless-stopped \
  -p 3000:3000 \
  -e AZEROTH_CORE_MYSQL_DATABASE_URL=... \
  -e APP_POSTGRES_DATABASE_URL=... \
  -e JWT_PRIVATE_KEY_PEM="-----BEGIN RSA PRIVATE KEY-----..." \
  -e JWT_PUBLIC_KEY_PEM="-----BEGIN PUBLIC KEY-----..." \
  $(IMAGE_NAME):$(TAG)
```

## 5) Docker + Postgres (compose)

Use `docker-compose.prod.yml`:

```bash
cp .env.example .env
make docker-compose-prod-up
```

Set in `.env`:
- `AZEROTH_CORE_MYSQL_DATABASE_URL`
- `JWT_PRIVATE_KEY_PATH` + `JWT_PUBLIC_KEY_PATH` (default uses `.secrets/`)
- `APP_POSTGRES_DB`, `APP_POSTGRES_USER`, `APP_POSTGRES_PASSWORD`

## 6) Portainer

1. `make docker-ship` to load image into the remote host
2. In Portainer, update the stack to use the new image tag
3. Redeploy stack

## Notes

- The app runs migrations on startup. For production, keep migration SQL in image.
- If you use the k3s migration Job, keep using it as the source of truth.
