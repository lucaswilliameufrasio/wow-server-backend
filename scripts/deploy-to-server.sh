#!/usr/bin/env bash
set -euo pipefail

: "${SSH_TARGET:?Set SSH_TARGET (SSH_ALIAS or SSH_USER@SSH_HOST)}"
: "${REMOTE_DIR:=/opt/wow-backend/deploy/vps}"
: "${REMOTE_ACORE_DIR:=/opt/azerothcore-wotlk}"
: "${IMAGE_NAME:=wow-server-backend}"
: "${WEB_IMAGE_NAME:=wow-server-backend-web}"
: "${TAG:=latest}"
: "${TARGET_PLATFORM:=linux/amd64}"
: "${ACORE_REPO_URL:=https://github.com/azerothcore/azerothcore-wotlk.git}"
: "${ACORE_IMAGE_TAG:=}"

temporary_acore_dir=''
cleanup() {
  [[ -z "$temporary_acore_dir" ]] || rm -rf "$temporary_acore_dir"
}
trap cleanup EXIT

for value in "$REMOTE_DIR" "$REMOTE_ACORE_DIR" "$IMAGE_NAME" "$WEB_IMAGE_NAME" "$TAG"; do
  [[ "$value" =~ ^[a-zA-Z0-9_./:-]+$ ]] || { echo "Unsupported character in deploy parameter: $value" >&2; exit 2; }
done

remote_repo_dir="$(dirname "$(dirname "$REMOTE_DIR")")"

echo "[deploy] Preparing remote checkout and VPS configuration"
ssh "$SSH_TARGET" "mkdir -p '$REMOTE_DIR' '$remote_repo_dir/.secrets' '$remote_repo_dir/scripts'"

# Install only provisions config, credentials, and the AzerothCore source. It
# does not start containers or build images.
# Sync operational code without replacing generated secrets or .env.
scp deploy/vps/wowctl deploy/vps/compose.yml deploy/vps/.env.example \
  deploy/vps/Caddyfile deploy/vps/wow-backup.sh "$SSH_TARGET:$REMOTE_DIR/"
scp -r scripts "$SSH_TARGET:$remote_repo_dir/"
ssh "$SSH_TARGET" "chmod +x '$REMOTE_DIR/wowctl' '$REMOTE_DIR/wow-backup.sh' && cd '$REMOTE_DIR' && if [ ! -f .env ]; then WOW_ACORE_DIR='$REMOTE_ACORE_DIR' ./wowctl install; fi"
if [[ -z "$ACORE_IMAGE_TAG" ]]; then
  ACORE_IMAGE_TAG="$(ssh "$SSH_TARGET" "awk -F= '\$1 == \"DOCKER_IMAGE_TAG\" { value=\$2 } END { print value }' '$REMOTE_ACORE_DIR/.env' 2>/dev/null")"
  ACORE_IMAGE_TAG="${ACORE_IMAGE_TAG:-master}"
fi
[[ "$ACORE_IMAGE_TAG" =~ ^[a-zA-Z0-9_.-]+$ ]] || { echo "Unsupported AzerothCore image tag: $ACORE_IMAGE_TAG" >&2; exit 2; }

echo "[deploy] Building backend and web images locally"
docker buildx inspect container-builder >/dev/null 2>&1 || docker buildx create --name container-builder --driver docker-container --use >/dev/null
docker buildx build --builder container-builder --platform "$TARGET_PLATFORM" \
  -t "$IMAGE_NAME:$TAG" -f Dockerfile.production --load .
docker buildx build --builder container-builder --platform "$TARGET_PLATFORM" \
  -t "$WEB_IMAGE_NAME:$TAG" -f web/Dockerfile.production --load .

ac_images=(
  "acore/ac-wotlk-db-import:$ACORE_IMAGE_TAG"
  "acore/ac-wotlk-worldserver:$ACORE_IMAGE_TAG"
  "acore/ac-wotlk-authserver:$ACORE_IMAGE_TAG"
  "acore/ac-wotlk-client-data:$ACORE_IMAGE_TAG"
)
if ! ssh "$SSH_TARGET" "docker image inspect '${ac_images[0]}' '${ac_images[1]}' '${ac_images[2]}' '${ac_images[3]}' >/dev/null 2>&1"; then
  echo "[deploy] AzerothCore images are missing remotely; building them locally"
  remote_acore_commit="$(ssh "$SSH_TARGET" "git -C '$REMOTE_ACORE_DIR' rev-parse HEAD")"
  acore_build_dir="${ACORE_REPO_DIR:-}"
  if [[ -z "$acore_build_dir" ]]; then
    acore_build_dir="$(mktemp -d "${TMPDIR:-/tmp}/wow-acore-build.XXXXXX")"
    temporary_acore_dir="$acore_build_dir"
    rmdir "$acore_build_dir"
    git clone --depth 1 --branch master "$ACORE_REPO_URL" "$acore_build_dir"
    if [[ "$(git -C "$acore_build_dir" rev-parse HEAD)" != "$remote_acore_commit" ]]; then
      git -C "$acore_build_dir" fetch --depth 1 origin "$remote_acore_commit"
      git -C "$acore_build_dir" checkout --detach "$remote_acore_commit"
    fi
  fi
  [[ -f "$acore_build_dir/docker-compose.yml" ]] || { echo "AzerothCore compose not found under $acore_build_dir" >&2; exit 1; }
  local_acore_commit="$(git -C "$acore_build_dir" rev-parse HEAD)"
  [[ "$local_acore_commit" == "$remote_acore_commit" ]] || {
    echo "Local AzerothCore checkout ($local_acore_commit) differs from remote ($remote_acore_commit)." >&2
    echo "Use ACORE_REPO_DIR at the matching commit, or let this target clone it locally." >&2
    exit 1
  }
  DOCKER_IMAGE_TAG="$ACORE_IMAGE_TAG" docker compose -f "$acore_build_dir/docker-compose.yml" \
    --project-name "wow-acore-image-build" build ac-db-import ac-worldserver ac-authserver ac-client-data-init
fi

all_images=("$IMAGE_NAME:$TAG" "$WEB_IMAGE_NAME:$TAG" "${ac_images[@]}")
echo "[deploy] Transferring prebuilt images to $SSH_TARGET"
docker save "${all_images[@]}" | ssh "$SSH_TARGET" docker load

echo "[deploy] Starting AzerothCore and deploying the application"
ssh "$SSH_TARGET" "cd '$REMOTE_DIR' && WOW_ACORE_DIR='$REMOTE_ACORE_DIR' WOWCTL_NO_BUILD=true ./wowctl up --ac-only"
ssh "$SSH_TARGET" "cd '$REMOTE_DIR' && WOWCTL_NO_BUILD=true ./wowctl deploy-image '$IMAGE_NAME' '$TAG' '$WEB_IMAGE_NAME' '$TAG'"
