# Docker weekend VPS quickstart

Stack completa: **AzerothCore WotLK 3.3.5a + API Rust** — sem Kubernetes, sem CloudNativePG, apenas Docker Compose.

## 0) Requisitos

- VPS Linux (Ubuntu 22.04/24.04 recomendado)
- 2 vCPU, 4 GB RAM, 20 GB disco
- Docker instalado
- Portas liberadas no firewall: `3724`, `8085`, `3000`

## 1) Instalar dependências

```bash
apt update && apt install -y git curl openssl
curl -fsSL https://get.docker.com | sh
```

## 2) Clonar repositórios

```bash
# Backend da API
git clone <URL_DO_SEU_REPOSITORIO> /opt/wow-server-backend

# AzerothCore oficial
git clone --depth 1 https://github.com/azerothcore/azerothcore-wotlk.git /opt/azerothcore-wotlk
```

## 3) Subir AzerothCore

NÃO use `--build`. As imagens oficiais pré-compiladas são baixadas automaticamente.

```bash
cd /opt/azerothcore-wotlk
docker compose up -d
```

Aguarde 1-2 minutos. Verifique:

```bash
docker compose ps
```

Saída esperada:

```
NAME                STATUS
ac-authserver       Up
ac-database         Up (healthy)
ac-worldserver      Up
```

**Se o `ac-db-import` falhar na primeira tentativa** (race condition com o MySQL), execute:

```bash
docker compose up -d ac-db-import
docker compose up -d ac-authserver ac-worldserver
```

## 4) Gerar chaves JWT

```bash
cd /opt/wow-server-backend
make jwt-keys
```

## 5) Subir banco da API e backend

A rede criada pelo AzerothCore se chama `azerothcore-wotlk_ac-network`. A API precisa estar nela para enxergar o `ac-database`.

```bash
cd /opt/wow-server-backend

ACORE_DOCKER_NETWORK=azerothcore-wotlk_ac-network \
PORT=3000 \
docker compose -f docker-compose.dev.yml up -d
```

Verifique:

```bash
curl -s http://127.0.0.1:3000/health-check
```

Resposta esperada:

```json
{"message":"ok"}
```

## 6) Liberar firewall

```bash
ufw allow OpenSSH
ufw allow 3724/tcp
ufw allow 8085/tcp
ufw allow 3000/tcp
ufw enable
```

NÃO libere `3306` (MySQL) nem `5432` (PostgreSQL) para a internet.

## 7) Criar contas WoW

```bash
curl -X POST http://127.0.0.1:3000/v1/auth/register \
  -H 'content-type: application/json' \
  -d '{"username":"admin","password":"Admin12345"}'
```

Para tornar a conta GM (administradora):

```bash
docker exec ac-database mysql -uroot -ppassword acore_auth \
  -e "INSERT INTO account_access (id, gmlevel, RealmID) VALUES (1, 3, -1)"
```

O `id` é o `account_id` retornado no registro.

## 8) Configurar clientes WoW

No arquivo `realmlist.wtf` do WoW 3.3.5a:

```text
set realmlist IP_PUBLICO_DA_VPS
```

## 9) Credenciais padrão

### MySQL AzerothCore

```
host:     ac-database (dentro da rede Docker)
port:     3306
user:     root
password: password

bancos:   acore_auth, acore_characters, acore_world
```

### PostgreSQL da API

```
host:     app-postgres (dentro da rede Docker)
port:     5432
database: wow_app
user:     postgres
password: password
```

### JWT

```
private: .secrets/jwt_private.pem
public:  .secrets/jwt_public.pem
```

## 10) Logs e diagnóstico

```bash
# Logs da API
docker compose -f /opt/wow-server-backend/docker-compose.dev.yml logs -f --tail=50

# Logs do AzerothCore
docker compose -f /opt/azerothcore-wotlk/docker-compose.yml logs -f --tail=50

# Listar containers
docker ps

# Status dos serviços
curl -s http://127.0.0.1:3000/health-check
curl -s http://127.0.0.1:3000/v1/auth/me

# Itens disponíveis
curl -s 'http://127.0.0.1:3000/v1/items?search=sword'
```

## 11) Swagger UI

Abra no navegador:

```
http://IP_PUBLICO_DA_VPS:3000/swagger-ui/
```
