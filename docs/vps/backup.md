# VPS — Backup e restauração

## Backup manual

```bash
cd /opt/wow-backend/deploy/vps
./wowctl backup
```

O comando:

1. Faz dump de **todos os bancos MySQL** do AzerothCore (`acore_auth`, `acore_characters`, `acore_world`)
2. Faz dump do **PostgreSQL** da API
3. Copia `.env`, JWT keys e configurações do worldserver
4. Calcula checksums SHA-256
5. Compacta em `/srv/wow/backups/YYYYMMDD_HHMMSS.tar.gz`
6. Remove backups mais antigos que 30 dias

## Restauração

```bash
./wowctl restore /srv/wow/backups/20260728_120000.tar.gz
```

Ou:

```bash
./wowctl restore
# (lista backups disponíveis, pede confirmação)
```

## Backup S3 (opcional)

Configure no `.env`:

```bash
WOW_BACKUP_S3_ENDPOINT=https://s3.us-east-1.amazonaws.com
WOW_BACKUP_S3_BUCKET=meu-bucket-wow
WOW_BACKUP_S3_ACCESS_KEY=AKIA...
WOW_BACKUP_S3_SECRET_KEY=...
```

Para enviar backups existentes:

```bash
# Usando aws-cli (instale com apt install awscli)
aws s3 sync /srv/wow/backups/ s3://meu-bucket-wow/backups/
```

## Backup programado (cron)

Adicione ao crontab (`crontab -e`):

```cron
# Backup diário às 04:00
0 4 * * * cd /opt/wow-backend/deploy/vps && ./wowctl backup >> /var/log/wow-backup.log 2>&1
```

## Estrutura do backup

```
YYYYMMDD_HHMMSS.tar.gz
├── mysql_all.sql              # dump MySQL (todos bancos)
├── postgres_all.sql           # dump PostgreSQL
├── env.txt                    # .env da API
├── azerothcore.env.txt        # env vars do AC Compose
├── secrets/                   # JWT keys
│   ├── jwt_private.pem
│   └── jwt_public.pem
├── azerothcore-config/        # Configurações do worldserver
│   └── modules/
│       └── 00-wowctl.conf
└── sha256sums.txt             # Checksums
```

## Notas

- Backups são **descompressão e uso imediato**: `tar xzf` extrai um dump SQL padrão que pode ser restaurado em qualquer MySQL/PostgreSQL.
- O MySQL é dumpado com `--single-transaction` para consistência sem travar o servidor.
- O mundo do WoW continua jogável durante o backup (os dumps são leves o suficiente).
- Para backup completo do VPS (incluindo dados do jogo, como itens criados por jogadores), **todos os bancos estão incluídos** no dump MySQL.
- Logs e dados de client data **não são incluídos** por serem regeneráveis ou de tamanho grande.
