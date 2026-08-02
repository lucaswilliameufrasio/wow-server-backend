# VPS — Instalação limpa

## Requisitos

- VPS Linux Ubuntu 24.04, 4 vCPU, 8 GB RAM, 30 GB SSD
- Portas liberadas no firewall do provedor: `3724/tcp`, `8085/tcp`
- Domínio (opcional, para HTTPS via Caddy)
- Acesso SSH

## 1. Provisionar VPS

```bash
apt update && apt upgrade -y
apt install -y git curl openssl ufw
curl -fsSL https://get.docker.com | sh
```

## 2. Clonar repositórios

```bash
# Backend da API
git clone <URL_DO_REPO> /opt/wow-backend

# AzerothCore
git clone --depth 1 --branch master \
  https://github.com/azerothcore/azerothcore-wotlk.git /opt/azerothcore-wotlk
```

## 3. Configurar

```bash
cd /opt/wow-backend/deploy/vps

# Criar .env e editar senhas
cp .env.example .env
nano .env
```

No `.env`, gere senhas fortes:

```bash
# Gerar senhas de 32 caracteres
openssl rand -base64 24   # para WOW_PG_PASSWORD
openssl rand -base64 24   # para WOW_MYSQL_PASSWORD
```

Variáveis mínimas obrigatórias:

| Variável | Descrição |
|---|---|
| `WOW_PG_PASSWORD` | Senha do PostgreSQL da API |
| `WOW_MYSQL_PASSWORD` | Senha root do MySQL do AzerothCore |

> **Docker rootful x rootless** — o AzerothCore é compilado a partir do source no
> primeiro `up`, e isso falha em daemons Docker rootless antigos (20.10.x). Use o
> daemon **rootful** (moderno). Se o `docker` do seu usuário apontar para um
> daemon rootless, rode os comandos com `sudo ./wowctl ...`. O primeiro `up`
> demora bastante (compila authserver/worldserver + backend Rust, ~30-60min).

## 4. Instalar

```bash
export WOW_ACORE_DIR=/opt/azerothcore-wotlk
./wowctl install   # use: sudo ./wowctl install  se o docker do usuário for rootless
```

## 5. Subir serviços

```bash
./wowctl up        # use: sudo ./wowctl up  se o docker do usuário for rootless
```

O script:
1. Sobe MySQL e aguarda healthcheck
2. Sobe db-import (cria os bancos do AC)
3. Sobe authserver e worldserver
4. Sobe PostgreSQL e API

## 6. Configurar realm

```bash
# Se tiver IP público fixo
./wowctl set-realm SEU_IP_PUBLICO

# Se tiver domínio
./wowctl set-realm wow.seudominio.com
```

## 7. Verificar

```bash
./wowctl smoke-test
```

## 8. Configurar firewall

```bash
ufw allow OpenSSH

# Público
ufw allow 3724/tcp comment 'WoW auth'
ufw allow 8085/tcp comment 'WoW world'

# Tailscale (opcional)
ufw allow in on tailscale0 comment 'Tailscale'

ufw enable
```

## 9. Criar contas de administrador

```bash
./wowctl create-account Admin MinhaSenha123 3
```

## 10. Cliente WoW

Edite `realmlist.wtf` no diretório do WoW 3.3.5a:

```text
set realmlist SEU_IP_PUBLICO
```

---

## Perfil de segurança (API privada)

A API escuta apenas em `127.0.0.1:3000` (não publicada na internet).

- Se usar **Tailscale** ou **NetBird**: faça SSH para a VPS e use `curl 127.0.0.1:3000`
- Se usar **Caddy com domínio**: veja configuração em [config.md](config.md)

## Próximos passos

- [Operação diária](operations.md)
- [Backup e restauração](backup.md)
- [Configuração avançada](config.md)
