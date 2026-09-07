# MCP Server — Spec (AzerothCore Admin)

Plano para expor as capacidades administrativas do backend como um **MCP Server**, sem
duplicar lógica de negócio e sem acesso direto aos bancos `acore_auth`, `acore_characters`
ou `acore_world`.

## Arquitetura

O backend Rust (Axum) é a **fonte única das operações administrativas**. Hoje ele fala
direto com o MySQL do AzerothCore via repos (`src/repos.rs`). O MCP é uma camada fina
que consome a API HTTP existente:

```text
Agente (Claude/ChatGPT/IDE)
        │
       MCP (stdio)
        │
        ▼
wow-server-backend (API HTTP Rust)   ← regras, RBAC, validação vivem aqui
        │
        ▼
acore_auth / acore_characters / acore_world (MySQL)
```

Regras:

- O MCP **não** abre conexão com MySQL/PostgreSQL.
- O MCP **não** reimplementa RBAC, validações ou queries — só repassa erros da API.
- Um futuro painel web seria apenas outro cliente da mesma API.

## Autenticação

- Service tokens da API (`wowst_...`): longa duração, revogáveis, hash SHA-256 no
  banco, herdam o RBAC da conta criadora (admin via `POST /v1/admin/service-tokens`).
  O MCP envia `Authorization: Bearer wowst_...`; nenhum token persiste em disco.
- Alternativa de teste: access token JWT de 15 min.
- Config via env/secret file: `MCP_API_BASE_URL`, `MCP_METRICS_BASE_URL`,
  `MCP_API_TOKEN`, `MCP_REQUEST_TIMEOUT_SECS`.
- Transporte **stdio** (uso local via SSH tunnel/Tailscale, mesmo perfil privado da API).
  Streamable HTTP só se houver necessidade de acesso remoto multiusuário.

## Tools (v1 — read-only, exponíveis hoje)

Baseada nos endpoints que já existem:

| Tool | Fonte (API atual) | Permissão |
|---|---|---|
| `list_online_players(limit?)` | `GET /v1/admin/online-players` | `players:read` |
| `search_players(search?, online?, limit?, cursor?)` | `GET /v1/admin/players` | `players:read` |
| `get_player_locations(account_id)` | `GET /v1/admin/players/{id}/locations` | `players:read` |
| `search_items(search?, class?, limit?, cursor?)` | `GET /v1/items` | `items:read` |
| `get_item(entry)` | `GET /v1/items/{entry}` | `items:read` |
| `get_health()` | `GET /health-check` | pública |
| `get_metrics()` | `GET :METRICS_PORT/metrics` (Prometheus) | porta dedicada e privada |

## Resources (v1)

Contexto consultável (não-parametrizado) vira resource; busca parametrizada vira tool:

- `server://health` → health-check + uptime
- `server://metrics` → snapshot Prometheus
- `players://online` → jogadores online agora
- `item://{entry}` → item por entry

## Tools (v2 — mutações)

| Tool | Fonte | Notas |
|---|---|---|
| `lock_account(account_id, locked, reason, dry_run?)` | `PATCH /v1/admin/players/{id}/lock` | única mutação da API hoje |

Requisitos para toda mutação:

- `dry_run` default `true` na primeira chamada; confirmação explícita do operador;
- annotation `destructive` no tool;
- audit log na API (ator, tool, args redigidos, alvo, resultado, timestamp);
- idempotência (chamar de novo com mesmo estado não é erro).

## Tools (v3 — requer novas capacidades na API)

Dependem de integração com o WorldServer (módulo/plugin AzerothCore ou SOAP local
orquestrado pela API — nunca o MCP falando SOAP/banco direto):

- `get_server_status()` — uptime/tick do realm (além do uptime da API)
- `kick_player(name, reason)`
- `ban_account(...)` / `unban_account(...)`
- `send_announcement(message)` / `send_server_message(message)`
- `schedule_restart(delay)`
- `get_crashes()` / `get_server_logs(tail)`
- `teleport_player(...)`, `give_item(...)`, `modify_money(...)`, `modify_level(...)`
- `execute_gm_command(...)` — **desabilitado por padrão**; exige permissão dedicada,
  allowlist de comandos e confirmação forte.

## Faltando na API (pré-requisitos para v2/v3)

1. **Feito** — service tokens (`wowst_...`, permissão `tokens:manage`, endpoints
   `GET/POST /v1/admin/service-tokens`, `DELETE /v1/admin/service-tokens/{id}`).
2. Audit log persistente (PostgreSQL já está disponível para isso).
3. RBAC granular além de `players:read`/`players:write` (ex.: `server:restart`,
   `players:kick`, `players:ban`).
4. **Feito no MCP** — redação de PII na fronteira (DTOs sem `email`/`last_ip`);
   a API ainda os retorna para clientes confiáveis.
5. **Feito** — `/metrics` em porta dedicada (`METRICS_PORT` 9090, bind privado);
   `/swagger-ui` continua local-only na API.
6. Camada de integração com o WorldServer (módulo do core ou SOAP encapsulado na API).

## Tratamento de erros

- O MCP repassa o payload padrão `{ message, error_code, extra? }` da API.
- `401` → renova token e tenta uma vez; falhou de novo, erro de credencial de serviço.
- `403` → erro de permissão (o modelo deve ver o `error_code`, não contornar).
- `404`/`409` → resultados vazios/erros de negócio normais.
- Timeout da API (30s) e indisponibilidade viram erros explícitos, nunca retry cego
  em mutações.

## Estrutura (implementada)

```text
Cargo.toml              # workspace: raiz (API) + mcp/
mcp/
  Cargo.toml            # crate wow-mcp (rmcp 3.2, reqwest, stdio)
  src/
    main.rs             # bootstrap stdio + tracing em stderr
    config.rs           # MCP_API_BASE_URL / MCP_API_TOKEN / MCP_REQUEST_TIMEOUT_SECS
    api_client.rs       # HTTP client da API (única fronteira)
    dto.rs              # DTOs sem email/last_ip (redação de PII na fronteira)
    wow_mcp.rs          # #[tool_router] + ServerHandler (tools e resources)
    mock.rs             # mock server Axum para testes
  README.md
```

## Plano incremental

1. **Feito** — sanitizar histórico (caminhos locais), gitleaks no CI.
2. **Feito** — spec (este documento).
3. **Feito** — MCP v1 read-only (crate `wow-mcp`, SDK rmcp 3.2, transporte stdio):
   7 tools, 3 resources + template `item://{entry}`, redação de PII, testes com mock.
4. **Feito** — service tokens (`wowst_...`) na API + `/metrics` em porta privada.
5. Audit log + `lock_account` com `dry_run`/confirmação.
6. Integração WorldServer (módulo do core ou SOAP) → status/kick/ban/anúncio/restart.
7. Por último: teleport, give item, dinheiro, level, e `execute_gm_command` atrás de
   allowlist + flag de config explícita.
