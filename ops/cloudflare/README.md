# Cloudflare Tunnel — ubl.agency (LAB512)

Objetivo: expor o serviço local (rodando neste computador) via **Cloudflare Tunnel** usando o domínio `ubl.agency`.

Setup recomendado:

- Registry (HTTP): `https://registry.ubl.agency` → `http://127.0.0.1:8791`
- Nightly artifacts (opcional): `https://nightly.ubl.agency` → diretório local (via app separado / nginx / caddy)

## Pré‑requisitos

- `cloudflared` instalado (neste host já existe em geral em `/opt/homebrew/bin/cloudflared`).
- Acesso ao painel Cloudflare do zone `ubl.agency`.

## Passo a passo (uma vez)

1) Login (gera cert local):

```bash
cloudflared tunnel login
```

2) Criar tunnel:

```bash
cloudflared tunnel create ai-nrf1
```

3) Criar DNS route:

```bash
cloudflared tunnel route dns ai-nrf1 registry.ubl.agency
```

4) Criar config local (NÃO commitar):

```bash
cp ops/cloudflare/cloudflared.config.example.yml ops/cloudflare/cloudflared.config.yml
```

Edite `ops/cloudflare/cloudflared.config.yml` para apontar para a porta certa (default `8791`) e para o arquivo de credenciais do tunnel.

## Rodar com o PM2 “limpo” do projeto

O PM2 isolado do ai‑nrf1 usa `PM2_HOME=$HOME/.pm2-ai-nrf1`.

Subir registry + tunnel:

```bash
make pm2-ai-start
make pm2-ai-tunnel-start
```

Logs:

```bash
make pm2-ai-logs
make pm2-ai-tunnel-logs
```

## Notes

- `ops/cloudflare/cloudflared.config.yml` fica gitignored (tem caminho do credentials).
- O hostname `registry.ubl.agency` é a convenção inicial; se quiser mudar, ajuste o DNS route e o `CDN_BASE` em `ops/pm2/local.env`.

