# VPS — Operação diária

## Comandos rápidos

```bash
cd /opt/wow-backend/deploy/vps

./wowctl status          # Status de todos os serviços
./wowctl logs            # Logs em tempo real
./wowctl logs ac-worldserver   # Logs de serviço específico
```

## Start/Stop

```bash
./wowctl up              # Sobe tudo (AC primeiro, depois backend)
./wowctl up --be-only    # Só backend (se AC já estiver rodando)
./wowctl down            # Para tudo
./wowctl down --ac-only  # Só AzerothCore
```

## Atualizar backend

```bash
# Via GHCR (recomendado)
WOW_BACKEND_TAG=v1.2.3 ./wowctl update v1.2.3

# Build local
./wowctl build
./wowctl update dev
```

## Rollback

```bash
# Para tag anterior
./wowctl rollback

# Para tag específica
./wowctl rollback v1.2.2
```

## Criar contas

```bash
./wowctl create-account <username> <password> [gmlevel]
```

Exemplo:

```bash
./wowctl create-account GuildMaster SenhaForte123 3
```

## Setup completo (uma vez)

Depois de subir os serviços, `setup-game` prepara tudo em um comando: conta GM,
realm, seeds (itens, vendors de raid, vendors BiS, FULLTEXT) e smoke-test:

```bash
./wowctl setup-game Admin MinhaSenha123 SEU_IP_PUBLICO
```

Seeds individuais podem ser pulados: `SKIP_SEED_ITEMS=true`, `SKIP_SEED_FAST_RAID=true`, `SKIP_SEED_SPEC_BIS=true`.

## Recarregar configs do worldserver (SOAP)

Após seeds ou mudanças de config, sem reiniciar o worldserver:

```bash
./wowctl reload                # .reload config + creature/creature_template/npc_vendor/item_template
./wowctl reload creature_template   # apenas um alvo
```

Exige SOAP habilitado (o `./wowctl install` já habilita; reinicie o worldserver
uma vez em instalações antigas).

## Atualizar realm

```bash
./wowctl set-realm novo-ip-ou-dominio
```

Além do banco, grava `deploy/vps/realmlist.wtf` — copie para o cliente WoW.

## Logs de diagnóstico

```bash
# AzerothCore
docker logs ac-worldserver --tail 200

# API
docker logs wow-backend-vps-wow-backend-1 --tail 200

# MySQL
docker exec ac-database mysql -uroot -p"$WOW_MYSQL_PASSWORD" acore_auth \
  -e "SELECT id, name, address, port FROM realmlist"

# Verificar jogadores online
docker exec ac-database mysql -uroot -p"$WOW_MYSQL_PASSWORD" acore_characters \
  -e "SELECT COUNT(*) FROM characters WHERE online=1"
```

## Healthcheck (API)

```bash
curl -s http://127.0.0.1:3000/health-check
# {"message":"ok"}

# Métricas Prometheus (porta dedicada e privada, sem publicação no compose)
docker compose -p wow-backend-vps exec wow-backend \
  wget -qO- http://127.0.0.1:9090/metrics | head

# Ou, com SSH tunnel:
ssh -L 9090:127.0.0.1:9090 user@vps
curl -s http://127.0.0.1:9090/metrics
```

## Swagger UI

Acessível apenas localmente. Use SSH tunnel se necessário:

```bash
ssh -L 3000:127.0.0.1:3000 user@vps
# Abra http://127.0.0.1:3000/swagger-ui/
```
