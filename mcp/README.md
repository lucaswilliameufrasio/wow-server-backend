# wow-mcp

MCP Server (stdio) read-only para a API do `wow-server-backend`. O MCP não fala com os
bancos do AzerothCore — todas as operações passam pela API HTTP existente, que é a fonte
única das regras e permissões.

Baseado no SDK oficial [rmcp](https://github.com/modelcontextprotocol/rust-sdk) (`rmcp` 3.2).

## Tools

| Tool | Endpoint da API |
|---|---|
| `list_online_players` | `GET /v1/admin/online-players` |
| `search_players` | `GET /v1/admin/players` |
| `get_player_locations` | `GET /v1/admin/players/{id}/locations` |
| `search_items` | `GET /v1/items` |
| `get_item` | `GET /v1/items/{entry}` |
| `get_health` | `GET /health-check` |
| `get_metrics` | `GET :METRICS_PORT/metrics` (Prometheus) |
| `get_audit_log` | `GET /v1/admin/audit-log` (requer `audit:read`) |
| `lock_account` | `PATCH /v1/admin/players/{id}/lock` (requer `players:write`) |

Todas as tools read-only são marcadas com `read_only_hint = true`; `lock_account` é
a única mutação, com `destructive_hint = true` e `dry_run` default `true`.

## Resources

- `server://health`
- `server://metrics`
- `players://online`
- `item://{entry}` (template)

## Privacidade

Os DTOs do MCP **não** incluem `email` nem `last_ip` das contas — esses campos são
descartados na fronteira MCP e nunca chegam ao modelo, mesmo que a API os retorne.

## Configuração

| Variável | Padrão | Descrição |
|---|---|---|
| `MCP_API_BASE_URL` | `http://127.0.0.1:3000` | Base URL da API |
| `MCP_METRICS_BASE_URL` | `http://127.0.0.1:9090` | Base URL do servidor de métricas (Prometheus) |
| `MCP_API_TOKEN` | — | Bearer token (service token `wowst_...` ou access token JWT) |
| `MCP_REQUEST_TIMEOUT_SECS` | `30` | Timeout por request |
| `RUST_LOG` | `warn` | Log vai para stderr (stdout é o canal do MCP) |

## Como usar agora

### 1. Subir a API

```bash
# dev local (precisa de PostgreSQL + MySQL do AzerothCore)
make dev

# ou VPS com docker compose
cd deploy/vps && ./wowctl up
```

### 2. Obter um token

**Opção A — service token (recomendado, longa duração e revogável):**

```bash
# 1) sign-in com uma conta admin (GM level 3) para pegar um JWT temporário
JWT=$(curl -s -X POST http://127.0.0.1:3000/v1/auth/sign-in \
  -H 'Content-Type: application/json' \
  -d '{"username":"Admin","password":"SuaSenha"}' | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')

# 2) criar o service token (guarde: só aparece uma vez)
curl -s -X POST http://127.0.0.1:3000/v1/admin/service-tokens \
  -H "Authorization: Bearer $JWT" -H 'Content-Type: application/json' \
  -d '{"name":"mcp-local","expires_in_days":90}'

export MCP_API_TOKEN="wowst_..."
```

**Opção B — JWT de 15 min (teste rápido):**

```bash
export MCP_API_TOKEN=$(curl -s -X POST http://127.0.0.1:3000/v1/auth/sign-in \
  -H 'Content-Type: application/json' \
  -d '{"username":"Admin","password":"SuaSenha"}' \
  | python3 -c 'import json,sys;print(json.load(sys.stdin)["access_token"])')
```

O token precisa pertencer a uma conta com `players:read` (GM level >= 1) para as tools
de admin e `items:read` (qualquer conta) para as tools de itens.

### 3. Rodar o MCP

```bash
cargo build -p wow-mcp
export MCP_API_BASE_URL=http://127.0.0.1:3000   # ou o IP Tailscale/SSH tunnel da VPS
export MCP_METRICS_BASE_URL=http://127.0.0.1:9090
./target/debug/wow-mcp
```

### 4. Testar sem cliente de IA

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | ./target/debug/wow-mcp
```

### 5. Perguntas que já funcionam

- "Quem está online agora e em que zona?"
- "Onde estão os personagens da conta 3?"
- "Qual o item de entry 19019?"
- "A API está saudável? Mostra as métricas."

## Registro por cliente

### opencode (`~/.config/opencode/opencode.json`)

```json
{
  "mcp": {
    "wow-backend": {
      "type": "local",
      "command": ["/caminho/para/wow-server-backend/target/debug/wow-mcp"],
      "environment": {
        "MCP_API_BASE_URL": "http://127.0.0.1:3000",
        "MCP_METRICS_BASE_URL": "http://127.0.0.1:9090",
        "MCP_API_TOKEN": "wowst_..."
      }
    }
  }
}
```

### Claude Desktop (`claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "wow-backend": {
      "command": "/caminho/para/wow-server-backend/target/debug/wow-mcp",
      "env": {
        "MCP_API_BASE_URL": "http://127.0.0.1:3000",
        "MCP_METRICS_BASE_URL": "http://127.0.0.1:9090",
        "MCP_API_TOKEN": "wowst_..."
      }
    }
  }
}
```

### Claude Code

```bash
claude mcp add wow-backend \
  -e MCP_API_BASE_URL=http://127.0.0.1:3000 \
  -e MCP_METRICS_BASE_URL=http://127.0.0.1:9090 \
  -e MCP_API_TOKEN=wowst_... \
  -- /caminho/para/wow-server-backend/target/debug/wow-mcp
```

### Cursor (`.cursor/mcp.json`)

```json
{
  "mcpServers": {
    "wow-backend": {
      "command": "/caminho/para/wow-server-backend/target/debug/wow-mcp",
      "env": {
        "MCP_API_BASE_URL": "http://127.0.0.1:3000",
        "MCP_METRICS_BASE_URL": "http://127.0.0.1:9090",
        "MCP_API_TOKEN": "wowst_..."
      }
    }
  }
}
```

> Acesso remoto à VPS: SSH tunnel (`ssh -L 3000:127.0.0.1:3000 -L 9090:127.0.0.1:9090 user@vps`)
> ou Tailscale/NetBird — o mesmo perfil privado da API. Nada é publicado na internet.

## Mutações (confirmação obrigatória)

`lock_account` é destrutiva e exige dois passos:

1. `lock_account(account_id=5, locked=true, reason="gold farming")` → retorna o
   preview (`dry_run: true`) sem alterar nada;
2. `lock_account(account_id=5, locked=true, reason="gold farming", dry_run=false)`
   → aplica e registra no audit log.

Toda mutação e toda negação de permissão ficam em `admin_audit_log` (PostgreSQL),
consultável pela tool `get_audit_log`. O reason informado vai junto no registro.

## Testes

```bash
cargo test -p wow-mcp
```

Os testes usam um mock server Axum local; nenhum banco ou API real é necessário.
