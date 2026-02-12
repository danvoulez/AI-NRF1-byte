export type ExecutionState = "ACK" | "NACK" | "ASK"

export interface Execution {
  id: string
  state: ExecutionState
  cid: string
  title: string
  origin: string
  timestamp: string
  integration: string
}

export interface SIRPNode {
  step: "INTENT" | "DELIVERY" | "EXECUTION" | "RESULT"
  signer: string
  timestamp: string
  verified: boolean
  algorithm: string
  hash: string
}

export interface Proof {
  type: string
  algorithm: string
  cid: string
  signer: string
  timestamp: string
  data?: string
}

export interface Evidence {
  cid: string
  url: string
  status: "fetched" | "pending" | "failed" | "protected"
  mime?: string
}

export interface AuditEntry {
  id: string
  user: string
  action: string
  target: string
  timestamp: string
  ip: string
  cid?: string
  diff?: Record<string, unknown>
}

export interface TeamMember {
  id: string
  name: string
  email: string
  role: "Owner" | "Admin" | "Operator" | "Auditor"
  mfa: boolean
  lastActive: string
}

export interface PolicyPack {
  id: string
  name: string
  enabled: boolean
  version: string
  category: string
  rules: number
  description: string
}

export interface Invoice {
  id: string
  date: string
  amount: string
  status: "paid" | "pending" | "overdue"
}

// Mock Executions
export const mockExecutions: Execution[] = [
  {
    id: "exec_001",
    state: "ACK",
    cid: "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
    title: "Transacao financeira #4821",
    origin: "api-gateway",
    timestamp: "2026-02-10T14:32:00Z",
    integration: "SDK Node.js",
  },
  {
    id: "exec_002",
    state: "NACK",
    cid: "bafybeihkoviema7g3gxyt6la7vd5ho32uj4yz3pdufml5tz7pzq7v55uru",
    title: "Verificacao KYC usuario #1293",
    origin: "webhook-handler",
    timestamp: "2026-02-10T14:28:00Z",
    integration: "Nginx Gateway",
  },
  {
    id: "exec_003",
    state: "ASK",
    cid: "bafybeiemxf5abjwjbikoz4mc3a3dla6ual3jsgpdr4cjr3oz3evfyavhwq",
    title: "Contrato digital #892",
    origin: "api-gateway",
    timestamp: "2026-02-10T14:15:00Z",
    integration: "SDK Python",
  },
  {
    id: "exec_004",
    state: "ACK",
    cid: "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzaa",
    title: "Auditoria compliance Q1",
    origin: "cron-worker",
    timestamp: "2026-02-10T13:45:00Z",
    integration: "SDK Rust",
  },
  {
    id: "exec_005",
    state: "ACK",
    cid: "bafybeihkoviema7g3gxyt6la7vd5ho32uj4yz3pdufml5tz7pzq7v55fbb",
    title: "Pagamento fornecedor #3301",
    origin: "api-gateway",
    timestamp: "2026-02-10T13:20:00Z",
    integration: "SDK Node.js",
  },
  {
    id: "exec_006",
    state: "NACK",
    cid: "bafybeiemxf5abjwjbikoz4mc3a3dla6ual3jsgpdr4cjr3oz3evfyavhcc",
    title: "Exportacao dados LGPD #102",
    origin: "webhook-handler",
    timestamp: "2026-02-10T12:58:00Z",
    integration: "Envoy Gateway",
  },
  {
    id: "exec_007",
    state: "ACK",
    cid: "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdd",
    title: "Assinatura contrato SaaS",
    origin: "api-gateway",
    timestamp: "2026-02-10T12:30:00Z",
    integration: "SDK Node.js",
  },
  {
    id: "exec_008",
    state: "ASK",
    cid: "bafybeihkoviema7g3gxyt6la7vd5ho32uj4yz3pdufml5tz7pzq7v55fee",
    title: "Revisao politica AML #47",
    origin: "manual",
    timestamp: "2026-02-10T11:45:00Z",
    integration: "Console",
  },
]

// Mock SIRP Timeline
export const mockSIRPNodes: SIRPNode[] = [
  {
    step: "INTENT",
    signer: "client:sdk-node@2.1.0",
    timestamp: "2026-02-10T14:31:58.120Z",
    verified: true,
    algorithm: "Ed25519",
    hash: "sha256:a1b2c3d4e5f6...",
  },
  {
    step: "DELIVERY",
    signer: "gateway:nginx@1.25.0",
    timestamp: "2026-02-10T14:31:58.250Z",
    verified: true,
    algorithm: "HMAC-SHA256",
    hash: "sha256:f6e5d4c3b2a1...",
  },
  {
    step: "EXECUTION",
    signer: "engine:tdln-core@3.0.1",
    timestamp: "2026-02-10T14:31:59.001Z",
    verified: true,
    algorithm: "Ed25519",
    hash: "sha256:1a2b3c4d5e6f...",
  },
  {
    step: "RESULT",
    signer: "engine:tdln-core@3.0.1",
    timestamp: "2026-02-10T14:32:00.000Z",
    verified: true,
    algorithm: "Ed25519",
    hash: "sha256:6f5e4d3c2b1a...",
  },
]

