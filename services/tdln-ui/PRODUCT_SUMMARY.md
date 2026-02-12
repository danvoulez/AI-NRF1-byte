# TDLN - Sistema de Prova Criptográfica
## Produto Completo & Pronto para Venda

### Visão Geral
TDLN é uma plataforma SaaS completa para verificação criptográfica, oferecendo provas verificáveis, imutáveis e auditáveis para qualquer decisão de aplicação. Protocolo SIRP em 4 etapas (INTENT→DELIVERY→EXECUTION→RESULT).

---

## 🎨 Design System

### Cores Primárias
- **ACK (Sucesso)**: `#16a34a` - Verde para decisões aceitas
- **ASK (Pendente)**: `#d97706` - Âmbar para evidências faltantes  
- **NACK (Falha)**: `#dc2626` - Vermelho para regras não atendidas
- **Neutrals Premium**: Cinzas refinados para UI corporativa

### Tipografia
- **Sans**: Inter (corpo e interface)
- **Mono**: JetBrains Mono (CIDs, código, dados técnicos)
- **Line-height**: 1.5-1.6 para legibilidade
- **Tamanhos**: 14-16px corpo, 20-32px títulos

### Componentes Visuais
- Radius: 10px (0.625rem) para elementos premium
- Sombras suaves e transições <150ms
- Estados focáveis para acessibilidade WCAG 2.1 AA
- Scrollbar customizada e refinada

---

## 📱 Páginas & Fluxos

### Marketing (Público)

#### Landing Page (`/`)
- Hero com valor principal e CTAs "Começar grátis" + "Ver demo"
- Stats row: 2M+ execuções/mês, 99.97% uptime, <142ms p99
- Logos de empresas (prova social)
- Seção "Como funciona" em 3 etapas: Enviar → Decidir → Provar
- Features principais: Verificável, Bundle offline, Sem custódia
- Trust section: AEAD, multi-região, p99<150ms, SOC 2 + ISO 27001
- Video placeholder 30-45s

#### Pricing (`/pricing`)
- 3 tiers: Free, Team, Enterprise
- Destaque para verificação offline e auditoria sem vendor lock-in
- Trial framing no plano Team
- Comparação de features clara

#### Docs (`/docs`)
- Quickstart, SDKs (Node.js, Python, Rust)
- API REST, Webhooks, Exemplos práticos
- **Nova seção Telemetria**: ativação, sucesso, adoção, saúde, monetização
- **Nova seção Testes UX de Aceite**: 8 cenários críticos (ACK, ASK, NACK, Offline, A11y, RBAC, Billing, Integrations)

#### Changelog/Status (`/changelog`)
- Versões, incidents, regional latency cards

#### Verificador Offline (`/verify/offline`)
- Upload de bundle.zip
- Verificação sem backend (SIRP timeline + proofs)
- Download e visualização completa offline

### App (Autenticado - `/console`)

#### Shell & Navegação
- **Sidebar**: Visão Geral, Execuções, Evidências, Políticas, Integrações, Auditorias, Faturamento, Equipe & RBAC, Configurações, Ajuda
- **Top Bar**: Tenant switcher, busca global (CID, b3:, título), quick actions, notificações, avatar
- Tenant indicator badge com status online

#### Onboarding (`/onboarding`)
- 4 etapas: Criar tenant → Escolher região → Gerar credenciais → Primeiro recibo
- Guided test "Hello Receipt"
- Snippets SDK + Nginx/Envoy
- Link direto para console ao concluir

#### Dashboard (`/console`)
- Cards de métricas: Execuções hoje, % ACK, p99, Últimos CIDs
- Chart semanal (execuções totais + ACK)
- Últimas execuções com preview
- Quick actions (docs, integrações, team)

#### Execuções (`/console/executions`)
- **Filtros**: Estado (ACK/ASK/NACK), período (date range picker), busca avançada (CID, título, origin, integration, b3:)
- **Export CSV**: Download client-side com toast
- Tabela com paginação, empty state com CTA
- Hover states suaves

#### Receipt Detail (`/console/r/[cid]`)
- **State banner** com ícone, título, badge, microcopy precisa
- **3 colunas** (desktop): Timeline SIRP | Proofs | Evidence + Ações
- **Timeline SIRP**: 4 nodes clicáveis com verificação canônica
- **Proofs**: Cards para Capsule INTENT/RESULT + Receipts DELIVERY/EXECUTION
- **Evidence**: Lista com status, AEAD/protegido, retry + fix instructions
- **ASK/PoI**: Items faltantes com "Como resolver"
- **NACK**: Broken rules + deep link para política
- **Share**: QR, Link verificador offline, PDF
- **Trails**: Requester, origin, integration, idempotency-key

#### Políticas (`/console/policies`)
- **Presets por vertical**: FinTech, HealthTech, E-commerce, AI/ML
- Lista de packs com toggle, versão
- Pack detail: regras com **soft/hard fail** indicators
- **Notas de revisão** por pack

#### Integrações (`/console/integrations`)
- **SDKs**: Node.js, Python, Rust com quickstart
- **Gateways**: Nginx/Envoy snippets
- **CI/CD**: GitHub Actions + GitLab CI com exemplos completos
- **SIEM**: Splunk, Elastic, Datadog, AWS CloudWatch, Azure Sentinel, Sumo Logic
- **Webhooks**: Create endpoint, secret, replays, delivery status

#### Evidências (`/console/evidence`)
- Busca por CID/URL
- **Resoluções & Mirrors**: Lista de mirrors por evidence com status, latency, pin/preferir
- Collapsible detail por item

