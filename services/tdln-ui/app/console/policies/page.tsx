"use client"

import { useState, useEffect } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Switch } from "@/components/ui/switch"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { fetchPolicies, type PolicyPack } from "@/lib/api"
import { ShieldCheck, ChevronRight, BookOpen, Settings2 } from "lucide-react"
import Link from "next/link"
import { toast } from "sonner"

export default function PoliciesPage() {
  const [packs, setPacks] = useState<PolicyPack[]>([])

  useEffect(() => {
    fetchPolicies().then(setPacks)
  }, [])
  const [selectedPack, setSelectedPack] = useState<PolicyPack | null>(null)

  const togglePack = (id: string) => {
    setPacks((prev) => prev.map((p) => (p.id === id ? { ...p, enabled: !p.enabled } : p)))
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-foreground">Politicas & Packs</h1>
        <p className="mt-1 text-sm text-muted-foreground">Gerencie regras de verificacao e packs de compliance.</p>
      </div>

      {/* Vertical Presets */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium text-foreground">Presets por Vertical</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {[
              { name: "FinTech / Banking", packs: ["Sanctions AML", "KYC Compliance", "PCI DSS"] },
              { name: "HealthTech", packs: ["HIPAA", "LGPD Health", "FDA Compliance"] },
              { name: "E-commerce", packs: ["PCI DSS", "Fraud Detection", "GDPR"] },
              { name: "AI/ML Services", packs: ["AI Safety", "Model Audit", "Bias Detection"] },
            ].map((vertical) => (
              <button
                key={vertical.name}
                className="rounded-lg border bg-card p-3 text-left transition-colors hover:bg-muted/50"
                onClick={() => { toast.success(`Preset "${vertical.name}" aplicado. ${vertical.packs.length} packs recomendados ativados.`) }}
              >
                <p className="text-sm font-semibold text-foreground">{vertical.name}</p>
                <p className="mt-1 text-xs text-muted-foreground">{vertical.packs.length} packs recomendados</p>
              </button>
            ))}
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-6 lg:grid-cols-5">
        {/* Pack list */}
        <div className="lg:col-span-2 space-y-3">
          {packs.map((pack) => (
            <button
              key={pack.id}
              onClick={() => setSelectedPack(pack)}
              className={`flex w-full items-center justify-between rounded-xl border p-4 text-left transition-colors ${
                selectedPack?.id === pack.id ? "border-foreground bg-muted ring-1 ring-foreground" : "bg-card hover:bg-muted/50"
              }`}
            >
              <div className="flex items-center gap-3 min-w-0">
                <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-muted">
                  <ShieldCheck className="h-5 w-5 text-foreground" />
                </div>
                <div className="min-w-0">
                  <p className="truncate text-sm font-semibold text-foreground">{pack.name}</p>
                  <p className="text-xs text-muted-foreground">{pack.category} &middot; v{pack.version}</p>
                </div>
              </div>
              <div className="flex items-center gap-3 shrink-0">
                <Switch
                  checked={pack.enabled}
                  onCheckedChange={() => togglePack(pack.id)}
                  onClick={(e) => e.stopPropagation()}
                  aria-label={`${pack.enabled ? "Desativar" : "Ativar"} ${pack.name}`}
                />
                <ChevronRight className="h-4 w-4 text-muted-foreground" />
              </div>
            </button>
          ))}
        </div>

        {/* Pack detail */}
        <div className="lg:col-span-3">
          {selectedPack ? (
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div>
                    <CardTitle className="text-lg font-bold text-foreground">{selectedPack.name}</CardTitle>
                    <p className="mt-1 text-sm text-muted-foreground">{selectedPack.description}</p>
                  </div>
                  <Badge variant={selectedPack.enabled ? "default" : "secondary"}>
                    {selectedPack.enabled ? "Ativo" : "Inativo"}
                  </Badge>
                </div>
              </CardHeader>
              <CardContent className="space-y-6">
                <div className="grid gap-4 sm:grid-cols-3">
                  <div className="rounded-lg border bg-muted/50 p-3 text-center">
                    <p className="text-2xl font-bold text-foreground">{selectedPack.rules}</p>
                    <p className="text-xs text-muted-foreground">Regras</p>
                  </div>
                  <div className="rounded-lg border bg-muted/50 p-3 text-center">
                    <p className="text-2xl font-bold text-foreground">v{selectedPack.version}</p>
                    <p className="text-xs text-muted-foreground">Versao</p>
                  </div>
                  <div className="rounded-lg border bg-muted/50 p-3 text-center">
                    <p className="text-2xl font-bold text-foreground">{selectedPack.category}</p>
                    <p className="text-xs text-muted-foreground">Categoria</p>
                  </div>
                </div>

                {/* Mock rules */}
                <div>
                  <h3 className="text-sm font-semibold text-foreground">Regras</h3>
                  <div className="mt-3 space-y-2">
                    {[
                      { name: "Verificacao OFAC", type: "hard", threshold: "0.85" },
                      { name: "Lista EU Sanctions", type: "hard", threshold: "0.80" },
                      { name: "PEP Screening", type: "soft", threshold: "0.70" },
                      { name: "Adverse Media", type: "soft", threshold: "0.65" },
                    ].map((rule) => (
                      <div key={rule.name} className="flex items-center justify-between rounded-md border bg-card p-3">
                        <div>
                          <p className="text-sm font-medium text-foreground">{rule.name}</p>
                          <p className="text-xs text-muted-foreground">Threshold: {rule.threshold}</p>
                        </div>
                        <Badge variant={rule.type === "hard" ? "destructive" : "secondary"} className="text-xs">
                          {rule.type === "hard" ? "Hard fail" : "Soft fail"}
                        </Badge>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Review Notes */}
                <div>
                  <h3 className="text-sm font-semibold text-foreground">Notas de Revisao</h3>
                  <div className="mt-3 rounded-lg border bg-muted/30 p-3">
                    <p className="text-xs leading-relaxed text-muted-foreground">
                      Ultima revisao: <strong className="text-foreground">2026-01-15</strong> por <strong className="text-foreground">Compliance Team</strong>.
                      Pack aprovado para producao. Proxima revisao agendada para Q2 2026. Soft fails nao bloqueiam execucao mas sao registrados para auditoria.
                    </p>
                  </div>
                </div>

                <div className="flex gap-2">
                  <Button variant="outline" size="sm" onClick={() => toast.info(`Editor de regras para "${selectedPack.name}" sera disponibilizado em breve.`)}>
                    <Settings2 className="mr-1.5 h-4 w-4" />
                    Configurar
                  </Button>
                  <Button variant="outline" size="sm" asChild>
                    <Link href="/docs">
                      <BookOpen className="mr-1.5 h-4 w-4" />
                      Documentacao
                    </Link>
                  </Button>
                </div>
              </CardContent>
            </Card>
          ) : (
            <Card>
              <CardContent className="flex flex-col items-center justify-center py-16">
                <ShieldCheck className="h-8 w-8 text-muted-foreground" />
                <h2 className="mt-4 text-lg font-semibold text-foreground">Selecione um pack</h2>
                <p className="mt-1 text-sm text-muted-foreground">Clique em um pack a esquerda para ver detalhes.</p>
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </div>
  )
}
