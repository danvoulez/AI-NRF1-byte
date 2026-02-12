"use client"

import { useState } from "react"
import type { Proof } from "@/lib/mock-data"
import { Copy, Download, Check } from "lucide-react"
import { Button } from "@/components/ui/button"
import { CIDChip } from "./cid-chip"

export function CardProva({ proof }: { proof: Proof }) {
  const [copied, setCopied] = useState(false)

  const handleCopyJSON = async () => {
    await navigator.clipboard.writeText(JSON.stringify(proof, null, 2))
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="rounded-lg border bg-card p-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-medium text-foreground">{proof.type}</p>
          <p className="mt-0.5 text-xs text-muted-foreground">{proof.algorithm}</p>
        </div>
      </div>
      <div className="mt-3 grid gap-2 text-xs text-muted-foreground">
        <div className="flex items-center justify-between">
          <span>CID</span>
          <CIDChip cid={proof.cid} />
        </div>
        <div className="flex items-center justify-between">
          <span>Assinante</span>
          <span className="font-mono text-foreground">{proof.signer}</span>
        </div>
        <div className="flex items-center justify-between">
          <span>Timestamp</span>
          <span className="text-foreground">{new Date(proof.timestamp).toLocaleString("pt-BR")}</span>
        </div>
      </div>
      <div className="mt-3 flex items-center gap-2">
        <Button variant="outline" size="sm" onClick={handleCopyJSON} className="h-7 text-xs bg-transparent">
          {copied ? <Check className="mr-1 h-3 w-3" /> : <Copy className="mr-1 h-3 w-3" />}
          {copied ? "Copiado" : "Copiar JSON"}
        </Button>
        <Button variant="outline" size="sm" className="h-7 text-xs bg-transparent">
          <Download className="mr-1 h-3 w-3" />
          Download
        </Button>
      </div>
    </div>
  )
}
