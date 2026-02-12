
# Divisão de Módulos (Renomeações)

> **Terminologia oficial (estável):** **ai-nrf1** (bytes) e **ai-json-nrf1** (view).
> **Aliases de marca (equivalentes):** **ubl-byte** (bytes) e **ubl-json** (view).

- **ubl-json** → Estrutura de pensamento do LLM (slots) e ponte para NRF‑1.1
- **ubl-transport** → SIRP‑like capsules para transporte/assinatura (BASE mínimo)
- **ubl-policy** → Fachada do avaliador TDLN/Chip‑as‑Code
- **envelope** → Criptografia de tuplas (X25519 + XChaChaPoly) com AAD=cid
- **ai-nrf1-core** → Canonical binary (NRF‑1.1)

`Chip as Code`: permanece como documento de integração e demos multi‑backend. Nesta fase BASE, fica acoplado via `ubl-policy`.