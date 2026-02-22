# Weekend one-command flow

Run everything in order:
1. preflight checks
2. backup
3. AzerothCore + backend update
4. fast raid vendor seed
5. smoke checks

Command:

```bash
make weekend-go
```

## Optional notifications (Discord + Telegram)

```bash
export NOTIFY_ENABLED=true
export DISCORD_WEBHOOK_URL="https://discord.com/api/webhooks/..."
export TELEGRAM_BOT_TOKEN="123456:ABC..."
export TELEGRAM_CHAT_ID="-1001234567890"
make weekend-go
```

## Optional skips

```bash
SKIP_BACKUP=true make weekend-go
SKIP_VENDOR_SEED=true make weekend-go
SKIP_UPDATE=true make weekend-go
```
