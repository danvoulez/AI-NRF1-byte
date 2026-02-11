# DanSkills — pacote completo

Data do pacote: 2026-02-10

Este zip reúne:
- **Semáforo de 3 agentes** (Builder, Sentinel, Reviewer), 100% configurável por **ENV**.
- **Skills prontas**: UI forte, Code‑Review UI, UI Rust/WASM, Orquestração de Módulos e Multiprodutos.
- **Scripts** de Reviewer (UI e Rust), Sentinel (tests/semgrep/OPA), e Bootstrap de ferramentas.
- **Workflows GitHub Actions** com concurrency e comentários automáticos no PR.

## Início rápido
```bash
unzip DanSkills.zip -d ./
cd DanSkills

cp .env.example .env
# Escolha o reviewer (veja opções no final do .env.example)
# Ex.: UI + Rust numa passada:
# REVIEWER_CMD=bash -lc 'scripts/review_ui.sh | tee review_report.md && scripts/review_rust.sh | tee -a review_report.md'

# (Opcional) Bootstrap local (pytest/semgrep/opa + cargo-llvm-cov)
bash scripts/bootstrap_tools.sh

chmod +x traffic/trafficctl.sh traffic/lib.sh agents/*.sh scripts/*.sh
./traffic/trafficctl.sh init
./agents/builder.sh    # terminal 1
./agents/sentinel.sh   # terminal 2
./agents/reviewer.sh   # terminal 3
```

## Onde editar
- **.env** — ordem dos agentes, comandos do Builder, gates do Sentinel, URLs de auditoria de UI, etc.
- **skills/** — SKILL.md por tema. Respeite o nome `SKILL.md` (caixa alta).
- **rules/semgrep** e **policies/opa** — regras e políticas do seu Blueprint.
- **scripts/** — miolos de review/testes; plugue linters/ferramentas do seu stack.

## Dicas
- Use labels no PR como protocolo entre agentes.
- Rode canário (5–10%) antes de expandir sweep/orquestração.
- Se crescer para dezenas/centenas em paralelo recorrentes, considere uma camada de orquestração em nuvem.
