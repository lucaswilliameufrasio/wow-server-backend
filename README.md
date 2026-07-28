# WoW Server Backend

API Rust + ferramentas de operação para servidor WoW WotLK 3.3.5a com AzerothCore.

## Deploy oficial: VPS com Docker Compose

Uma VPS Linux com Docker Compose é o destino oficial e mais simples.

```bash
git clone <URL> /opt/wow-backend
git clone --depth 1 https://github.com/azerothcore/azerothcore-wotlk.git /opt/azerothcore-wotlk
cd /opt/wow-backend/deploy/vps
cp .env.example .env
# edite as senhas no .env
./wowctl install
./wowctl up
./wowctl set-realm <IP_PUBLICO>
./wowctl smoke-test
```

Documentação completa em [docs/vps/install.md](docs/vps/install.md).

## Comandos rápidos

| Ação | Comando |
|---|---|
| Subir tudo | `./wowctl up` |
| Parar tudo | `./wowctl down` |
| Status | `./wowctl status` |
| Logs | `./wowctl logs` |
| Backup | `./wowctl backup` |
| Smoke test | `./wowctl smoke-test` |
| Dev local | `make dev` |
| Criar conta GM | `./wowctl create-account <user> <pass> 3` |

## Documentação

- **[docs/vps/](docs/vps/)** — Deploy VPS (instalação, operação, backup, configuração)
- `docs/dev-setup.md` — Desenvolvimento local
- `docs/fast-raid-vendors.md` — Vendors rápidos de raid
- `docs/spec-bis-vendors.md` — Vendors de BiS por spec

## Deploys alternativos (não oficiais)

- `docs/deployment-targets.md` — k3s, Railway, Portainer
- `docs/deployment-maintenance.md` — Manutenção k3s
- `docker-compose.prod.yml` — Production Compose legado (usar deploy/vps/ no lugar)
- `k8s/` — Manifests Kubernetes legados

## Notificações (Discord + Telegram)

```bash
export NOTIFY_ENABLED=true
export DISCORD_WEBHOOK_URL="https://discord.com/api/webhooks/..."
export TELEGRAM_BOT_TOKEN="123456:ABC..."
export TELEGRAM_CHAT_ID="-1001234567890"
```
