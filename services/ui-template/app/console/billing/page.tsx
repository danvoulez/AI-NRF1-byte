"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Progress } from "@/components/ui/progress"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { mockInvoices } from "@/lib/mock-data"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  CreditCard,
  Download,
  ArrowUpRight,
  TrendingUp,
  BarChart3,
  Calendar,
  AlertTriangle,
  Check,
  Zap,
} from "lucide-react"
import Link from "next/link"
import { toast } from "sonner"

const usageData = {
  currentPlan: "Team",
  executions: { used: 7842, limit: 10000 },
  integrations: { used: 4, limit: -1 },
  retention: "30 dias",
  billingCycle: "Mensal",
  nextBilling: "01/03/2026",
  paymentMethod: "Visa **** 4242",
  projectedUsage: 9200,
}

const statusColors: Record<string, string> = {
  paid: "bg-emerald-500/10 text-emerald-600",
  pending: "bg-amber-500/10 text-amber-600",
  overdue: "bg-red-500/10 text-red-600",
}

const statusLabels: Record<string, string> = {
  paid: "Pago",
  pending: "Pendente",
  overdue: "Atrasado",
}

export default function BillingPage() {
  const [showUpgrade, setShowUpgrade] = useState(false)
  const [cancelOpen, setCancelOpen] = useState(false)
  const [paymentOpen, setPaymentOpen] = useState(false)
  const usagePercent = Math.round((usageData.executions.used / usageData.executions.limit) * 100)
  const projectedPercent = Math.round((usageData.projectedUsage / usageData.executions.limit) * 100)
  const isNearLimit = usagePercent >= 75

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-foreground">Faturamento</h1>
          <p className="mt-1 text-sm text-muted-foreground">Gerencie seu plano, uso e metodos de pagamento.</p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={() => toast.success("Exportacao de faturas iniciada. Voce recebera o arquivo por email.")}>
            <Download className="mr-1.5 h-4 w-4" />
            Exportar faturas
          </Button>
          <Button size="sm" onClick={() => setShowUpgrade(!showUpgrade)}>
            <ArrowUpRight className="mr-1.5 h-4 w-4" />
            Upgrade
          </Button>
        </div>
      </div>

      {/* Upsell banner when near limit */}
      {isNearLimit && (
        <div className="flex items-center gap-4 rounded-xl border border-amber-500/20 bg-amber-500/5 p-4">
          <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-amber-500/10">
            <AlertTriangle className="h-5 w-5 text-amber-600" />
          </div>
          <div className="flex-1">
            <p className="text-sm font-medium text-foreground">
              Voce esta usando {usagePercent}% do seu limite mensal
            </p>
            <p className="text-xs text-muted-foreground">
              Projecao: {usageData.projectedUsage.toLocaleString("pt-BR")} execucoes ate o fim do ciclo. Considere fazer upgrade para nao ter interrupcoes.
            </p>
          </div>
          <Button size="sm" variant="outline" className="shrink-0 bg-transparent" asChild>
            <Link href="/pricing">
              <Zap className="mr-1.5 h-3.5 w-3.5" />
              Ver planos
            </Link>
          </Button>
        </div>
      )}

      {/* Plan + Usage Cards */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Plano atual</CardTitle>
            <CreditCard className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <span className="text-2xl font-bold text-foreground">{usageData.currentPlan}</span>
              <Badge variant="secondary" className="text-xs">Ativo</Badge>
            </div>
            <p className="mt-1 text-xs text-muted-foreground">R$ 990/mes</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Execucoes</CardTitle>
            <BarChart3 className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-foreground">
              {usageData.executions.used.toLocaleString("pt-BR")}
            </div>
            <p className="mt-1 text-xs text-muted-foreground">
              de {usageData.executions.limit.toLocaleString("pt-BR")} execucoes/mes
            </p>
            <Progress value={usagePercent} className="mt-2 h-1.5" />
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Projecao</CardTitle>
            <TrendingUp className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-foreground">
              {usageData.projectedUsage.toLocaleString("pt-BR")}
            </div>
            <p className="mt-1 text-xs text-muted-foreground">
              estimativa para este ciclo ({projectedPercent}%)
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Proxima fatura</CardTitle>
            <Calendar className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-foreground">{usageData.nextBilling}</div>
            <p className="mt-1 text-xs text-muted-foreground">Ciclo {usageData.billingCycle.toLowerCase()}</p>
          </CardContent>
        </Card>
      </div>

      {/* Payment method + Plan details */}
      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="text-sm font-medium text-foreground">Metodo de pagamento</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-4 rounded-lg border p-4">
              <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-muted">
                <CreditCard className="h-6 w-6 text-foreground" />
              </div>
              <div className="flex-1">
                <p className="text-sm font-medium text-foreground">{usageData.paymentMethod}</p>
                <p className="text-xs text-muted-foreground">Expira em 12/2028</p>
              </div>
              <Button variant="outline" size="sm" onClick={() => setPaymentOpen(true)}>Alterar</Button>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-sm font-medium text-foreground">Detalhes do plano</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3 text-sm">
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Execucoes/mes</span>
                <span className="font-medium text-foreground">{usageData.executions.limit.toLocaleString("pt-BR")}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Integracoes</span>
                <span className="font-medium text-foreground">Ilimitadas</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Retencao</span>
                <span className="font-medium text-foreground">{usageData.retention}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">RBAC</span>
                <span className="inline-flex items-center gap-1 font-medium text-emerald-600">
                  <Check className="h-3.5 w-3.5" />
                  Incluido
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Webhooks</span>
                <span className="inline-flex items-center gap-1 font-medium text-emerald-600">
                  <Check className="h-3.5 w-3.5" />
                  Incluido
                </span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Invoices */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle className="text-sm font-medium text-foreground">Historico de faturas</CardTitle>
          <Button variant="outline" size="sm" className="h-8 text-xs bg-transparent" onClick={() => toast.success("Exportacao iniciada. Voce recebera todas as faturas por email.")}>
            <Download className="mr-1.5 h-3 w-3" />
            Exportar tudo
          </Button>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Fatura</TableHead>
                <TableHead>Data</TableHead>
                <TableHead>Valor</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="text-right">Acoes</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {mockInvoices.map((inv) => (
                <TableRow key={inv.id}>
                  <TableCell>
                    <span className="font-mono text-sm text-foreground">{inv.id}</span>
                  </TableCell>
                  <TableCell>
                    <span className="text-sm text-muted-foreground">
                      {new Date(inv.date).toLocaleDateString("pt-BR")}
                    </span>
                  </TableCell>
                  <TableCell>
                    <span className="text-sm font-medium text-foreground">{inv.amount}</span>
                  </TableCell>
                  <TableCell>
                    <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${statusColors[inv.status]}`}>
                      {statusLabels[inv.status]}
                    </span>
                  </TableCell>
                  <TableCell className="text-right">
                    <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={() => toast.success(`Download da fatura ${inv.id} iniciado.`)}>
                      <Download className="mr-1 h-3 w-3" />
                      PDF
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {/* Cancel section */}
      <Card className="border-destructive/20">
        <CardContent className="flex items-center justify-between p-4">
          <div>
            <p className="text-sm font-medium text-foreground">Cancelar plano</p>
            <p className="text-xs text-muted-foreground">
              Ao cancelar, voce voltara para o plano Free no proximo ciclo. Suas verificacoes existentes continuarao acessiveis.
            </p>
          </div>
          <Button variant="outline" size="sm" className="shrink-0 border-destructive/30 text-destructive hover:bg-destructive/5 hover:text-destructive bg-transparent" onClick={() => setCancelOpen(true)}>
            Cancelar plano
          </Button>
        </CardContent>
      </Card>

      {/* Cancel confirmation dialog */}
      <Dialog open={cancelOpen} onOpenChange={setCancelOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Cancelar plano {usageData.currentPlan}?</DialogTitle>
            <DialogDescription>
              Ao confirmar, seu plano sera reduzido para Free no proximo ciclo de faturamento ({usageData.nextBilling}). Suas execucoes e recibos existentes continuarao acessiveis ate o fim do periodo de retencao.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCancelOpen(false)}>Manter plano</Button>
            <Button variant="destructive" onClick={() => { setCancelOpen(false); toast.success("Cancelamento agendado para o proximo ciclo.") }}>
              Confirmar cancelamento
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Change payment dialog */}
      <Dialog open={paymentOpen} onOpenChange={setPaymentOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Alterar metodo de pagamento</DialogTitle>
            <DialogDescription>
              Atualize o cartao de credito associado a sua conta. A alteracao sera aplicada na proxima fatura.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div>
              <label htmlFor="card-number" className="text-sm font-medium text-foreground">Numero do cartao</label>
              <input id="card-number" placeholder="4242 4242 4242 4242" className="mt-1.5 w-full rounded-md border bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label htmlFor="expiry" className="text-sm font-medium text-foreground">Validade</label>
                <input id="expiry" placeholder="MM/AA" className="mt-1.5 w-full rounded-md border bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" />
              </div>
              <div>
                <label htmlFor="cvc" className="text-sm font-medium text-foreground">CVC</label>
                <input id="cvc" placeholder="123" className="mt-1.5 w-full rounded-md border bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" />
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setPaymentOpen(false)}>Cancelar</Button>
            <Button onClick={() => { setPaymentOpen(false); toast.success("Metodo de pagamento atualizado com sucesso.") }}>
              Salvar cartao
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
