"use client"

import { useState } from "react"
import { cn } from "@/lib/utils"
import { Copy, Check } from "lucide-react"

export function CIDChip({ cid, className }: { cid: string; className?: string }) {
  const [copied, setCopied] = useState(false)
  const truncated = `${cid.slice(0, 8)}...${cid.slice(-6)}`

  const handleCopy = async () => {
    await navigator.clipboard.writeText(cid)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <button
      onClick={handleCopy}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-md bg-muted px-2 py-1 font-mono text-xs text-muted-foreground transition-colors hover:bg-muted/80 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        className
      )}
      title={cid}
      aria-label={`CID: ${cid}. Clique para copiar.`}
    >
      <span className="select-all">{truncated}</span>
      {copied ? <Check className="h-3 w-3 text-emerald-500" /> : <Copy className="h-3 w-3" />}
    </button>
  )
}
