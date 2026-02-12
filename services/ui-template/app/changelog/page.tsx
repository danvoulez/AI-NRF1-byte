"use client"

import { useState } from "react"
import { MarketingHeader } from "@/components/marketing/header"
import { MarketingFooter } from "@/components/marketing/footer"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import {
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Clock,
  Globe,
  Activity,
  ArrowUpRight,
  Zap,
} from "lucide-react"

const systemStatus = {
  overall: "operational" as const,
  lastUpdated: "2026-02-10T14:45:00Z",
}

const services = [
  { name: "API Gateway", status: "operational", latency: "32ms" },
  { name: "Motor SIRP", status: "operational", latency: "89ms" },
  { name: "Armazenamento", status: "operational", latency: "12ms" },
  { name: "Webhooks", status: "operational", latency: "45ms" },
  { name: "Console", status: "operational", latency: "—" },
]

const regionLatency = [
  { region: "BR Brasil Sul (Sao Paulo)", p50: "28ms", p95: "89ms", p99: "142ms", status: "healthy" },
  { region: "US US East (Virginia)", p50: "45ms", p95: "120ms", p99: "198ms", status: "healthy" },
  { region: "EU EU West (Frankfurt)", p50: "52ms", p95: "135ms", p99: "210ms", status: "healthy" },
]

const incidents = [
  {
    id: "inc_004",
    date: "2026-02-08",
    title: "Latencia elevada no Motor SIRP (BR South)",
    status: "resolved" as const,
    duration: "23 minutos",
    description: "Latencia p99 atingiu 380ms por 23 minutos devido a pico de trafego. Autoscaling ativado. Sem perda de dados.",
  },
  {
    id: "inc_003",
    date: "2026-01-28",
    title: "Falha parcial na entrega de webhooks",
    status: "resolved" as const,
    duration: "45 minutos",
    description: "Aproximadamente 2.3% das entregas de webhook falharam. Todos os eventos foram reenviados automaticamente apos resolucao.",
  },
  {
    id: "inc_002",
    date: "2026-01-15",
    title: "Manutencao programada — atualizacao de certificados",
    status: "resolved" as const,
    duration: "5 minutos",
    description: "Rotacao de certificados TLS. Impacto zero para clientes. SDK reconectou automaticamente.",
  },
]

const changelog = [
  {
    version: "3.2.0",
    date: "2026-02-07",
    tag: "Novo",
    title: "Verificacao offline aprimorada",
    changes: [
      "Bundle .zip agora inclui metadados de politica para verificacao completa offline",
      "QR Code nos recibos para compartilhamento rapido",
      "Suporte a impressao PDF do recibo completo",
    ],
  },
  {
    version: "3.1.0",
    date: "2026-01-25",
    tag: "Melhoria",
    title: "RBAC e controles de equipe",
    changes: [
      "Novo papel Auditor com acesso somente leitura",
      "Convites por email com link temporario seguro",
      "Opcao para exigir MFA para todos os membros",
    ],
  },
  {
    version: "3.0.0",
    date: "2026-01-10",
    tag: "Major",
    title: "Protocolo SIRP v2",
    changes: [
      "Novo encadeamento de assinaturas com hash SHA-256 encadeado",
      "Suporte a Ed25519 e HMAC-SHA256 simultaneamente",
      "API REST v1 com versionamento estavel",
      "SDK Rust (tdln-rs) lancado em beta",
    ],
  },
  {
    version: "2.9.0",
    date: "2025-12-20",
    tag: "Melhoria",
    title: "Packs de politicas expandidos",
    changes: [
      "Novo pack: PCI DSS com 32 regras",
      "Pack Data Residency atualizado com regras LGPD",
      "Soft fail/hard fail configuravel por regra",
    ],
  },
  {
    version: "2.8.0",
    date: "2025-12-05",
    tag: "Novo",
    title: "Integracoes SIEM e export",
    changes: [
      "Export CSV do audit log",
      "Integracao SIEM via webhook customizado (Enterprise)",
      "Novo formato de export JSON padronizado",
    ],
  },
]

const statusColor: Record<string, string> = {
  operational: "text-emerald-600",
  degraded: "text-amber-600",
  outage: "text-red-600",
}

const statusLabel: Record<string, string> = {
  operational: "Operacional",
  degraded: "Degradado",
  outage: "Indisponivel",
}

const tagColors: Record<string, string> = {
  Novo: "bg-emerald-500/10 text-emerald-600 border-emerald-500/20",
  Melhoria: "bg-muted text-foreground",
  Major: "bg-foreground text-background",
  Fix: "bg-red-500/10 text-red-600 border-red-500/20",
}

