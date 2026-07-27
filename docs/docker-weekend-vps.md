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

## 6) Mapa de portas

| Porta | Serviço | Público? | Observação |
|---|---|---|---|
| `22/tcp` | SSH | Restrito | Só libere para seu IP ou VPN |
| `3724/tcp` | Authserver WoW | Opcional | Necessário para clientes se conectarem |
| `8085/tcp` | Worldserver WoW | Opcional | Necessário para clientes se conectarem |
| `3000/tcp` | Backend API | Opcional | Swagger UI e chamadas REST |
| `7878/tcp` | SOAP worldserver | **NÃO** | Apenas rede interna |
| `3306/tcp` | MySQL AzerothCore | **NÃO** | Apenas rede interna |
| `5432/tcp` | PostgreSQL API | **NÃO** | Apenas rede interna |

Portas Docker que não devem ser expostas: verifique se `docker-compose.prod.yml` ou `docker-compose.dev.yml` publicam `3306`, `5432` ou `7878`. Remova esses `ports:` ou deixe apenas `127.0.0.1:`.

## 7) Liberar acesso à rede

### 7.1 Descobrir interfaces e IPs

```bash
# Ver interfaces de rede disponíveis
ip link show | grep -E '^[0-9]'

# Descobrir CIDR da LAN (ex.: 192.168.1.0/24)
ip -4 addr show | grep inet

# Verificar Tailscale (interface: tailscale0)
tailscale ip --4

# Verificar NetBird (interface: wt0)
nmctl status 2>/dev/null || echo "NetBird pode nao estar instalado"
```

### 7.2 Regras UFW base (obrigatórias)

Toda regra deve ter `comment` para identificação.

```bash
ufw allow OpenSSH comment 'SSH acesso administrativo'
ufw enable
```

### 7.3 Acesso público (amigos pela internet)

```bash
ufw allow 3724/tcp comment 'WoW authserver publico'
ufw allow 8085/tcp comment 'WoW worldserver publico'
ufw allow 3000/tcp comment 'WoW backend API publico'
```

A API em `3000` expõe cadastro e login sem autenticação. Se possível, restrinja a LAN/VPN.

### 7.4 Acesso LAN

```bash
# Substitua 192.168.1.0/24 pelo CIDR da sua rede local
ufw allow from 192.168.1.0/24 to any port 3724 proto tcp comment 'WoW LAN authserver'
ufw allow from 192.168.1.0/24 to any port 8085 proto tcp comment 'WoW LAN worldserver'
ufw allow from 192.168.1.0/24 to any port 3000 proto tcp comment 'WoW LAN backend API'
```

### 7.5 Acesso via Tailscale

```bash
ufw allow in on tailscale0 to any port 3724 proto tcp comment 'WoW Tailscale authserver'
ufw allow in on tailscale0 to any port 8085 proto tcp comment 'WoW Tailscale worldserver'
ufw allow in on tailscale0 to any port 3000 proto tcp comment 'WoW Tailscale backend API'
```

### 7.6 Acesso via NetBird

A interface do NetBird costuma ser `wt0`.

```bash
ufw allow in on wt0 to any port 3724 proto tcp comment 'WoW NetBird authserver'
ufw allow in on wt0 to any port 8085 proto tcp comment 'WoW NetBird worldserver'
ufw allow in on wt0 to any port 3000 proto tcp comment 'WoW NetBird backend API'
```

### 7.7 Resumo por perfil

| Perfil | SSH | Auth 3724 | World 8085 | API 3000 | DBs 3306/5432 |
|---|---|---|---|---|---|
| Público irrestrito | ❌ | ✅ | ✅ | ✅ (não recomendado) | ❌ |
| Só LAN + VPN | ✅ LAN/VPN | ✅ LAN/VPN | ✅ LAN/VPN | ✅ LAN/VPN | ❌ |
| Só Tailscale | ✅ Tailscale | ✅ Tailscale | ✅ Tailscale | ✅ Tailscale | ❌ |
| Só NetBird | ✅ NetBird | ✅ NetBird | ✅ NetBird | ✅ NetBird | ❌ |

### 7.8 ⚠️ Ressalva: Docker pode contornar o UFW

O Docker publica portas diretamente nas regras `iptables` do kernel, que podem estar acima das regras do UFW. Para garantir que MySQL, PostgreSQL e SOAP **não** fiquem acessíveis externamente, mesmo que o Compose os publique:

```bash
# Bloquear acesso externo as portas internas via DOCKER-USER
iptables -I DOCKER-USER -p tcp --dport 3306 -j DROP
iptables -I DOCKER-USER -p tcp --dport 5432 -j DROP
iptables -I DOCKER-USER -p tcp --dport 7878 -j DROP
```

Para persistir após reboot, instale `iptables-persistent`:

```bash
apt install -y iptables-persistent
netfilter-persistent save
```

Verifique:

```bash
ufw status verbose
iptables -L DOCKER-USER -n --line-numbers
```

## 8) Criar contas WoW

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

## 9) Configurar clientes WoW

Edite `realmlist.wtf` no diretório de instalação do WoW 3.3.5a.

**Público:**

```text
set realmlist IP_PUBLICO_DA_VPS
```

**LAN:**

```text
set realmlist 192.168.1.100   # IP da VPS na rede local
```

**Tailscale:**

```text
set realmlist 100.x.x.x        # IP Tailscale da VPS
```

**NetBird:**

```text
set realmlist 10.x.x.x         # IP NetBird da VPS
```

**Mesma máquina (localhost):**

```text
set realmlist 127.0.0.1
```

## 11) Credenciais padrão

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

### Contas WoW de teste

| Username | Password | GM level | Criada por |
|---|---|---|---|
| `NEWTEST` | `Test12345` | 3 (admin) | Setup automático |
| `PLAYER2` | `Player12345` | 0 (jogador) | Setup automático |

Para criar novas contas: `POST /v1/auth/register`

Para elevar uma conta a GM:

```bash
docker exec ac-database mysql -uroot -ppassword acore_auth \
  -e "INSERT INTO account_access (id, gmlevel, RealmID) VALUES (ID_DA_CONTA, 3, -1)"
```

## 12) Logs e diagnóstico

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

## 13) Swagger UI

Abra no navegador:

```
http://IP_PUBLICO_DA_VPS:3000/swagger-ui/
```
