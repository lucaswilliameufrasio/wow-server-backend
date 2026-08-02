# VPS — Troubleshooting

Problemas e correções encontrados durante o bring-up real em ARM (Oracle Always Free)
com AzerothCore master. Leia antes de mexer em produção.

---

## 1. Login falha com "invalid password" (SRP6) — o mais importante

**Sintoma**

O cliente conecta no realm, mas o login falha. No log do authserver:

```
[AuthChallenge] account OLOCO tried to login with invalid password!
```

**Causa raiz**

A criação de conta pela API gera o *verifier* SRP6. O backend Rust tem um toggle
`WOW_SRP6_CORE5_MODE`. Quando `true`, ele gera o verifier no formato **"core 5"**
(salt invertido + verifier invertido), que **não é o formato que o AzerothCore
master espera**. O resultado é um verifier gravado no banco que o authserver
recalcula diferente → sempre "invalid password".

O AzerothCore master usa o SRP6 padrão:

```
H1 = SHA1("USUARIO:SENHA")            # ambos em UPPERCASE
x  = little-endian(SHA1(salt || H1))  # salt na ordem normal
v  = little-endian(g^x mod N)          # N e g fixos do WoW
```

O modo core 5 é usado por outras variantes/forks de core WoW. **Para AzerothCore
master o valor correto é `false`.**

**Correção**

```bash
cd /opt/wow-backend/deploy/vps
sed -i 's/^WOW_SRP6_CORE5_MODE=.*/WOW_SRP6_CORE5_MODE=false/' .env
docker compose -p wow-backend-vps up -d --force-recreate wow-backend

# Apague e recrie a conta (o verifier antigo ficou no formato errado):
./wowctl create-account <usuario> <senha>
```

**Verificação (opcional)**

Compare o `verifier` gravado no banco com o esperado pelo AC:

```bash
docker exec ac-database mysql -uroot -p"$WOW_MYSQL_PASSWORD" acore_auth \
  -N -e "SELECT username,HEX(salt),HEX(verifier) FROM account WHERE username='X'"
```

Calcule o esperado com o algoritmo acima (ex.: em Python) e confira se é igual.
Se `WOW_SRP6_CORE5_MODE=true`, o verifier gravado NÃO bate.

> **Por que o toggle existe?** Ele foi adicionado para suportar outros cores que
> usam a convenção de bytes invertidos. No AzerothCore master ele quebra o login
> e deve ficar `false` (padrão atual do compose e do `.env.example`).

---

## 2. app-postgres unhealthy com erro de volume (Postgres 18)

**Sintoma**

```text
PostgreSQL data in: /var/lib/postgresql/data (unused mount/volume)
This is usually the result of upgrading the Docker image...
```

**Causa**

A imagem `postgres:18` mudou o layout: o volume deve ser montado em
`/var/lib/postgresql` (o PG18 coloca os dados em um subdiretório), não em
`/var/lib/postgresql/data`.

**Correção**

O compose já usa `wow_pg_data:/var/lib/postgresql`. Se você tiver uma imagem
antiga/volumes antigos, recrie com o volume limpo:

```bash
cd /opt/wow-backend/deploy/vps
docker compose -p wow-backend-vps down --volumes
docker compose -p wow-backend-vps up -d
```

---

## 3. Backend em crash loop: "missing private key"

**Sintoma**

```text
Error: failed to create jwt config: missing private key: set JWT_PRIVATE_KEY_PEM or JWT_PRIVATE_KEY_PATH
```

São **duas** causas possíveis, em sequência:

**3a. `target` da secret** — docker compose monta a secret em `/run/secrets/jwt_private`
(sem extensão), mas o app lê `/run/secrets/jwt_private.pem`. Correção (já no compose):

```yaml
secrets:
  - source: jwt_private
    target: jwt_private.pem
  - source: jwt_public
    target: jwt_public.pem
```

**3b. Ownership do arquivo** — a imagem roda como `appuser` (uid `10001`), e as
chaves são geradas com `0600` de outro usuário. O appuser não consegue ler.

```bash
chown 10001:10001 /opt/wow-backend/.secrets/jwt_private.pem /opt/wow-backend/.secrets/jwt_public.pem
```

Reaplique após `wowctl install` regenerar as chaves.

---

## 4. Backend: "network wow-acore declared as external, but could not be found"

**Sintoma** ao subir o backend.

**Causa** — a rede criada pelo compose do AzerothCore é
`<COMPOSE_PROJECT>_ac-network` (ex.: `azerothcore-wotlk_ac-network`), não `wow-acore`.

**Correção** — o compose do backend usa `WOW_ACORE_NETWORK` (padrão
`azerothcore-wotlk_ac-network`). Se o projeto do AC tiver outro nome, ajuste:

```bash
# deploy/vps/.env
WOW_ACORE_NETWORK=<project_do_ac>_ac-network
```

---

## 5. `wowctl` subcomandos hifenizados quebrados

**Sintoma**

```text
./wowctl: cmd_set-realm: command not found
```

**Causa** — o dispatch chamava `cmd_$cmd` mantendo o hífen, mas as funções usam
underscore (`cmd_set_realm`). Afetava `set-realm`, `create-account` e `smoke-test`.
Já corrigido: o dispatch traduz `-` → `_`.

`up`, `down` e `status` (sem hífen) sempre funcionaram.

---

## 6. Build do AzerothCore falha no Docker rootless antigo

**Sintoma** — `ac-db-import` (ou outro serviço com `build:`) falha durante
`apt-get update` dentro do Dockerfile com:

```text
Problem executing scripts APT::Update::Post-Invoke 'rm -f /var/cache/apt/archives/*.deb ...'
```

**Causa** — Docker rootless antigo (ex.: server `20.10.x`) com problema de storage
driver. **Correção** — rode o stack do WoW com o Docker rootful (daemon moderno):

```bash
sudo ./wowctl install
sudo ./wowctl up
```

> O servidor ARM roda dois dockers: um rootless (antigo) e um rootful (28.x).
> O stack do WoW deve usar o rootful. O `wowctl` lê o mesmo `.env` nos dois casos.

---

## 7. `ac-db-import`: "cp: cannot create regular file ... Permission denied"

**Sintoma** — o db-import não consegue copiar os `.dist` para
`/azerothcore/env/dist/etc`.

**Causa** — as imagens AC rodam como usuário `acore` (uid `1000` via
`DOCKER_USER_ID`), mas os arquivos do repo pertenciam a outro uid.

**Correção** — alinhe o ownership do checkout do AC com o uid do container:

```bash
chown -R 1000:1000 /opt/azerothcore-wotlk
```

Se o `.env` do AC definir `DOCKER_USER_ID` diferente, use o mesmo valor.

---

## Referência rápida (comandos úteis)

```bash
# Logs
docker logs ac-authserver --tail 50
docker logs ac-worldserver --tail 50
docker logs wow-backend-vps-wow-backend-1 --tail 50

# Realm
docker exec ac-database mysql -uroot -p"$WOW_MYSQL_PASSWORD" acore_auth \
  -e "SELECT id,name,address,port FROM realmlist"

# Atualizar realm para o IP da tailnet
docker exec ac-database mysql -uroot -p"$WOW_MYSQL_PASSWORD" acore_auth \
  -e "UPDATE realmlist SET address='100.64.0.1', localAddress='100.64.0.1' WHERE id=1"

# Verificar verifier (SRP6)
docker exec ac-database mysql -uroot -p"$WOW_MYSQL_PASSWORD" acore_auth \
  -N -e "SELECT username,HEX(salt),HEX(verifier) FROM account"
```
