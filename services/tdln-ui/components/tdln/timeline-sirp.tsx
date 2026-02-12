"use client"

import { useState } from "react"
import { cn } from "@/lib/utils"
import type { SIRPNode } from "@/lib/mock-data"
import { CheckCircle2, Send, Truck, Cpu, Flag } from "lucide-react"

const stepIcons = {
  INTENT: Send,
  DELIVERY: Truck,
  EXECUTION: Cpu,
  RESULT: Flag,
}

const stepLabels = {
  INTENT: "Intent",
  DELIVERY: "Delivery",
  EXECUTION: "Execution",
  RESULT: "Result",
}

export function TimelineSIRP({ nodes }: { nodes: SIRPNode[] }) {
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null)

  return (
    <div className="flex flex-col gap-4">
      {/* Timeline visualization */}
      <div className="flex items-center justify-between" role="list" aria-label="SIRP Timeline">
        {nodes.map((node, i) => {
          const Icon = stepIcons[node.step]
          const isSelected = selectedIndex === i

          return (
            <div key={node.step} className="flex items-center flex-1 last:flex-none" role="listitem">
              <button
                onClick={() => setSelectedIndex(isSelected ? null : i)}
                className={cn(
                  "relative flex flex-col items-center gap-2 rounded-lg p-3 transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                  isSelected ? "bg-muted" : "hover:bg-muted/50"
                )}
                aria-expanded={isSelected}
                aria-label={`${stepLabels[node.step]}: ${node.verified ? "Verificado" : "Nao verificado"}`}
              >
                <div className={cn(
                  "flex h-10 w-10 items-center justify-center rounded-full border-2 transition-colors",
                  node.verified ? "border-emerald-500 bg-emerald-500/10 text-emerald-600" : "border-muted-foreground/30 bg-muted text-muted-foreground"
                )}>
                  <Icon className="h-4 w-4" />
                </div>
                <span className="text-xs font-medium text-foreground">{stepLabels[node.step]}</span>
                {node.verified && (
                  <CheckCircle2 className="absolute -right-0.5 -top-0.5 h-4 w-4 text-emerald-500" aria-hidden="true" />
                )}
              </button>
              {i < nodes.length - 1 && (
                <div className={cn(
                  "h-0.5 flex-1 mx-1",
                  node.verified ? "bg-emerald-500/40" : "bg-border"
                )} aria-hidden="true" />
              )}
            </div>
          )
        })}
      </div>

      {/* Detail panel */}
      {selectedIndex !== null && (
        <div className="rounded-lg border bg-card p-4 animate-in fade-in-0 slide-in-from-top-2 duration-150">
          <div className="grid gap-3 text-sm">
            <div className="flex items-center justify-between">
              <span className="font-medium text-foreground">{stepLabels[nodes[selectedIndex].step]}</span>
              {nodes[selectedIndex].verified && (
                <span className="inline-flex items-center gap-1 text-xs text-emerald-600">
                  <CheckCircle2 className="h-3 w-3" />
                  Verificado
                </span>
              )}
            </div>
            <div className="grid gap-2 text-muted-foreground">
              <div className="flex justify-between">
                <span>Assinante</span>
                <span className="font-mono text-xs text-foreground">{nodes[selectedIndex].signer}</span>
              </div>
              <div className="flex justify-between">
                <span>Algoritmo</span>
                <span className="font-mono text-xs text-foreground">{nodes[selectedIndex].algorithm}</span>
              </div>
              <div className="flex justify-between">
                <span>Hash</span>
                <span className="font-mono text-xs text-foreground">{nodes[selectedIndex].hash}</span>
              </div>
              <div className="flex justify-between">
                <span>Timestamp</span>
                <span className="text-xs text-foreground">{new Date(nodes[selectedIndex].timestamp).toLocaleString("pt-BR")}</span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
