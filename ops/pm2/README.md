# PM2 (limpo) — AI‑NRF1

Objetivo: rodar **um PM2 separado** (isolado) só para este projeto, sem misturar com outros pilotos.

## Como funciona (isolamento)

O PM2 suporta múltiplas instâncias via `PM2_HOME`. Aqui usamos:

- `PM2_HOME=$HOME/.pm2-ai-nrf1`

Isso cria um “PM2 limpo” (daemon + logs + state) só para o stack do ai‑nrf1.

## Comandos do dia a dia

Wrapper incluído no repo:

```bash
./tools/pm2/pm2-ai
```

Exemplos:

```bash
./tools/pm2/pm2-ai ls
./tools/pm2/pm2-ai logs ai-nrf1-registry
./tools/pm2/pm2-ai restart ai-nrf1-registry
./tools/pm2/pm2-ai stop ai-nrf1-registry
```

## Subir o stack (registry)

1) Criar env local (não commitar):

```bash
cp ops/pm2/local.env.example ops/pm2/local.env
```

O `ops/pm2/run-registry.sh` faz `source ops/pm2/local.env` (se existir) antes de executar o binário.

2) Se for expor via Cloudflare Tunnel (domínio `ubl.agency`):

- Siga `ops/cloudflare/README.md` para criar o tunnel e gerar `ops/cloudflare/cloudflared.config.yml`.

2) Build release:

```bash
cargo build --release -p registry
```

3) Start via ecosystem:

```bash
./tools/pm2/pm2-ai start ops/pm2/ecosystem.config.cjs
./tools/pm2/pm2-ai save
```

4) Subir o tunnel no mesmo PM2 “limpo” (opcional):

```bash
./tools/pm2/pm2-ai start ops/pm2/ecosystem.tunnel.config.cjs
./tools/pm2/pm2-ai save
```

## Auto-start no boot (macOS)

Gere o comando de startup do PM2 para este `PM2_HOME`:

```bash
PM2_HOME="$HOME/.pm2-ai-nrf1" pm2 startup
```

O PM2 vai imprimir os comandos exatos para o `launchd`. Depois:

```bash
./tools/pm2/pm2-ai save
```

## Observações

- `ops/pm2/local.env` fica gitignored (contém secrets/paths locais).
- Os artefatos do PM2 ficam em `~/.pm2-ai-nrf1/` (fora do repo).
