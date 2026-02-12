"use client"

import { useState, useEffect } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { fetchMetrics, fetchExecutions, runPipeline, type Execution } from "@/lib/api"
import { mockMetrics, mockExecutions } from "@/lib/mock-data"
import { BadgeEstado } from "@/components/tdln/badge-estado"
import { CIDChip } from "@/components/tdln/cid-chip"
import { Activity, TrendingUp, Clock, Plug, Play, Loader2 } from "lucide-react"
import Link from "next/link"
import { BarChart, Bar, XAxis, YAxis, ResponsiveContainer, Tooltip } from "recharts"
import { toast } from "sonner"

function buildMetricCards(metrics: typeof mockMetrics) {
  return [
    {
      title: "Execucoes hoje",
      value: metrics.executionsToday.toLocaleString("pt-BR"),
      icon: Activity,
      change: "+12.3%",
    },
    {
      title: "Taxa ACK",
      value: `${metrics.ackPercentage}%`,
      icon: TrendingUp,
      change: "+2.1%",
    },
    {
      title: "Latencia p99",
      value: `${metrics.p99Latency}ms`,
      icon: Clock,
      change: "-8ms",
    },
    {
      title: "Integracoes ativas",
      value: metrics.activeIntegrations.toString(),
      icon: Plug,
      change: null,
    },
  ]
}

export default function ConsoleDashboard() {
  const [metrics, setMetrics] = useState(mockMetrics)
  const [executions, setExecutions] = useState<Execution[]>(mockExecutions)

  useEffect(() => {
    fetchMetrics().then(setMetrics)
    fetchExecutions().then(setExecutions)
  }, [])

  const [showRun, setShowRun] = useState(false)
  const [runTitle, setRunTitle] = useState("")
  const [runData, setRunData] = useState("")
  const [running, setRunning] = useState(false)

  const handleRun = async () => {
    if (!runTitle.trim()) return
    setRunning(true)
    try {
      const resp = await runPipeline({
        tenant: "demo",
        manifest: {
          v: "1",
          name: runTitle,
          version: "1.0.0",
          pipeline: [
            { step_id: "intake", kind: "cap-intake", version: "*", config: { mappings: [{ from: "data", to: "payload" }] } },
          ],
        },
        env: { data: runData || "payload" },
      })
      toast.success(`${resp.verdict} — ${resp.receipt_cid.slice(0, 24)}...`)
      setShowRun(false)
      setRunTitle("")
      setRunData("")
      fetchMetrics().then(setMetrics)
      fetchExecutions().then(setExecutions)
    } catch (e: any) {
      toast.error(e.message || "Pipeline failed")
    } finally {
      setRunning(false)
    }
  }

  const metricCards = buildMetricCards(metrics)
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-foreground">Visao Geral</h1>
          <p className="mt-1 text-sm text-muted-foreground">Metricas e atividade recente do seu tenant.</p>
        </div>
        <Button onClick={() => setShowRun(!showRun)} size="sm">
          <Play className="mr-1.5 h-4 w-4" />
          Executar Pipeline
        </Button>
      </div>

      {showRun && (
        <Card>
          <CardContent className="pt-6">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-end">
              <div className="flex-1">
                <label className="text-xs font-medium text-muted-foreground">Titulo</label>
                <Input
                  placeholder="Ex: Transacao financeira 4821"
                  value={runTitle}
                  onChange={(e) => setRunTitle(e.target.value)}
                  className="mt-1"
                />
              </div>
              <div className="flex-1">
                <label className="text-xs font-medium text-muted-foreground">Dados (payload)</label>
                <Input
                  placeholder="Ex: hello world"
                  value={runData}
                  onChange={(e) => setRunData(e.target.value)}
                  className="mt-1"
                />
              </div>
              <Button onClick={handleRun} disabled={running || !runTitle.trim()}>
                {running ? <Loader2 className="mr-1.5 h-4 w-4 animate-spin" /> : <Play className="mr-1.5 h-4 w-4" />}
                {running ? "Executando..." : "Executar"}
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Metric Cards */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {metricCards.map((metric) => (
          <Card key={metric.title} className="relative overflow-hidden transition-shadow hover:shadow-md">
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">{metric.title}</CardTitle>
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-muted transition-colors group-hover:bg-muted/80">
                <metric.icon className="h-4 w-4 text-foreground" />
              </div>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold tracking-tight text-foreground">{metric.value}</div>
              {metric.change && (
                <p className="mt-1.5 text-xs font-medium text-emerald-600">{metric.change} vs ontem</p>
              )}
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Chart + Recent executions */}
      <div className="grid gap-6 lg:grid-cols-5">
        {/* Weekly chart */}
        <Card className="lg:col-span-3">
          <CardHeader>
            <CardTitle className="text-sm font-medium text-foreground">Execucoes esta semana</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={metrics.weeklyData}>
                  <XAxis dataKey="day" axisLine={false} tickLine={false} className="text-xs" />
                  <YAxis axisLine={false} tickLine={false} className="text-xs" />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: "hsl(var(--card))",
                      border: "1px solid hsl(var(--border))",
                      borderRadius: "0.5rem",
                      fontSize: "0.75rem",
                    }}
                  />
                  <Bar dataKey="executions" fill="hsl(var(--foreground))" radius={[4, 4, 0, 0]} name="Total" />
                  <Bar dataKey="ack" fill="hsl(var(--ack))" radius={[4, 4, 0, 0]} name="ACK" />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </CardContent>
        </Card>

        {/* Recent executions */}
        <Card className="lg:col-span-2">
          <CardHeader className="flex flex-row items-center justify-between">
            <CardTitle className="text-sm font-medium text-foreground">Ultimos CIDs</CardTitle>
            <Link href="/console/executions" className="text-xs text-muted-foreground hover:text-foreground">
              Ver todos
            </Link>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {executions.slice(0, 5).map((exec) => (
                <Link
                  key={exec.id}
                  href={`/console/r/${exec.cid}`}
                  className="flex items-center justify-between rounded-md p-2 transition-colors hover:bg-muted"
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <BadgeEstado state={exec.state} size="sm" />
                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium text-foreground">{exec.title}</p>
                      <CIDChip cid={exec.cid} className="mt-0.5" />
                    </div>
                  </div>
                </Link>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Quick actions */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium text-foreground">Acoes rapidas</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-3">
            <Link href="/console/integrations" className="rounded-lg border bg-card px-4 py-3 text-sm font-medium text-foreground transition-colors hover:bg-muted">
              Configurar integracao
            </Link>
            <Link href="/console/policies" className="rounded-lg border bg-card px-4 py-3 text-sm font-medium text-foreground transition-colors hover:bg-muted">
              Gerenciar politicas
            </Link>
            <Link href="/console/team" className="rounded-lg border bg-card px-4 py-3 text-sm font-medium text-foreground transition-colors hover:bg-muted">
              Convidar membro
            </Link>
            <Link href="/console/settings" className="rounded-lg border bg-card px-4 py-3 text-sm font-medium text-foreground transition-colors hover:bg-muted">
              Chaves de API
            </Link>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
