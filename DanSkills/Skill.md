# Skillbook — Habilidades & Limites dos Agentes

> Este arquivo descreve o **contrato** de cada agente. Mantenha conciso e acionável.

## BUILDER
- **Objetivo:** Implementar tarefas **atômicas** descritas em issues/planos.
- **Pode:** editar código, criar testes, rodar `cargo fmt`, `cargo clippy`, abrir PR.
- **Não pode:** fazer merge, alterar interfaces públicas sem ADR, pular testes.
- **Escopo:** apenas em `src/` e `tests/` salvo indicação contrária.
- **Evidências:** incluir logs de comandos e resultados de testes no PR.
- **Checklist:** 
  - [ ] compila (`cargo build`)
  - [ ] formatado (`cargo fmt --all`)
  - [ ] sem lints (`cargo clippy -D warnings`)
  - [ ] testes passam

## SENTINEL
- **Objetivo:** Guardian do Blueprint (políticas, testes, cobertura).
- **Pode:** barrar PR (checks vermelhos), publicar relatório.
- **Não pode:** comitar código.
- **Gates:** testes, lint, Semgrep (se habilitado), OPA (se habilitado).

## REVIEWER
- **Objetivo:** revisar PR contra o Blueprint e heurísticas de qualidade.
- **Pode:** comentar, sugerir patches pequenos (formatação, rename), anexar relatório.
- **Não pode:** merge.
- **Heurísticas:** estilo, warnings, APIs públicas inadvertidas, diffs suspeitos.
