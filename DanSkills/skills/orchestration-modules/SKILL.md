---
name: "orchestration-modules"
description: "Padrões para orquestrar mudanças em múltiplos módulos/serviços com segurança (monorepo ou multi-repo)."
when_to_use:
  - "Sweeps de código, refactors, upgrades de dependências."
  - "Mudanças em contratos internos (APIs, schemas, eventos)."
inputs:
  - "Manifest de módulos (service catalog/Backstage, matrix do CI)."
  - "Blueprint/policies (tests, Semgrep/OPA)."
  - "Plano de rollback e limites (rate, canário)."
references:
  - "Service Catalog (ex.: Backstage), matrices de CI, semáforos e gates determinísticos."
---

## Objetivo
Aplicar mudanças coordenadas em **N módulos** minimizando risco, com **canários**, **rate‑limits**, **contratos explícitos** e **trilha de auditoria**.

## Princípios
- **Declarativo**: manifest com escopo/paths/owners/labels.
- **Contratos primeiro**: testes de contrato e checagens estáticas antes de mexer em código.
- **Progressivo**: canário → lote pequeno → expansão.
- **Reprodutível**: idempotente, logs consistentes, rollback claro.
- **Isolado**: cada módulo independente; nunca fazer “big‑bang merge”.

## Checklist
- [ ] Manifest atualizado (owners, paths, tipo do módulo, dependências).
- [ ] Matriz do CI gerada (parallelismo, `max-parallel`, `concurrency`).
- [ ] Gates verdes (tests, lint, Semgrep/OPA) por módulo.
- [ ] Plano de **canário** (5–10%), **rate‑limit** (N PRs/h) e **rollback**.
- [ ] Labels e métricas: `sweep`, `dry-run`, `attempts`, tempo de ciclo.
- [ ] Release notes internas por módulo (mudanças de contrato).

## Procedimento
1) **Descobrir** alvos via service catalog/manifest. Validar que os módulos existem e têm owners.
2) **Simular** (dry‑run) e gerar diffs por módulo. Revisar em 3–5 canários.
3) **Executar** por lotes limitados (ex.: 10‑20 em paralelo). Pausar se taxa de falha > X%.
4) **Validar** gates por módulo (tests/OPA/Semgrep). Reprovar módulos com falhas.
5) **Consolidar** relatório (sucesso/falha), owners notificados, PRs com labels.
6) **Expandir** gradualmente até 100% ou **rollback** seletivo.

## Saídas
- PRs por módulo com diffs claros, labels e checks.
- Relatório final por módulo (sucesso/falha, tempo, follow‑ups).
