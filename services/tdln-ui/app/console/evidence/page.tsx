"use client"

import { useState, useEffect } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible"
import { CIDChip } from "@/components/tdln/cid-chip"
import { fetchEvidence, type Evidence } from "@/lib/api"
import { Search, Lock, RefreshCw, ExternalLink, Database, ChevronDown, Star, Globe } from "lucide-react"
import { toast } from "sonner"

// Mock mirrors data
const mockMirrors = [
  { url: "https://ipfs.io/ipfs/Qm...", status: "online", latency: 42, preferred: true },
  { url: "https://gateway.pinata.cloud/ipfs/Qm...", status: "online", latency: 87, preferred: false },
  { url: "https://cloudflare-ipfs.com/ipfs/Qm...", status: "online", latency: 65, preferred: false },
]

export default function EvidencePage() {
  const [query, setQuery] = useState("")
  const [expandedCid, setExpandedCid] = useState<string | null>(null)
  const [evidence, setEvidence] = useState<Evidence[]>([])

  useEffect(() => {
    fetchEvidence().then(setEvidence)
  }, [])

  const filtered = evidence.filter((e) =>
    query ? e.cid.includes(query) || e.url.includes(query) : true
  )

  const handlePinMirror = (mirrorUrl: string) => {
    toast.success(`Mirror ${mirrorUrl} definido como preferido`)
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-foreground">Evidencias</h1>
        <p className="mt-1 text-sm text-muted-foreground">Busque e gerencie evidencias por CID ou URL.</p>
      </div>

      <div className="relative max-w-md">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          placeholder="Buscar por CID ou URL..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="pl-9"
          aria-label="Buscar evidencias"
        />
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        {filtered.map((ev) => (
          <Card key={ev.cid}>
            <CardContent className="p-4">
              <div className="flex items-start justify-between">
                <CIDChip cid={ev.cid} />
                <span className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium ${
                  ev.status === "fetched" ? "bg-emerald-500/10 text-emerald-600" :
                  ev.status === "protected" ? "bg-muted text-muted-foreground" :
                  "bg-red-500/10 text-red-600"
                }`}>
                  {ev.status === "protected" && <Lock className="h-3 w-3" />}
                  {ev.status === "fetched" ? "Obtido" : ev.status === "protected" ? "AEAD/protegido" : "Falha"}
                </span>
              </div>
              <p className="mt-2 truncate text-xs text-muted-foreground">{ev.url}</p>
              {ev.mime && <p className="mt-1 text-xs text-muted-foreground">MIME: {ev.mime}</p>}
              
              <div className="mt-3 flex gap-2">
                <Button variant="outline" size="sm" className="h-7 text-xs bg-transparent">
                  <ExternalLink className="mr-1 h-3 w-3" />
                  Abrir
                </Button>
                {ev.status === "failed" && (
                  <Button variant="outline" size="sm" className="h-7 text-xs bg-transparent">
                    <RefreshCw className="mr-1 h-3 w-3" />
                    Tentar Novamente
                  </Button>
                )}
              </div>

              {/* Resolutions & Mirrors */}
              {ev.status === "fetched" && (
                <Collapsible
                  open={expandedCid === ev.cid}
                  onOpenChange={(open) => setExpandedCid(open ? ev.cid : null)}
                  className="mt-3"
                >
                  <CollapsibleTrigger asChild>
                    <Button variant="ghost" size="sm" className="h-7 w-full justify-between text-xs">
                      <span className="flex items-center gap-1.5">
                        <Globe className="h-3 w-3" />
                        Resolucoes & Mirrors ({mockMirrors.length})
                      </span>
                      <ChevronDown className={`h-3 w-3 transition-transform ${expandedCid === ev.cid ? "rotate-180" : ""}`} />
                    </Button>
                  </CollapsibleTrigger>
                  <CollapsibleContent className="mt-2 space-y-2">
                    {mockMirrors.map((mirror, idx) => (
                      <div key={idx} className="rounded-md border bg-muted/30 p-2.5">
                        <div className="flex items-start justify-between gap-2">
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-1.5">
                              <p className="truncate text-xs font-medium text-foreground">{mirror.url}</p>
                              {mirror.preferred && (
                                <Star className="h-3 w-3 shrink-0 fill-amber-500 text-amber-500" />
                              )}
                            </div>
                            <div className="mt-1 flex items-center gap-2">
                              <Badge variant="secondary" className="h-4 px-1.5 text-[10px]">
                                {mirror.latency}ms
                              </Badge>
                              <Badge 
                                variant={mirror.status === "online" ? "default" : "destructive"} 
                                className="h-4 px-1.5 text-[10px]"
                              >
                                {mirror.status}
                              </Badge>
                            </div>
                          </div>
                          {!mirror.preferred && (
                            <Button
                              variant="ghost"
                              size="sm"
                              className="h-6 px-2 text-[10px]"
                              onClick={() => handlePinMirror(mirror.url)}
                            >
                              <Star className="mr-1 h-2.5 w-2.5" />
                              Preferir
                            </Button>
                          )}
                        </div>
                      </div>
                    ))}
                  </CollapsibleContent>
                </Collapsible>
              )}
            </CardContent>
          </Card>
        ))}
      </div>

      {filtered.length === 0 && (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-16">
            <Database className="h-8 w-8 text-muted-foreground" />
            <h2 className="mt-4 text-lg font-semibold text-foreground">Nenhuma evidencia encontrada</h2>
            <p className="mt-1 text-sm text-muted-foreground">Tente buscar por outro CID ou URL.</p>
          </CardContent>
        </Card>
      )}
    </div>
  )
}
