# WoW Server Backend Ops Guide

This repository includes API services plus operational tooling for fast weekend server runs.

## Quick paths

- **[Docker weekend VPS (recomendado)](docs/docker-weekend-vps.md)** — Stack simples com Docker Compose, sem Kubernetes
- [Dev setup (local)](docs/dev-setup.md) — Para desenvolvimento na máquina local
- [Deployment targets (k3s, Railway, Docker/Portainer)](docs/deployment-targets.md)
- [Existing server updates](docs/update-existing-server.md)
- [Fast raid vendors](docs/fast-raid-vendors.md)
- [Fixed spec-BiS vendors](docs/spec-bis-vendors.md)

## Most common commands

```bash
# New VPS (Docker, sem Kubernetes)
# Siga o guia: docs/docker-weekend-vps.md

# Dev local
make setup
make dev

# Seeds
make seed-fast-raid-vendors
make seed-spec-bis-vendors
```

## Optional notifications (Discord + Telegram)

```bash
export NOTIFY_ENABLED=true
export DISCORD_WEBHOOK_URL="https://discord.com/api/webhooks/..."
export TELEGRAM_BOT_TOKEN="123456:ABC..."
export TELEGRAM_CHAT_ID="-1001234567890"
```
