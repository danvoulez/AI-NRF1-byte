---
name: "ui-strong-web"
description: "Pílula de UI forte: UX clara, acessibilidade (WCAG 2.2), Core Web Vitals, responsividade e Design System."
when_to_use:
  - "Qualquer PR que altere UI (web) ou UX."
  - "Antes do merge; após o Builder."
inputs:
  - "Diff do PR e URL de preview (UI_A11Y_URL / LHCI_URL)"
  - "Design tokens/DS e Blueprint"
  - "Este SKILL.md"
references:
  - "Primer A11y checklist; WCAG 2.2 code examples; Perf checklists 2025; Lighthouse CI"
---

## Objetivo
Garantir interfaces **acessíveis**, **performáticas**, **consistentes** com o Design System e com **fluxos claros**.

## Checklist
- [ ] **A11y** (WCAG 2.2): foco visível, navegação via teclado, landmarks, labels/titles, contraste AA.
- [ ] **Vitals**: LCP ≤ 2.5s, INP ≤ 200ms, CLS ≤ 0.1 (em preview local/PR).
- [ ] **Responsivo**: mobile/tablet/desktop com targets de toque ≥ 44×44.
- [ ] **DS**: usa componentes/tokens; evita estilos mágicos fora do tema.
- [ ] **i18n**: sem strings hardcoded; RTL ok; pluralização.
- [ ] **Form UX**: validação inline, loading/disabled; mensagens acionáveis.
- [ ] **Segurança**: evita `innerHTML`/`dangerouslySetInnerHTML` sem sanitização; CSP compatível.
- [ ] **Telemetria**: eventos de view/submit/error nomeados.

## Procedimento
1) Rode `scripts/review_ui.sh` (ESLint/Stylelint/axe/LHCI quando configurados).
2) Verifique Core Web Vitals no preview. Otimize imagens (formatos, `srcset/sizes`), reduza JS e divida por rota.
3) Faça QA de responsividade (3 breakpoints). Garanta foco/ordem de navegação.
4) Documente variações no Storybook (se houver) e vincule ao PR.

## Saídas
- Relatório `review_report.md` com achados (a11y, vitals, DS, responsivo, i18n). Patches pequenos quando viáveis.
