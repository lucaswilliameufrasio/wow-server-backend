# VPS Docker Compose — documentação

Deploy oficial do servidor WoW WotLK 3.3.5a em uma única VPS com Docker Compose.

## Guias

- [Instalação limpa](install.md) — De zero a servidor funcional
- [Configuração](config.md) — Rede, portas, variáveis, Caddy
- [Operação diária](operations.md) — Start/stop, logs, contas, atualização
- [Backup e restauração](backup.md) — Dump, compressão, S3 opcional, cron

## Ferramenta principal

Todos os comandos são centralizados no script `wowctl`:

```bash
cd /opt/wow-backend/deploy/vps
./wowctl install    # Primeira instalação
./wowctl up         # Subir serviços
./wowctl down       # Parar serviços
./wowctl status     # Status
./wowctl logs       # Logs
./wowctl backup     # Backup
./wowctl restore    # Restaurar
./wowctl smoke-test # Validar
./wowctl update     # Atualizar backend
./wowctl rollback   # Rollback
```

## Arquitetura

Dois projetos Compose conectados pela rede `wow-acore`:

| Projeto | Diretório | Serviços |
|---|---|---|
| AzerothCore | `/opt/azerothcore-wotlk` | MySQL, authserver, worldserver |
| Backend | `/opt/wow-backend/deploy/vps` | PostgreSQL, API Rust |

## Modelo de segurança

- Authserver (3724) e worldserver (8085) são públicos
- API escuta apenas em `127.0.0.1` — acessível via SSH ou Caddy opcional
- MySQL e SOAP escutam apenas em `127.0.0.1`
- Todas as senhas são geradas na instalação, sem defaults em produção
