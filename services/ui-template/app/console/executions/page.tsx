"use client"

import { useState } from "react"
import Link from "next/link"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover"
import { Calendar } from "@/components/ui/calendar"
import { BadgeEstado } from "@/components/tdln/badge-estado"
import { CIDChip } from "@/components/tdln/cid-chip"
import { mockExecutions, type ExecutionState } from "@/lib/mock-data"
import { Search, Download, ArrowRight, FileText, Plus, CalendarIcon } from "lucide-react"
import { format } from "date-fns"
import { toast } from "sonner"

export default function ExecutionsPage() {
  const [stateFilter, setStateFilter] = useState<string>("all")
  const [searchQuery, setSearchQuery] = useState("")
  const [dateRange, setDateRange] = useState<{ from: Date | undefined; to: Date | undefined }>({
    from: undefined,
    to: undefined,
  })

  const filtered = mockExecutions.filter((exec) => {
    // State filter
    if (stateFilter !== "all" && exec.state !== stateFilter) return false
    
    // Search filter (title, CID, origin, integration, b3:)
    if (searchQuery) {
      const query = searchQuery.toLowerCase()
      const matchesTitle = exec.title.toLowerCase().includes(query)
      const matchesCID = exec.cid.toLowerCase().includes(query)
      const matchesOrigin = exec.origin.toLowerCase().includes(query)
      const matchesIntegration = exec.integration.toLowerCase().includes(query)
      const matchesB3 = query.startsWith("b3:") && exec.cid.includes(query.slice(3))
      
      if (!matchesTitle && !matchesCID && !matchesOrigin && !matchesIntegration && !matchesB3) {
        return false
      }
    }
    
    // Date range filter
    if (dateRange.from || dateRange.to) {
      const execDate = new Date(exec.timestamp)
      if (dateRange.from && execDate < dateRange.from) return false
      if (dateRange.to && execDate > dateRange.to) return false
    }
    
    return true
  })

  const handleExportCSV = () => {
    // Generate CSV
    const headers = ["Estado", "CID", "Titulo", "Origem", "Integracao", "Timestamp"]
    const rows = filtered.map((exec) => [
      exec.state,
      exec.cid,
      exec.title,
      exec.origin,
      exec.integration,
      new Date(exec.timestamp).toISOString(),
    ])
    
    const csvContent = [
      headers.join(","),
      ...rows.map((row) => row.map((cell) => `"${cell}"`).join(",")),
    ].join("\n")
    
    // Download
    const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" })
    const link = document.createElement("a")
    const url = URL.createObjectURL(blob)
    link.setAttribute("href", url)
    link.setAttribute("download", `execucoes-${format(new Date(), "yyyy-MM-dd")}.csv`)
    link.style.visibility = "hidden"
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    
    toast.success("CSV exportado com sucesso")
  }

  const isEmpty = mockExecutions.length === 0

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-foreground">Execucoes</h1>
          <p className="mt-1 text-sm text-muted-foreground">Historico completo de decisoes e recibos verificaveis.</p>
        </div>
        <Button asChild>
          <Link href="/onboarding">
            <Plus className="mr-1.5 h-4 w-4" />
            Nova execucao
          </Link>
        </Button>
      </div>

      {isEmpty ? (
        /* Empty state */
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-16">
            <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-muted">
              <FileText className="h-8 w-8 text-muted-foreground" />
            </div>
            <h2 className="mt-4 text-lg font-semibold text-foreground">Sem execucoes ainda</h2>
            <p className="mt-1 text-sm text-muted-foreground">Gere seu primeiro recibo verificavel para comecar.</p>
            <Button className="mt-6" asChild>
              <Link href="/onboarding">
                Gere seu primeiro recibo
                <ArrowRight className="ml-2 h-4 w-4" />
              </Link>
            </Button>
          </CardContent>
        </Card>
      ) : (
        <>
          {/* Filters */}
          <div className="flex flex-col gap-3 sm:flex-row sm:flex-wrap">
            <div className="relative flex-1 max-w-sm">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                placeholder="Buscar por titulo, CID, origin, integration, b3:..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-9"
                aria-label="Filtrar execucoes"
              />
            </div>
            <Select value={stateFilter} onValueChange={setStateFilter}>
              <SelectTrigger className="w-[140px]" aria-label="Filtrar por estado">
                <SelectValue placeholder="Estado" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">Todos</SelectItem>
                <SelectItem value="ACK">ACK</SelectItem>
                <SelectItem value="NACK">NACK</SelectItem>
                <SelectItem value="ASK">ASK</SelectItem>
              </SelectContent>
            </Select>
            <Popover>
              <PopoverTrigger asChild>
                <Button variant="outline" size="sm" className="h-10 justify-start text-left font-normal bg-transparent">
                  <CalendarIcon className="mr-2 h-4 w-4" />
                  {dateRange.from ? (
                    dateRange.to ? (
                      <>
                        {format(dateRange.from, "dd/MM/yy")} - {format(dateRange.to, "dd/MM/yy")}
                      </>
                    ) : (
                      format(dateRange.from, "dd/MM/yyyy")
                    )
                  ) : (
                    <span>Periodo</span>
                  )}
                </Button>
              </PopoverTrigger>
              <PopoverContent className="w-auto p-0" align="start">
                <Calendar
                  mode="range"
                  selected={{ from: dateRange.from, to: dateRange.to }}
                  onSelect={(range) => setDateRange({ from: range?.from, to: range?.to })}
                  numberOfMonths={2}
                />
                {(dateRange.from || dateRange.to) && (
                  <div className="border-t p-2">
                    <Button
                      variant="ghost"
                      size="sm"
                      className="w-full"
                      onClick={() => setDateRange({ from: undefined, to: undefined })}
                    >
                      Limpar
                    </Button>
                  </div>
                )}
              </PopoverContent>
            </Popover>
            <Button variant="outline" size="sm" className="h-10 bg-transparent" onClick={handleExportCSV}>
              <Download className="mr-1.5 h-4 w-4" />
              Exportar CSV
            </Button>
          </div>

          {/* Table */}
          <Card>
            <CardContent className="p-0">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[100px]">Estado</TableHead>
                    <TableHead>CID</TableHead>
                    <TableHead>Titulo / Origem</TableHead>
                    <TableHead>Integracao</TableHead>
                    <TableHead className="text-right">Timestamp</TableHead>
                    <TableHead className="w-[80px]" />
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {filtered.map((exec) => (
                    <TableRow key={exec.id} className="transition-colors hover:bg-muted/50">
                      <TableCell>
                        <BadgeEstado state={exec.state} size="sm" />
                      </TableCell>
                      <TableCell>
                        <CIDChip cid={exec.cid} />
                      </TableCell>
                      <TableCell>
                        <div>
                          <p className="text-sm font-medium text-foreground">{exec.title}</p>
                          <p className="text-xs text-muted-foreground">{exec.origin}</p>
                        </div>
                      </TableCell>
                      <TableCell>
                        <span className="text-sm text-muted-foreground">{exec.integration}</span>
                      </TableCell>
                      <TableCell className="text-right">
                        <span className="text-sm text-muted-foreground">
                          {new Date(exec.timestamp).toLocaleString("pt-BR")}
                        </span>
                      </TableCell>
                      <TableCell>
                        <Button variant="ghost" size="sm" className="h-8" asChild>
                          <Link href={`/console/r/${exec.cid}`}>
                            Ver
                            <ArrowRight className="ml-1 h-3 w-3" />
                          </Link>
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>

          {/* Pagination */}
          <div className="flex items-center justify-between text-sm text-muted-foreground">
            <span>Exibindo {filtered.length} de {mockExecutions.length} execucoes</span>
            <div className="flex gap-2">
              <Button variant="outline" size="sm" disabled>Anterior</Button>
              <Button variant="outline" size="sm" disabled>Proxima</Button>
            </div>
          </div>
        </>
      )}
    </div>
  )
}
