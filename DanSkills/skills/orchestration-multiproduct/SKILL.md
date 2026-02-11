---
name: "orchestration-multiproduct"
description: "Playbook para sincronizar releases/lançamentos em múltiplos produtos/linhas, alinhando contratos, datas e riscos."
when_to_use:
  - "Release train multi‑produto, bundles e features cross‑produto."
  - "Deprecações/renames que impactam ecossistema."
inputs:
  - "Calendário de releases, owners, níveis de criticidade."
  - "Matriz de dependências (técnicas e comerciais)."
  - "Planos de comunicação e rollback."
references:
  - "Release train, feature flags, progressive delivery, service catalog."
---

## Objetivo
Entregar mudanças **coordenadas** em vários produtos com previsibilidade, **sem downtime**, e com comunicação clara para clientes e times.

## Princípios
- **Train > Big‑bang**: janelas fixas de release; feature flags para desacoplar.
- **Compatibilidade**: versionamento/contratos com períodos de *grace* e migração.
- **Observabilidade**: SLIs/alertas definidos por produto.
- **Comms**: mensagens por persona (clientes, suporte, vendas, parceiros).

## Checklist
- [ ] **Calendário** de janelas + critérios de “go/no‑go”.
- [ ] **Flags** e *kill switches* definidos; migração de dados ensaiada.
- [ ] **Backwards compatibility** ou plano de *breaking change* (com datas).
- [ ] **Runbooks** por produto (ativação/rollback) e responsáveis de plantão.
- [ ] **Comunicados** (interno/externo) prontos e aprovados.
- [ ] **Métricas** de sucesso (adoção, erro, latência); dashboards preparados.

## Procedimento
1) **Planejar** release train (D‑30 a D‑0): escopo, riscos, owners, validação legal/comercial se necessário.
2) **Ensaiar** em ambientes e *slices*: canários por produto/segmento.
3) **Ativar** por ondas (região, plano, cliente piloto). Monitorar SLIs/alertas.
4) **Comunicar** marcos (pré/atualização/pós) com *templates*.
5) **Fechar** com relatório de impacto, *post‑mortem* se houve incidentes, e backlog de follow‑ups.

## Saídas
- Changelog e notas por produto/segmento.
- Timeline do train, incidentes e aprendizados.
