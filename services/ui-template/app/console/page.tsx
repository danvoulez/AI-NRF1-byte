"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { mockMetrics, mockExecutions } from "@/lib/mock-data"
import { BadgeEstado } from "@/components/tdln/badge-estado"
import { CIDChip } from "@/components/tdln/cid-chip"
import { Activity, TrendingUp, Clock, Plug } from "lucide-react"
import Link from "next/link"
import { BarChart, Bar, XAxis, YAxis, ResponsiveContainer, Tooltip } from "recharts"

const metricCards = [
  {
    title: "Execucoes hoje",
    value: mockMetrics.executionsToday.toLocaleString("pt-BR"),
    icon: Activity,
    change: "+12.3%",
  },
  {
    title: "Taxa ACK",
    value: `${mockMetrics.ackPercentage}%`,
    icon: TrendingUp,
    change: "+2.1%",
  },
  {
    title: "Latencia p99",
    value: `${mockMetrics.p99Latency}ms`,
    icon: Clock,
    change: "-8ms",
  },
  {
    title: "Integracoes ativas",
    value: mockMetrics.activeIntegrations.toString(),
    icon: Plug,
    change: null,
  },
]

export default function ConsoleDashboard() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-foreground">Visao Geral</h1>
        <p className="mt-1 text-sm text-muted-foreground">Metricas e atividade recente do seu tenant.</p>
      </div>

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
                <BarChart data={mockMetrics.weeklyData}>
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
              {mockExecutions.slice(0, 5).map((exec) => (
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
