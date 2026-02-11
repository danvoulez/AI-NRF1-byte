---
name: "code-review-ui"
description: "Checklist de revisão de código para UI (React/TS + genérico), cobrindo arquitetura, efeitos, estado, DX e performance."
when_to_use:
  - "Sempre que revisar PRs de UI/Frontend."
inputs:
  - "Diff do PR, logs de build/lint/test e URL de preview"
references:
  - "Guia de revisão React/TS; Frontend checklists; DS guidelines"
---

## Objetivo
Fazer revisões **consistentes** e **objetivas**, focadas em bugs, DX e performance — não em preferências já cobertas por lint/formatter.

## Itens principais
- **Arquitetura**: componentização adequada; boundaries claros; nada “god component”.
- **Estado/Efeitos**: efeitos idempotentes; dependências corretas; evitar re-renders desnecessários; suspense/SSR quando fizer sentido.
- **Formulários**: controlados; validação e mensagens claras; acessíveis.
- **Deps**: sem sobreposição (p.ex. moment + date-fns); peso e manutenção considerados.
- **Performance**: memos seletivos; splitting por rota; images budget; evitar trabalho síncrono em handlers.
- **A11y**: roles/labels; ordem de foco; atalhos de teclado quando aplicável.
- **DX**: tipagem (TS) clara; nomes semânticos; pastas/layers consistentes.
- **Segurança**: XSS, injection via props/HTML, escapes.

## Procedimento
1) Conferir lints/formatter (ESLint/Prettier/Stylelint) e testes.
2) Rodar o app (ou preview) e validar interações críticas.
3) Revisar o diff com foco em **efeitos**, **estado**, **re-renders** e **inputs**.
4) Sugerir patches pequenos; bloquear apenas críticas (bug, segurança, UX quebrada).

## Saídas
- Comentários objetivos com justificativa técnica e exemplos; rótulo `review:needs-human` quando houver decisão arquitetural.
