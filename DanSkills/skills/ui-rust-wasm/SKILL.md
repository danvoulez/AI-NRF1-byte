---
name: "ui-rust-wasm"
description: "Boas práticas para UIs em Rust (Yew/Leptos/Dioxus): reatividade eficiente, alocação, bridges JS e medição."
when_to_use:
  - "PRs que toquem UI com Rust/WASM ou interop JS."
references:
  - "Guias de performance com Yew/Leptos; perf checklists web"
---

## Objetivo
Evitar quedas de performance por **alocação excessiva**, **renderização redundante** e **bridges JS caras**; manter UIs reativas e previsíveis.

## Princípios
- **Reatividade enxuta**: prefira signals/stores granulares; evite passar estados grandes por props.
- **Alocação**: minimize cópias; use slices/refs; medir com `wasm-bindgen` e `wee_alloc` quando fizer sentido.
- **Interop JS**: agregue mensagens; evite chamadas síncronas frequentes; serialize de forma barata.
- **Rendering**: diffing eficiente; chaves estáveis em listas; evite re-render global.
- **Async**: tarefas canceláveis; sem `spawn` órfã; backpressure em fetches.

## Procedimento
1) Perf local: medir tempo de interação; observar INP e execução em handlers.
2) Auditar bridges JS↔︎WASM (frequência, payload).
3) Verificar memória/alocação em hot paths; reduzir cópias/clone().
4) Validar responsividade/layout (ver Skill de UI forte).

## Saídas
- Notas de performance/interop, patches pequenos (split de estado, memoização, chaves).
