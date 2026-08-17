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

## 3. Instalar

O `./wowctl install` cria o `.env` e **gera automaticamente as senhas fortes**
de PostgreSQL, MySQL e SOAP (sem edição manual). Também gera as chaves JWT,
configura o env do AzerothCore e habilita SOAP no worldserver.

```bash
cd /opt/wow-backend/deploy/vps
export WOW_ACORE_DIR=/opt/azerothcore-wotlk
./wowctl install   # use: sudo ./wowctl install  se o docker do usuário for rootless
```

> **Docker rootful x rootless** — o AzerothCore é compilado a partir do source no
> primeiro `up`, e isso falha em daemons Docker rootless antigos (20.10.x). Use o
> daemon **rootful** (moderno). Se o `docker` do seu usuário apontar para um
> daemon rootless, rode os comandos com `sudo ./wowctl ...`. O primeiro `up`
> demora bastante (compila authserver/worldserver + backend Rust, ~30-60min).

> Os segredos ficam em `deploy/vps/.env` (e `$ACORE_DIR/.env`, `.secrets/`).
> Guarde-os ou rode `./wowctl backup` depois de subir os serviços.

## 4. Subir serviços

```bash
./wowctl up        # use: sudo ./wowctl up  se o docker do usuário for rootless
```

O script:
1. Sobe MySQL e aguarda healthcheck
2. Sobe db-import (cria os bancos do AC)
3. Sobe authserver e worldserver
4. Sobe PostgreSQL e API

## 5. Setup completo (um comando)

Cria a conta GM, configura o realm, roda os seeds (itens, vendors de raid,
vendors BiS, índice FULLTEXT) e valida tudo:

```bash
./wowctl setup-game Admin MinhaSenha123 SEU_IP_PUBLICO
```

Para pular seeds individuais: `SKIP_SEED_ITEMS=true SKIP_SEED_FAST_RAID=true SKIP_SEED_SPEC_BIS=true ./wowctl setup-game`.

## 6. Configurar firewall

```bash
ufw allow OpenSSH

# Público
ufw allow 3724/tcp comment 'WoW auth'
ufw allow 8085/tcp comment 'WoW world'

# Tailscale (opcional)
ufw allow in on tailscale0 comment 'Tailscale'

ufw enable
```

## 7. Cliente WoW

`./wowctl set-realm` grava o `realmlist.wtf` em `deploy/vps/realmlist.wtf`
(pode ser chamado a qualquer momento para atualizar o realm):

```bash
./wowctl set-realm SEU_IP_PUBLICO
```

Copie `deploy/vps/realmlist.wtf` para o diretório do WoW 3.3.5a:

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