#### Auditorias (`/console/audits`)
- Log imutável: user, action, target, timestamp, IP, diff(JSON)
- Filtros + export CSV
- Include CID quando aplicável

#### Faturamento (`/console/billing`)
- Usage/quota/projection cards
- Invoices history, payment method
- Cancel/upgrade flows
- In-product upsell (soft, não bloqueia verificação)

#### Equipe & RBAC (`/console/team`)
- Roles: Owner/Admin/Operator/Auditor
- Invites por email
- Enterprise: SSO/SAML, SCIM, enforce MFA

#### Configurações (`/console/settings`)
- API keys, webhooks, regiões, retenção
- Data export
- Key rotation

#### Ajuda (`/console/help`)
- In-app guides, docs links, support tickets

---

## 🧩 UI Kit (Componentes Reutilizáveis)

### `BadgeEstado`
- Estados: ACK/ASK/NACK com cores + ícones
- Acessível: aria-label, não depende só de cor
- Tamanhos: sm, default, lg

### `CIDChip`
- Monospace, truncate inteligente
- Copy to clipboard built-in

### `TimelineSIRP`
- 4 nodes: INTENT → DELIVERY → EXECUTION → RESULT
- Node detail panel, verificação visual ✅

### `CardProva`
- Header (type/algorithm)
- Body (CID, signer, timestamp)
- Actions (copy/download)

### `PoIList` (ASK)
- Items acionáveis com "Como resolver"
- Checklist interativa

### Ações
- DownloadBundle (.zip)
- QRLink
- CopyToClipboard com feedback

---

## ♿ Acessibilidade & Segurança

### WCAG 2.1 AA
- ✅ 100% navegação por teclado
- ✅ Screen reader states funcionais
- ✅ Contraste adequado
- ✅ Estados não indicados apenas por cor
- ✅ Focus visível em todos os elementos

### Security UX
- Secrets mostrados apenas uma vez na criação
- Copy com confirmação visual
- MFA/SSO surfaces
- Algoritmos + signers sempre visíveis
- AEAD encryption UI para conteúdo protegido

---

## 📊 Telemetria & Métricas

### Ativação
- Time to first receipt
- % com bundle downloaded

### Sucesso
- % ACK
- p95 decision time
- ASK→ACK resolution rate

### Adoção
- Active integrations
- MAU
- Expansion by team

### Saúde
- Errors by region
- Latency p50/p95
- Evidence fetch failures

### Monetização
- Execs per plan
- Conversions Free→Team→Enterprise
- Churn signals

---

## 🧪 Testes UX de Aceite (Must Cover)

1. **ACK**: Gerar /r/\<cid\>, ver timeline, baixar bundle, copiar link ✓
2. **ASK**: Ver PoI + passos; após correção → ACK ✓
3. **NACK**: Ver broken rules + link política ✓
4. **Offline**: Abrir bundle sem backend, verificação passa ✓
5. **A11y**: 100% teclado + screen reader ✓
6. **RBAC**: Auditor vê /r/\<cid\> mas não altera policies ✓
7. **Billing**: Exceder quota mostra upsell (não bloqueia verificação) ✓
8. **Integrations**: Colar Nginx snippet → 1º recibo <5min ✓

---

## 💎 Microcopy Premium

### Estados
- **ACK**: "Decisão aceita — verificação concluída ✓"
- **ASK**: "Faltam evidências para concluir. Envie: {{itens}}"
- **NACK**: "Regras não atendidas: {{motivos}} (política {{id}})"

### Botões
- "Ver Provas"
- "Baixar Bundle"
- "Copiar Link"
- "Abrir Verificador"
- "Tentar Novamente"
- "Começar grátis"

### Placeholders
- "Buscar por título, CID, origin, integration, b3:..."

---

## 🚀 Stack Técnico

- **Framework**: Next.js 16 App Router + TypeScript
- **UI**: Tailwind CSS + shadcn/ui
- **Icons**: Lucide React
- **Fonts**: Inter (sans), JetBrains Mono (mono)
- **Charts**: Recharts
- **Providers**: next-themes (ThemeProvider) + sonner (Toaster)
- **Responsive**: Mobile-first, breakpoints: sm (640px), md (768px), lg (1024px)

---

## 🎯 Diferenciais

1. **Verificação Offline**: Bundle .zip auto-contido, verificável sem backend
2. **Sem Vendor Lock-in**: Export completo, dados sob controle do cliente
3. **SIRP Timeline**: 4 etapas verificáveis com provas criptográficas
4. **RBAC Granular**: 4 roles (Owner/Admin/Operator/Auditor)
5. **Integrações Nativas**: SDKs, Gateways, CI/CD, SIEM
6. **Compliance Built-in**: LGPD/GDPR, multi-região, AEAD encryption
7. **UX Premium**: <150ms transitions, WCAG 2.1 AA, dark mode ready

---

## 📦 Status: ✅ Pronto para Venda

- [x] Marketing completo (landing, pricing, docs, changelog, status)
- [x] Console funcional (dashboard, execuções, receipts, políticas, integrações, auditorias, billing, team, settings)
- [x] Onboarding guiado (4 etapas)
- [x] Verificador offline (/verify/offline)
- [x] Design system consistente
- [x] Acessibilidade WCAG 2.1 AA
- [x] Microcopy profissional em pt-BR
- [x] Componentes reutilizáveis documentados
- [x] Telemetria mapeada
- [x] Testes UX de aceite definidos

**Última atualização**: 2026-02-10
**Versão**: 1.0.0 - Production Ready
