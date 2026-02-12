"use client"

import { useState, useEffect } from "react"
import { Card, CardContent } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { fetchAuditLog, type AuditEntry } from "@/lib/api"
import { Search, Download, Shield } from "lucide-react"

const actionLabels: Record<string, string> = {
  "execution.create": "Execucao criada",
  "execution.download_bundle": "Bundle baixado",
  "policy.update": "Politica atualizada",
  "team.invite": "Convite enviado",
  "key.rotate": "Chave rotacionada",
  "integration.create": "Integracao criada",
}

export default function AuditsPage() {
  const [searchQuery, setSearchQuery] = useState("")
  const [auditLog, setAuditLog] = useState<AuditEntry[]>([])

  useEffect(() => {
    fetchAuditLog().then(setAuditLog)
  }, [])

  const filtered = auditLog.filter((entry) =>
    searchQuery
      ? entry.user.includes(searchQuery) || entry.action.includes(searchQuery) || entry.target.includes(searchQuery)
      : true
  )

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-foreground">Auditorias</h1>
          <p className="mt-1 text-sm text-muted-foreground">Log imutavel de todas as acoes no tenant.</p>
        </div>
        <Button variant="outline" size="sm" onClick={() => { const el = document.createElement("a"); el.setAttribute("download", "tdln-audit-log.csv"); el.click(); }}>
          <Download className="mr-1.5 h-4 w-4" />
          Exportar CSV
        </Button>
      </div>

      <div className="flex gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Buscar por usuario, acao, alvo..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
            aria-label="Filtrar auditorias"
          />
        </div>
      </div>

      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Usuario</TableHead>
                <TableHead>Acao</TableHead>
                <TableHead>Alvo</TableHead>
                <TableHead>CID</TableHead>
                <TableHead>IP</TableHead>
                <TableHead className="text-right">Timestamp</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filtered.map((entry) => (
                <TableRow key={entry.id}>
                  <TableCell>
                    <span className="text-sm font-medium text-foreground">{entry.user}</span>
                  </TableCell>
                  <TableCell>
                    <Badge variant="secondary" className="text-xs">
                      {actionLabels[entry.action] || entry.action}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <span className="font-mono text-xs text-muted-foreground">{entry.target}</span>
                  </TableCell>
                  <TableCell>
                    {entry.cid ? (
                      <span className="font-mono text-xs text-muted-foreground">{entry.cid}</span>
                    ) : (
                      <span className="text-xs text-muted-foreground">—</span>
                    )}
                  </TableCell>
                  <TableCell>
                    <span className="font-mono text-xs text-muted-foreground">{entry.ip}</span>
                  </TableCell>
                  <TableCell className="text-right">
                    <span className="text-xs text-muted-foreground">
                      {new Date(entry.timestamp).toLocaleString("pt-BR")}
                    </span>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <div className="flex items-center justify-between text-sm text-muted-foreground">
        <div className="flex items-center gap-2">
          <Shield className="h-4 w-4" />
          <span>Log imutavel — registros nao podem ser alterados ou excluidos</span>
        </div>
        <span>{filtered.length} registros</span>
      </div>
    </div>
  )
}
