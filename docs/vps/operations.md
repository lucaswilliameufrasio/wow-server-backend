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

## Atualizar realm

```bash
./wowctl set-realm novo-ip-ou-dominio
```

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

curl -s http://127.0.0.1:3000/metrics
# Métricas Prometheus
```

## Swagger UI

Acessível apenas localmente. Use SSH tunnel se necessário:

```bash
ssh -L 3000:127.0.0.1:3000 user@vps
# Abra http://127.0.0.1:3000/swagger-ui/
```
