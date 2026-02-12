"use client"

import { AlertTriangle, HelpCircle, ExternalLink } from "lucide-react"
import { Button } from "@/components/ui/button"

interface PoIItem {
  id: string
  label: string
  helpUrl?: string
  resolved?: boolean
}

const mockPoIItems: PoIItem[] = [
  { id: "poi_1", label: "Certificado de origem do documento", helpUrl: "#", resolved: false },
  { id: "poi_2", label: "Assinatura do responsavel legal", helpUrl: "#", resolved: false },
  { id: "poi_3", label: "Comprovante de endereco atualizado", helpUrl: "#", resolved: true },
]

export function PoIList() {
  return (
    <div className="rounded-lg border border-amber-500/20 bg-amber-500/5 p-4">
      <div className="flex items-center gap-2 text-amber-600">
        <AlertTriangle className="h-4 w-4" />
        <h3 className="text-sm font-semibold">Evidencias pendentes</h3>
      </div>
      <p className="mt-1 text-xs text-muted-foreground">
        Faltam evidencias para concluir a verificacao. Envie os itens abaixo.
      </p>
      <ul className="mt-3 space-y-2" role="list">
        {mockPoIItems.map((item) => (
          <li
            key={item.id}
            className="flex items-center justify-between rounded-md border bg-card px-3 py-2"
          >
            <div className="flex items-center gap-2">
              <div className={`h-2 w-2 rounded-full ${item.resolved ? "bg-emerald-500" : "bg-amber-500"}`} />
              <span className={`text-sm ${item.resolved ? "text-muted-foreground line-through" : "text-foreground"}`}>
                {item.label}
              </span>
            </div>
            <div className="flex items-center gap-1">
              {item.helpUrl && (
                <Button variant="ghost" size="sm" className="h-7 text-xs" asChild>
                  <a href={item.helpUrl}>
                    <HelpCircle className="mr-1 h-3 w-3" />
                    Como resolver
                    <ExternalLink className="ml-0.5 h-2.5 w-2.5" />
                  </a>
                </Button>
              )}
            </div>
          </li>
        ))}
      </ul>
    </div>
  )
}
