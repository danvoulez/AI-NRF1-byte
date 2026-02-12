# Pacotão INTEGRADO 🚀 (com hardening + execução real/stub + idempotência de recibos)

Este pacote entrega:
- **Comandos `ubl`**: `tdln {policy,runtime}` e `llm {engine,smart}`
- **Hardening**: schema, whitelist, IO seguro
- **Execução real (feature `runner-real`)** via `module-runner` **ou** stub determinístico
- **`cap-runtime` hardened**
- **Helper de idempotência** para `AppendReceipt`

## Como aplicar

1. Copie **tudo** para as mesmas localizações no repo.
2. Aplique patches:
   - `patches/tools_ubl_cli_Cargo_toml.patch`
   - `patches/tools_ubl_cli_main_rs.patch`
3. Build (stub por padrão):
```bash
cargo build -p ubl-cli -p cap-runtime -p receipt-idem
```
4. Para usar o **runner real**, habilite a feature:
```bash
cargo build -p ubl-cli --features runner-real
```
> No `tools/ubl-cli/src/execute.rs`, ajuste os `use module_runner_inprocess::{...}` para o namespace real do seu `module-runner` se o nome/ninho diferir. A função `run_manifest()` já retorna o shape esperado (`receipt_cid`/`url_rica`/artifacts/metrics).

## Comandos

```bash
# 1) tdln com políticas
ubl tdln policy run --manifest manifests/tdln/policy.yaml --out -

# 2) tdln com Certified Runtime
ubl tdln runtime run --manifest manifests/tdln/runtime.yaml --out -

# 3) LLM Engine (premium, com pré-processamento tdln)
ubl llm engine run --out -

# 4) LLM Smart (local 8B, com pré-processamento tdln)
ubl llm smart run --out -
```

## Idempotência dos recibos
Use `receipt-idem::idempotency_key(tenant, trace_id, plan_cid)` na hora de emitir `AppendReceipt`
no executor de effects (ou gateway de recibos). Essa chave **previne duplicidade** em replays.

## Notas
- Por segurança, manifests > 256KB ou com paths inseguros são **rejeitados**.
- `use` fora da whitelist → **rejeitado**.
- `tdln.*`/`llm.*` exigem `outputs.fields` com `receipt_cid` e `url_rica`.
- O stub gera `receipt_cid` determinístico (`blake3`) e uma URL local — ótimo para DX/CI.
- Troque o módulo `module_runner_inprocess` pelos tipos reais do seu `module-runner`.

Qualquer ajuste fino na integração do runner real, me chama que eu adapto na hora ✨