export default function ChangelogPage() {
  return (
    <div className="flex min-h-screen flex-col">
      <MarketingHeader />
      <main className="flex-1">
        <section className="py-12 lg:py-20">
          <div className="mx-auto max-w-4xl px-4 lg:px-8">
            <h1 className="text-balance text-3xl font-bold tracking-tight text-foreground md:text-4xl">
              Status & Changelog
            </h1>
            <p className="mt-2 text-muted-foreground">
              Status operacional da plataforma, incidentes e notas de versao.
            </p>

            <Tabs defaultValue="status" className="mt-8">
              <TabsList>
                <TabsTrigger value="status">Status</TabsTrigger>
                <TabsTrigger value="changelog">Changelog</TabsTrigger>
                <TabsTrigger value="incidents">Incidentes</TabsTrigger>
              </TabsList>

              {/* Status */}
              <TabsContent value="status" className="mt-6 space-y-6">
                {/* Overall banner */}
                <div className="flex items-center gap-4 rounded-xl border border-emerald-500/20 bg-emerald-500/5 p-5">
                  <CheckCircle2 className="h-6 w-6 shrink-0 text-emerald-600" />
                  <div>
                    <p className="text-lg font-semibold text-foreground">Todos os sistemas operacionais</p>
                    <p className="text-sm text-muted-foreground">
                      Atualizado em {new Date(systemStatus.lastUpdated).toLocaleString("pt-BR")}
                    </p>
                  </div>
                </div>

                {/* Services */}
                <Card>
                  <CardHeader>
                    <CardTitle className="text-sm font-medium text-foreground">Servicos</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-3">
                      {services.map((svc) => (
                        <div key={svc.name} className="flex items-center justify-between rounded-md border p-3">
                          <div className="flex items-center gap-3">
                            <div className="h-2.5 w-2.5 rounded-full bg-emerald-500" />
                            <span className="text-sm font-medium text-foreground">{svc.name}</span>
                          </div>
                          <div className="flex items-center gap-4">
                            <span className="text-xs text-muted-foreground">{svc.latency}</span>
                            <span className={`text-xs font-medium ${statusColor[svc.status]}`}>
                              {statusLabel[svc.status]}
                            </span>
                          </div>
                        </div>
                      ))}
                    </div>
                  </CardContent>
                </Card>

                {/* Region latency */}
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-sm font-medium text-foreground">
                      <Globe className="h-4 w-4" />
                      Latencia por regiao
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="overflow-x-auto">
                      <table className="w-full text-sm">
                        <thead>
                          <tr className="border-b">
                            <th className="pb-2 text-left font-medium text-muted-foreground">Regiao</th>
                            <th className="pb-2 text-right font-medium text-muted-foreground">p50</th>
                            <th className="pb-2 text-right font-medium text-muted-foreground">p95</th>
                            <th className="pb-2 text-right font-medium text-muted-foreground">p99</th>
                            <th className="pb-2 text-right font-medium text-muted-foreground">Status</th>
                          </tr>
                        </thead>
                        <tbody>
                          {regionLatency.map((r) => (
                            <tr key={r.region} className="border-b last:border-0">
                              <td className="py-3 text-foreground">{r.region}</td>
                              <td className="py-3 text-right font-mono text-xs text-foreground">{r.p50}</td>
                              <td className="py-3 text-right font-mono text-xs text-foreground">{r.p95}</td>
                              <td className="py-3 text-right font-mono text-xs text-muted-foreground">{r.p99}</td>
                              <td className="py-3 text-right">
                                <span className="inline-flex items-center gap-1 text-xs font-medium text-emerald-600">
                                  <Activity className="h-3 w-3" />
                                  Saudavel
                                </span>
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  </CardContent>
                </Card>
              </TabsContent>

              {/* Changelog */}
              <TabsContent value="changelog" className="mt-6 space-y-6">
                {changelog.map((entry) => (
                  <div key={entry.version} className="relative pl-8">
                    {/* Timeline line */}
                    <div className="absolute left-3 top-0 h-full w-px bg-border" aria-hidden="true" />
                    <div className="absolute left-1.5 top-1.5 h-3 w-3 rounded-full border-2 border-foreground bg-background" aria-hidden="true" />

                    <div className="rounded-xl border bg-card p-5">
                      <div className="flex flex-wrap items-center gap-2">
                        <Badge className={`text-xs ${tagColors[entry.tag] || tagColors.Melhoria}`}>
                          {entry.tag}
                        </Badge>
                        <span className="font-mono text-sm font-semibold text-foreground">v{entry.version}</span>
                        <span className="text-xs text-muted-foreground">{new Date(entry.date).toLocaleDateString("pt-BR")}</span>
                      </div>
                      <h3 className="mt-2 text-base font-semibold text-foreground">{entry.title}</h3>
                      <ul className="mt-3 space-y-1.5">
                        {entry.changes.map((change) => (
                          <li key={change} className="flex items-start gap-2 text-sm text-muted-foreground">
                            <Zap className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground/50" />
                            {change}
                          </li>
                        ))}
                      </ul>
                    </div>
                  </div>
                ))}
              </TabsContent>

              {/* Incidents */}
              <TabsContent value="incidents" className="mt-6 space-y-4">
                {incidents.map((incident) => (
                  <Card key={incident.id}>
                    <CardContent className="p-5">
                      <div className="flex items-start justify-between gap-4">
                        <div className="flex-1">
                          <div className="flex flex-wrap items-center gap-2">
                            <CheckCircle2 className="h-4 w-4 shrink-0 text-emerald-600" />
                            <h3 className="text-sm font-semibold text-foreground">{incident.title}</h3>
                          </div>
                          <p className="mt-2 text-sm text-muted-foreground leading-relaxed">{incident.description}</p>
                        </div>
                        <div className="shrink-0 text-right">
                          <p className="text-xs text-muted-foreground">{new Date(incident.date).toLocaleDateString("pt-BR")}</p>
                          <div className="mt-1 flex items-center gap-1">
                            <Clock className="h-3 w-3 text-muted-foreground" />
                            <span className="text-xs text-muted-foreground">{incident.duration}</span>
                          </div>
                        </div>
                      </div>
                      <div className="mt-3">
                        <Badge variant="secondary" className="text-xs">
                          <CheckCircle2 className="mr-1 h-3 w-3 text-emerald-600" />
                          Resolvido
                        </Badge>
                      </div>
                    </CardContent>
                  </Card>
                ))}

                <div className="text-center">
                  <p className="text-sm text-muted-foreground">
                    Exibindo os 3 incidentes mais recentes. Uptime nos ultimos 90 dias: <span className="font-semibold text-foreground">99.97%</span>
                  </p>
                </div>
              </TabsContent>
            </Tabs>
          </div>
        </section>
      </main>
      <MarketingFooter />
    </div>
  )
}
