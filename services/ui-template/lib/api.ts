import { REGISTRY_URL } from "./env"
import type {
  Execution,
  ExecutionState,
  SIRPNode,
  Proof,
  Evidence,
  AuditEntry,
} from "./mock-data"
import {
  mockExecutions,
  mockSIRPNodes,
  mockProofs,
  mockEvidence,
  mockAuditLog,
  mockMetrics,
  mockPolicyPacks,
} from "./mock-data"

// ---------------------------------------------------------------------------
// Wire types (match Rust RunResponse)
// ---------------------------------------------------------------------------

export interface RunRequest {
  manifest: Record<string, unknown>
  env: Record<string, unknown>
  tenant: string
}

export interface HopInfo {
  step: string
  kind: string
  hash: string
  verified: boolean
}

export interface MetricEntry {
  step: string
  metric: string
  value: number
}

export interface RunResponse {
  ok: boolean
  verdict: string
  stopped_at: string | null
  receipt_cid: string
  receipt_chain: string[]
  url_rica: string
  hops: HopInfo[]
  metrics: MetricEntry[]
  artifacts: number
  error?: string
}

// ---------------------------------------------------------------------------
// API client
// ---------------------------------------------------------------------------

async function post<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${REGISTRY_URL}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }))
    throw new Error(err.error || `HTTP ${res.status}`)
  }
  return res.json()
}

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${REGISTRY_URL}${path}`)
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }))
    throw new Error(err.error || `HTTP ${res.status}`)
  }
  return res.json()
}

// ---------------------------------------------------------------------------
// Pipeline execution
// ---------------------------------------------------------------------------

export async function runPipeline(req: RunRequest): Promise<RunResponse> {
  return post<RunResponse>("/modules/run", req)
}

// ---------------------------------------------------------------------------
// Verdict → ExecutionState mapping
// ---------------------------------------------------------------------------

function verdictToState(verdict: string): ExecutionState {
  if (verdict === "Allow") return "ACK"
  if (verdict === "Deny") return "NACK"
  return "ASK"
}

// ---------------------------------------------------------------------------
// Convert RunResponse → UI types
// ---------------------------------------------------------------------------

export function runResponseToExecution(
  resp: RunResponse,
  title: string,
  origin: string = "api-gateway",
  integration: string = "SDK"
): Execution {
  return {
    id: `exec_${Date.now()}`,
    state: verdictToState(resp.verdict),
    cid: resp.receipt_cid,
    title,
    origin,
    timestamp: new Date().toISOString(),
    integration,
  }
}

export function hopsToSIRPNodes(hops: HopInfo[]): SIRPNode[] {
  const sirpSteps: Array<"INTENT" | "DELIVERY" | "EXECUTION" | "RESULT"> = [
    "INTENT",
    "DELIVERY",
    "EXECUTION",
    "RESULT",
  ]
  return hops.map((hop, i) => ({
    step: sirpSteps[i] || ("EXECUTION" as const),
    signer: `engine:${hop.kind}@1.0.0`,
    timestamp: new Date().toISOString(),
    verified: hop.verified,
    algorithm: "Ed25519",
    hash: hop.hash,
  }))
}

export function hopsToProofs(hops: HopInfo[]): Proof[] {
  const proofTypes = [
    "Capsule INTENT",
    "Receipt DELIVERY",
    "Receipt EXECUTION",
    "Capsule RESULT",
  ]
  return hops.map((hop, i) => ({
    type: proofTypes[i] || `Receipt ${hop.step}`,
    algorithm: "Ed25519",
    cid: hop.hash,
    signer: `engine:${hop.kind}@1.0.0`,
    timestamp: new Date().toISOString(),
  }))
}

// ---------------------------------------------------------------------------
// Data fetchers (real API with mock fallback)
// ---------------------------------------------------------------------------

export async function fetchExecutions(): Promise<Execution[]> {
  try {
    const data = await get<Execution[]>("/api/executions")
    return data
  } catch {
    return mockExecutions
  }
}

export async function fetchReceipt(cid: string): Promise<{
  execution: Execution
  sirp: SIRPNode[]
  proofs: Proof[]
  evidence: Evidence[]
}> {
  try {
    const data = await get<{
      execution: Execution
      sirp: SIRPNode[]
      proofs: Proof[]
      evidence: Evidence[]
    }>(`/api/receipts/${encodeURIComponent(cid)}`)
    return data
  } catch {
    const exec = mockExecutions.find((e) => e.cid === cid) || mockExecutions[0]
    return {
      execution: exec,
      sirp: mockSIRPNodes,
      proofs: mockProofs,
      evidence: mockEvidence,
    }
  }
}

export async function fetchAuditLog(): Promise<AuditEntry[]> {
  try {
    return await get<AuditEntry[]>("/api/audits")
  } catch {
    return mockAuditLog
  }
}

export async function fetchMetrics(): Promise<typeof mockMetrics> {
  try {
    return await get<typeof mockMetrics>("/api/metrics")
  } catch {
    return mockMetrics
  }
}

export async function fetchEvidence(): Promise<Evidence[]> {
  try {
    return await get<Evidence[]>("/api/evidence")
  } catch {
    return mockEvidence
  }
}

export async function fetchPolicies(): Promise<import("./mock-data").PolicyPack[]> {
  try {
    return await get<import("./mock-data").PolicyPack[]>("/api/policies")
  } catch {
    return mockPolicyPacks
  }
}

// ---------------------------------------------------------------------------
// Re-export types for convenience
// ---------------------------------------------------------------------------

export type {
  Execution,
  ExecutionState,
  SIRPNode,
  Proof,
  Evidence,
  AuditEntry,
  TeamMember,
  PolicyPack,
  Invoice,
} from "./mock-data"
