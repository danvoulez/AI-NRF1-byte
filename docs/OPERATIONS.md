# Guia Oficial de Operação — BASE (ai-nrf1 / ubl)

Este documento padroniza **como trabalhar**, **testar**, **versionar** e **publicar binários** da BASE.
O foco é: PRs → CI → release/tag → artefatos (Releases + LAB512/local) → verificação offline (WASM) → supply chain (SBOM + checksums + assinatura).

## 1) Fluxo de branches e PRs

- `main`: deve ficar sempre verde.
- Branches:
  - `feat/<slug>`, `fix/<slug>`, `chore/<slug>`
- Commits: recomendado usar Conventional Commits (ex.: `feat(capsule): ...`, `fix(nrf): ...`).
- Antes de abrir PR:
  ```bash
  git fetch origin
  git rebase origin/main
  ```

### Proteções recomendadas (GitHub UI)

Em **Settings → Branches → Branch protection rules** para `main`:
- Require status checks (CI) to pass before merging
- Require at least 1 approval
- Disallow force pushes

Observação: isso é configuração do repositório (não dá para “commitar” no git).

## 2) O que o CI executa (qualidade)

### 2.1 Rust (formatação, lint, testes)

```bash
cargo fmt --all -- --check
cargo clippy -p nrf-core -p ai-nrf1 -p ubl_json_view -p ubl_capsule --all-targets --all-features -- -D warnings
cargo test --workspace --locked
```

### 2.2 Vetores (KATs) e invariantes (capsule/receipts/expired/tamper)

```bash
make vectors-verify
```

### 2.3 WASM (offline verify smoke via Node)

Requer `wasm-pack` e `node`:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo install wasm-pack
PATH="$HOME/.cargo/bin:$PATH" bash tools/wasm/build_node_pkgs.sh
node tests/wasm/node_smoke.cjs
```

### 2.4 Diferencial Python ↔ Rust

```bash
python3 -m pip install -r tests/differential/requirements.txt
cargo build -p nrf1-cli
PYTHONPATH=impl/python/nrf_core_ref PATH=target/debug:$PATH \
  python3 -m pytest -q tests/differential/test_diff_cli_vs_python.py
```

## 3) Releases oficiais (tag → binários + checksums + SBOM + assinatura)

### 3.1 Versionamento e tag

- SemVer: `MAJOR.MINOR.PATCH` (ex.: `2.0.0`)
- Criar tag e publicar:
  ```bash
  git tag v2.0.0
  git push origin v2.0.0
  ```

### 3.2 Artefatos do release

O workflow `.github/workflows/release.yml` gera e publica, no GitHub Releases:
- Binários (`ai-nrf1`, `ubl`, `nrf1`) por OS
- `CHECKSUMS.sha256` + `CHECKSUMS.sha512`
- SBOMs em `dist/sbom/*` (best-effort)
- Assinaturas `cosign sign-blob` (keyless) para cada arquivo em `dist/`

## 4) Distribuição (LAB512 = este computador)

### 4.1 Nightly local (self-hosted runner)

O workflow `.github/workflows/nightly-lab512.yml` (opcional) roda em runner **self-hosted** e publica em disco local.

- Diretório padrão: `/opt/lab512/artifacts/nightly/<sha>/`
- Link “latest”: `/opt/lab512/artifacts/nightly/latest`

Para habilitar, registre este computador como **GitHub Actions self-hosted runner** e adicione a label `lab512`.
Depois, o workflow pode rodar via `workflow_dispatch` ou cron.

### 4.2 Exposição via Cloudflare Tunnel (opcional)

Isso é infra local (fora do repositório), mas o objetivo é publicar `.../latest/` atrás de Cloudflare Access.

## 5) Segurança e supply chain

- Checksums: `scripts/make_checksums.sh`
- SBOM: `scripts/sbom.sh` (usa `syft` quando disponível)
- Assinatura: `cosign` keyless (OIDC) no workflow de release
- Segredos: nunca commitar chaves privadas; vetores usam `tests/keys/` (gitignored)

## 6) Dia a dia (TL;DR)

1. `git checkout -b feat/<slug>`
2. Rodar checks locais (seção 2)
3. Commit + push + PR
4. CI verde + review
5. Merge
6. Release: `git tag vX.Y.Z && git push origin vX.Y.Z`