// Mock Proofs
export const mockProofs: Proof[] = [
  {
    type: "Capsule INTENT",
    algorithm: "Ed25519",
    cid: "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
    signer: "client:sdk-node@2.1.0",
    timestamp: "2026-02-10T14:31:58.120Z",
  },
  {
    type: "Receipt DELIVERY",
    algorithm: "HMAC-SHA256",
    cid: "bafybeihkoviema7g3gxyt6la7vd5ho32uj4yz3pdufml5tz7pzq7v55uru",
    signer: "gateway:nginx@1.25.0",
    timestamp: "2026-02-10T14:31:58.250Z",
  },
  {
    type: "Receipt EXECUTION",
    algorithm: "Ed25519",
    cid: "bafybeiemxf5abjwjbikoz4mc3a3dla6ual3jsgpdr4cjr3oz3evfyavhwq",
    signer: "engine:tdln-core@3.0.1",
    timestamp: "2026-02-10T14:31:59.001Z",
  },
  {
    type: "Capsule RESULT",
    algorithm: "Ed25519",
    cid: "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzaa",
    signer: "engine:tdln-core@3.0.1",
    timestamp: "2026-02-10T14:32:00.000Z",
  },
]

// Mock Evidence
export const mockEvidence: Evidence[] = [
  { cid: "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi", url: "https://ipfs.io/ipfs/bafybeig...", status: "fetched", mime: "application/json" },
  { cid: "bafybeihkoviema7g3gxyt6la7vd5ho32uj4yz3pdufml5tz7pzq7v55uru", url: "https://arweave.net/tx/abc123", status: "fetched", mime: "application/pdf" },
  { cid: "bafybeiemxf5abjwjbikoz4mc3a3dla6ual3jsgpdr4cjr3oz3evfyavhwq", url: "https://storage.tdln.io/e/...", status: "protected", mime: "application/octet-stream" },
]

// Mock Audit Log
export const mockAuditLog: AuditEntry[] = [
  { id: "aud_001", user: "maria@empresa.com", action: "execution.create", target: "exec_001", timestamp: "2026-02-10T14:32:00Z", ip: "189.45.32.100", cid: "bafybeigdyrzt..." },
  { id: "aud_002", user: "carlos@empresa.com", action: "policy.update", target: "pack_sanctions_aml", timestamp: "2026-02-10T14:00:00Z", ip: "189.45.32.101", diff: { threshold: { from: 0.7, to: 0.85 } } },
  { id: "aud_003", user: "ana@empresa.com", action: "team.invite", target: "joao@empresa.com", timestamp: "2026-02-10T13:45:00Z", ip: "189.45.32.102" },
  { id: "aud_004", user: "sistema", action: "key.rotate", target: "api_key_prod_01", timestamp: "2026-02-10T12:00:00Z", ip: "internal" },
  { id: "aud_005", user: "maria@empresa.com", action: "execution.download_bundle", target: "exec_001", timestamp: "2026-02-10T11:30:00Z", ip: "189.45.32.100", cid: "bafybeigdyrzt..." },
  { id: "aud_006", user: "carlos@empresa.com", action: "integration.create", target: "webhook_slack", timestamp: "2026-02-10T10:00:00Z", ip: "189.45.32.101" },
]

// Mock Team
export const mockTeam: TeamMember[] = [
  { id: "u_001", name: "Maria Silva", email: "maria@empresa.com", role: "Owner", mfa: true, lastActive: "2026-02-10T14:32:00Z" },
  { id: "u_002", name: "Carlos Oliveira", email: "carlos@empresa.com", role: "Admin", mfa: true, lastActive: "2026-02-10T14:00:00Z" },
  { id: "u_003", name: "Ana Santos", email: "ana@empresa.com", role: "Operator", mfa: false, lastActive: "2026-02-10T13:45:00Z" },
  { id: "u_004", name: "Joao Pereira", email: "joao@empresa.com", role: "Auditor", mfa: true, lastActive: "2026-02-09T18:00:00Z" },
]

// Mock Policy Packs
export const mockPolicyPacks: PolicyPack[] = [
  { id: "pack_001", name: "Sanctions AML", enabled: true, version: "2.3.1", category: "Financeiro", rules: 24, description: "Verificacao de sancoes e anti-lavagem de dinheiro com listas OFAC, EU e ONU." },
  { id: "pack_002", name: "AI Safety", enabled: true, version: "1.1.0", category: "IA", rules: 12, description: "Guardrails para outputs de modelos de IA incluindo toxicidade, bias e alucinacao." },
  { id: "pack_003", name: "SLA Compliance", enabled: false, version: "1.0.0", category: "Operacional", rules: 8, description: "Monitoramento de conformidade com SLAs contratuais e tempos de resposta." },
  { id: "pack_004", name: "Data Residency", enabled: true, version: "1.2.0", category: "Privacidade", rules: 6, description: "Verificacao de residencia de dados conforme LGPD, GDPR e regulacoes locais." },
  { id: "pack_005", name: "PCI DSS", enabled: false, version: "3.0.0", category: "Financeiro", rules: 32, description: "Conformidade com padroes de seguranca de dados para industria de cartoes de pagamento." },
]

// Mock Invoices
export const mockInvoices: Invoice[] = [
  { id: "inv_001", date: "2026-02-01", amount: "R$ 2.490,00", status: "paid" },
  { id: "inv_002", date: "2026-01-01", amount: "R$ 2.490,00", status: "paid" },
  { id: "inv_003", date: "2025-12-01", amount: "R$ 1.990,00", status: "paid" },
  { id: "inv_004", date: "2025-11-01", amount: "R$ 1.990,00", status: "paid" },
]

// Dashboard metrics
export const mockMetrics = {
  executionsToday: 1247,
  ackPercentage: 94.2,
  p99Latency: 142,
  activeIntegrations: 4,
  weeklyData: [
    { day: "Seg", executions: 980, ack: 920 },
    { day: "Ter", executions: 1120, ack: 1050 },
    { day: "Qua", executions: 1340, ack: 1280 },
    { day: "Qui", executions: 1100, ack: 1020 },
    { day: "Sex", executions: 1247, ack: 1175 },
    { day: "Sab", executions: 420, ack: 405 },
    { day: "Dom", executions: 280, ack: 270 },
  ],
}
