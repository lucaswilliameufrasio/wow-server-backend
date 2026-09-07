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
| `get_metrics` | `GET /metrics` |

Todas as tools são marcadas com `read_only_hint = true`.

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
| `MCP_API_TOKEN` | — | Bearer token (access token de uma conta com `players:read`) |
| `MCP_REQUEST_TIMEOUT_SECS` | `30` | Timeout por request |
| `RUST_LOG` | `warn` | Log vai para stderr (stdout é o canal do MCP) |

## Build e uso

```bash
cargo build -p wow-mcp
./target/debug/wow-mcp
```

Exemplo de registro em um cliente MCP (ex. `opencode.json`):

```json
{
  "mcp": {
    "wow-backend": {
      "type": "local",
      "command": ["./target/debug/wow-mcp"],
      "environment": {
        "MCP_API_BASE_URL": "http://127.0.0.1:3000",
        "MCP_API_TOKEN": "<access_token>"
      }
    }
  }
}
```

Para acesso remoto, use SSH tunnel ou Tailscale até a VPS — o mesmo perfil privado
da API (que escuta apenas em `127.0.0.1`).

## Testes

```bash
cargo test -p wow-mcp
```

Os testes usam um mock server Axum local; nenhum banco ou API real é necessário.
