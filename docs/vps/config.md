# VPS — Configuração avançada

## Caddy HTTPS (opcional)

Se você tem um domínio apontado para a VPS, pode expor a API com HTTPS.

No `.env`:

```bash
WOW_CADY_DOMAIN=wow.seuservidor.com
WOW_CADY_EMAIL=admin@seuservidor.com
```

Arquivo `Caddyfile` em `deploy/vps/`:

```
{$WOW_CADY_DOMAIN} {
    reverse_proxy wow-backend:3000
}
```

Ative com:

```bash
docker compose -f compose.yml -f caddy-compose.yml up -d
```

> Nota: com Caddy ativo, a API fica acessível na internet.  
> Caddy gerencia TLS automaticamente via Let's Encrypt.

## API acessível via Tailscale/NetBird (sem Caddy)

A API escuta em `127.0.0.1:3000`. Para acessá-la remotamente sem SSH:

1. Descubra o IP da VPS na interface da VPN:

```bash
tailscale ip --4           # Tailscale
nmctl status               # NetBird
```

2. Altere o bind do `.env`:

```bash
WOW_API_BIND=100.64.0.1    # IP Tailscale da VPS
```

3. Atualize o compose para publicar no IP da VPN:

```yaml
ports:
  - "${WOW_API_BIND:-127.0.0.1}:${WOW_API_PORT:-3000}:${WOW_API_PORT:-3000}"
```

## Variáveis de ambiente completas

| Variável | Padrão | Descrição |
|---|---|---|
| `WOW_PG_PASSWORD` | **obrigatório** | Senha do PostgreSQL |
| `WOW_PG_USER` | `wow_api` | Usuário PostgreSQL |
| `WOW_PG_DB` | `wow_app` | Database PostgreSQL |
| `WOW_MYSQL_PASSWORD` | **obrigatório** | Senha root MySQL |
| `WOW_MYSQL_USER` | `root` | Usuário MySQL da API |
| `WOW_API_PORT` | `3000` | Porta da API |
| `WOW_METRICS_PORT` | `9090` | Porta do servidor de métricas Prometheus (interno, não publicado pelo compose) |
| `WOW_RUST_LOG` | `info` | Nível de log da API |
| `WOW_SRP6_CORE5_MODE` | `false` | Formato do verifier SRP6. `false` = AzerothCore master (padrão); `true` = formato "core 5" de outros cores. Ver [SRP6](#srp6-e-criacao-de-contas) |
| `WOW_ENABLE_GM_COMMANDS` | `false` | **Perigoso**: habilita `POST /v1/admin/server/command` (GM raw). Mantenha off |
| `WOW_GM_COMMAND_ALLOWLIST` | — | Prefixos de comando permitidos, separados por vírgula (ex.: `server info,reload config`) |
| `WOW_JWT_ISSUER` | `wow-backend` | Emissor JWT |
| `WOW_JWT_AUDIENCE` | `wow-web` | Audiência JWT |
| `WOW_JWT_EXPIRES_MINUTES` | `15` | Expiração do access token |
| `WOW_JWT_REFRESH_EXPIRES_DAYS` | `30` | Expiração do refresh token |
| `WOW_BACKEND_IMAGE` | `ghcr.io/lucaswilliameufrasio/wow-server-backend` | Imagem do backend |
| `WOW_BACKEND_TAG` | `latest` | Tag da imagem |
| `WOW_ACORE_PROJECT` | `azerothcore-wotlk` | Nome do projeto Compose do AC |
| `WOW_ACORE_NETWORK` | `azerothcore-wotlk_ac-network` | Rede Docker externa criada pelo compose do AC |
| `WOW_ACORE_DIR` | `/opt/azerothcore-wotlk` | Caminho do checkout do AC |
| `WOW_BACKUP_DIR` | `/srv/wow/backups` | Diretório de backups |
| `WOW_BACKUP_RETENTION_DAYS` | `30` | Retenção em dias |
| `WOW_API_BIND` | `127.0.0.1` | IP para bind da API |

## SRP6 e criação de contas

O backend gera o *verifier* SRP6 usado pelo authserver do WoW. O toggle
`WOW_SRP6_CORE5_MODE` escolhe o formato:

- `false` (padrão) — SRP6 padrão, **o que o AzerothCore master aceita**:
  salt na ordem normal, `x = little-endian(SHA1(salt || SHA1("USER:SENHA")))`,
  verifier gravado little-endian.
- `true` — formato "core 5" (salt/verifier invertidos), usado por outras
  variantes de core WoW. **No AzerothCore master isso faz o login falhar**
  com "invalid password".

Se o login falhar com "invalid password", confira se está `false`, recrie o
container do backend e recrie a conta. Detalhes em
[troubleshooting.md](troubleshooting.md#1-login-falha-com-invalid-password-srp6--o-mais-importante).

## Rede Docker

```
                    wow-shared (internal)
    ┌───────────────────────────────────┐
    │  app-postgres:5432                │
    │  wow-backend:3000                 │
    └──────────────┬────────────────────┘
                   │
           wow-acore (external)
    ┌──────────────┴────────────────────┐
    │  ac-database:3306                 │
    │  ac-authserver:3724 (public)      │
    │  ac-worldserver:8085 (public)     │
    │  ac-worldserver:7878 (SOAP, local)│
    └───────────────────────────────────┘
```

## Portas publicadas

| Porta | Serviço | Bind | Público |
|---|---|---|---|
| 3724/tcp | WoW auth | `0.0.0.0` | Sim |
| 8085/tcp | WoW world | `0.0.0.0` | Sim |
| 7878/tcp | SOAP | `127.0.0.1` | Não |
| 3306/tcp | MySQL | `127.0.0.1` | Não |
| 5432/tcp | PostgreSQL | `127.0.0.1` | Não |
| 3000/tcp | API | `127.0.0.1` | Não (padrão) |
| 9090/tcp | Métricas Prometheus | interno | Não (só rede Docker; sem `ports:` no compose) |
