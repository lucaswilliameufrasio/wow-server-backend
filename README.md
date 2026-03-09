# WoW Server Backend Ops Guide

This repository includes API services plus operational tooling for fast weekend server runs.

## Quick paths

- New server bootstrap (fresh VPS): `docs/weekend-quickstart.md`
- One-command weekend run (existing server): `docs/weekend-go.md`
- Deployment + maintenance runbook: `docs/deployment-maintenance.md`
- Deployment targets (k3s, Railway, Docker/Portainer): `docs/deployment-targets.md`
- Existing server updates: `docs/update-existing-server.md`
- Fast raid vendors: `docs/fast-raid-vendors.md`
- Fixed spec-BiS vendors: `docs/spec-bis-vendors.md`

## Most common commands

```bash
# New server (fresh VPS)
./scripts/provision-weekend-server.sh

# Existing server, full safe flow
make weekend-go

# Update only
make update-server

# Backup only
make backup-before-update

# Seed fast auto-filtered class+misc vendors
make seed-fast-raid-vendors

# Seed fixed ICC-style preset vendors
make seed-spec-bis-vendors
```

## Optional notifications (Discord + Telegram)

```bash
export NOTIFY_ENABLED=true
export DISCORD_WEBHOOK_URL="https://discord.com/api/webhooks/..."
export TELEGRAM_BOT_TOKEN="123456:ABC..."
export TELEGRAM_CHAT_ID="-1001234567890"
```
