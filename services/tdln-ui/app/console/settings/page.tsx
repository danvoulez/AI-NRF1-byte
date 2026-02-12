"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import { Switch } from "@/components/ui/switch"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Key,
  RotateCcw,
  Plus,
  Globe,
  Clock,
  Download,
  Trash2,
  Copy,
  Check,
  AlertTriangle,
  Eye,
  EyeOff,
  Webhook,
  Shield,
} from "lucide-react"
import { toast } from "sonner"

const mockApiKeys = [
  { id: "key_001", name: "Producao", prefix: "tdln_k_live_a1b2...", created: "2026-01-15", lastUsed: "2026-02-10T14:32:00Z", status: "active" },
  { id: "key_002", name: "Staging", prefix: "tdln_k_test_z9y8...", created: "2026-01-20", lastUsed: "2026-02-09T18:00:00Z", status: "active" },
  { id: "key_003", name: "CI/CD (rotacionada)", prefix: "tdln_k_ci_q7w8...", created: "2025-12-01", lastUsed: "2026-01-30", status: "rotated" },
]

const mockWebhookEndpoints = [
  { id: "wh_001", url: "https://api.empresa.com/tdln/events", events: ["execution.created", "execution.completed"], status: "active" },
  { id: "wh_002", url: "https://slack.empresa.com/hooks/tdln", events: ["execution.nack"], status: "active" },
]

export default function SettingsPage() {
  const [region, setRegion] = useState("br-south")
  const [retention, setRetention] = useState("30")
  const [copiedIdx, setCopiedIdx] = useState<string | null>(null)
  const [newKeyOpen, setNewKeyOpen] = useState(false)
  const [newKeyName, setNewKeyName] = useState("")
  const [newWhOpen, setNewWhOpen] = useState(false)
  const [deleteOpen, setDeleteOpen] = useState(false)

  const handleCopy = async (text: string, id: string) => {
    await navigator.clipboard.writeText(text)
    setCopiedIdx(id)
    setTimeout(() => setCopiedIdx(null), 2000)
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-foreground">Configuracoes</h1>
        <p className="mt-1 text-sm text-muted-foreground">Chaves de API, webhooks, regioes e preferencias do tenant.</p>
      </div>

      <Tabs defaultValue="keys">
        <TabsList>
          <TabsTrigger value="keys">Chaves de API</TabsTrigger>
          <TabsTrigger value="webhooks">Webhooks</TabsTrigger>
          <TabsTrigger value="region">Regiao & Retencao</TabsTrigger>
          <TabsTrigger value="export">Exportar dados</TabsTrigger>
        </TabsList>

        {/* API Keys */}
        <TabsContent value="keys" className="mt-6 space-y-4">
          <div className="flex items-center justify-between">
            <p className="text-sm text-muted-foreground">
              Gerencie chaves de API para autenticar suas integracoes. Secrets so sao exibidos uma vez na criacao.
            </p>
            <Button size="sm" onClick={() => setNewKeyOpen(true)}>
              <Plus className="mr-1.5 h-4 w-4" />
              Nova chave
            </Button>
          </div>

          <Card>
            <CardContent className="p-0">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Nome</TableHead>
                    <TableHead>Prefixo</TableHead>
                    <TableHead>Criada em</TableHead>
                    <TableHead>Ultimo uso</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead className="w-[120px]" />
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {mockApiKeys.map((key) => (
                    <TableRow key={key.id}>
                      <TableCell>
                        <span className="text-sm font-medium text-foreground">{key.name}</span>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <code className="font-mono text-xs text-muted-foreground">{key.prefix}</code>
                          <button
                            onClick={() => handleCopy(key.prefix, key.id)}
                            className="text-muted-foreground hover:text-foreground"
                            aria-label="Copiar prefixo"
                          >
                            {copiedIdx === key.id ? (
                              <Check className="h-3 w-3 text-emerald-500" />
                            ) : (
                              <Copy className="h-3 w-3" />
                            )}
                          </button>
                        </div>
                      </TableCell>
                      <TableCell>
                        <span className="text-xs text-muted-foreground">{new Date(key.created).toLocaleDateString("pt-BR")}</span>
                      </TableCell>
                      <TableCell>
                        <span className="text-xs text-muted-foreground">{new Date(key.lastUsed).toLocaleString("pt-BR")}</span>
                      </TableCell>
                      <TableCell>
                        <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${
                          key.status === "active" ? "bg-emerald-500/10 text-emerald-600" : "bg-muted text-muted-foreground"
                        }`}>
                          {key.status === "active" ? "Ativa" : "Rotacionada"}
                        </span>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-1">
                          <Button variant="ghost" size="icon" className="h-7 w-7" aria-label="Rotacionar chave" onClick={() => toast.success(`Chave "${key.name}" rotacionada. A chave antiga funcionara por mais 24h.`)}>
                            <RotateCcw className="h-3.5 w-3.5" />
                          </Button>
                          <Button variant="ghost" size="icon" className="h-7 w-7 text-destructive hover:text-destructive" aria-label="Revogar chave" onClick={() => toast.success(`Chave "${key.name}" revogada permanentemente.`)}>
                            <Trash2 className="h-3.5 w-3.5" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>

          <div className="rounded-lg border border-amber-500/20 bg-amber-500/5 p-3">
            <div className="flex items-start gap-3">
              <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-600" />
              <div>
                <p className="text-sm font-medium text-amber-600">Rotacao de chaves</p>
                <p className="text-xs text-muted-foreground">
                  Ao rotacionar, a chave antiga continuara funcionando por 24h. Atualize suas integracoes antes desse prazo.
                </p>
              </div>
            </div>
          </div>
        </TabsContent>

        {/* Webhooks */}
        <TabsContent value="webhooks" className="mt-6 space-y-4">
          <div className="flex items-center justify-between">
            <p className="text-sm text-muted-foreground">
              Receba notificacoes em tempo real sobre eventos no TDLN.
            </p>
            <Button size="sm" onClick={() => setNewWhOpen(true)}>
              <Plus className="mr-1.5 h-4 w-4" />
              Novo endpoint
            </Button>
          </div>

          {mockWebhookEndpoints.map((wh) => (
            <Card key={wh.id}>
              <CardContent className="p-4">
                <div className="flex items-start justify-between gap-4">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <Webhook className="h-4 w-4 shrink-0 text-foreground" />
                      <code className="truncate font-mono text-sm text-foreground">{wh.url}</code>
                    </div>
                    <div className="mt-2 flex flex-wrap gap-1.5">
                      {wh.events.map((ev) => (
                        <Badge key={ev} variant="secondary" className="text-xs">{ev}</Badge>
                      ))}
                    </div>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    <span className="inline-flex items-center gap-1 rounded-full bg-emerald-500/10 px-2 py-0.5 text-xs font-medium text-emerald-600">
                      Ativo
                    </span>
                    <Button variant="outline" size="sm" className="h-7 text-xs bg-transparent" onClick={() => toast.success(`Ping enviado para ${wh.url}. Status: 200 OK.`)}>
                      Testar
                    </Button>
                  </div>
                </div>

                <div className="mt-3 flex items-center gap-4 text-xs text-muted-foreground">
                  <span>Secret: whs_••••••••</span>
                  <span>Ultimas entregas: 100% sucesso</span>
                </div>
              </CardContent>
            </Card>
          ))}
        </TabsContent>

        {/* Region & Retention */}
        <TabsContent value="region" className="mt-6 space-y-6">
          <div className="grid gap-6 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-sm font-medium text-foreground">
                  <Globe className="h-4 w-4" />
                  Regiao de dados
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <p className="text-sm text-muted-foreground">
                  Define onde seus dados e execucoes sao processados e armazenados.
                </p>
                <Select value={region} onValueChange={setRegion}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="br-south">BR Brasil Sul (Sao Paulo)</SelectItem>
                    <SelectItem value="us-east">US US East (Virginia)</SelectItem>
                    <SelectItem value="eu-west">EU EU West (Frankfurt)</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  Alterar a regiao pode afetar a latencia. Dados existentes nao sao migrados automaticamente.
                </p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-sm font-medium text-foreground">
                  <Clock className="h-4 w-4" />
                  Retencao de dados
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <p className="text-sm text-muted-foreground">
                  Periodo de retencao para execucoes e evidencias. Apos o prazo, os dados sao removidos permanentemente.
                </p>
                <Select value={retention} onValueChange={setRetention}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="7">7 dias (Free)</SelectItem>
                    <SelectItem value="30">30 dias (Team)</SelectItem>
                    <SelectItem value="90">90 dias</SelectItem>
                    <SelectItem value="365">365 dias (Enterprise)</SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  Seu plano atual permite ate 30 dias de retencao.
                </p>
              </CardContent>
            </Card>
          </div>

          <Button onClick={() => toast.success("Configuracoes de regiao e retencao salvas com sucesso.")}>Salvar configuracoes</Button>
        </TabsContent>

        {/* Export */}
        <TabsContent value="export" className="mt-6 space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-sm font-medium text-foreground">
                <Download className="h-4 w-4" />
                Exportar dados
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-sm text-muted-foreground">
                Exporte todos os seus dados a qualquer momento. Sem vendor lock-in — seus dados sao seus.
              </p>
              <div className="space-y-3">
                {[
                  { label: "Execucoes e recibos", desc: "Todas as execucoes, CIDs e metadados", format: "JSON / CSV" },
                  { label: "Evidencias", desc: "Todas as evidencias indexadas (exceto protegidas)", format: "JSON" },
                  { label: "Audit log", desc: "Log completo de auditoria imutavel", format: "JSON / CSV" },
                  { label: "Politicas e regras", desc: "Configuracao de packs e regras", format: "JSON" },
                  { label: "Export completo", desc: "Todos os dados acima em um unico bundle", format: "ZIP" },
                ].map((item) => (
                  <div key={item.label} className="flex items-center justify-between rounded-lg border p-4">
                    <div>
                      <p className="text-sm font-medium text-foreground">{item.label}</p>
                      <p className="text-xs text-muted-foreground">{item.desc}</p>
                    </div>
                    <div className="flex items-center gap-3 shrink-0">
                      <Badge variant="secondary" className="text-xs">{item.format}</Badge>
                      <Button variant="outline" size="sm" className="bg-transparent" onClick={() => toast.success(`Exportacao de "${item.label}" iniciada. Voce recebera o arquivo por email.`)}>
                        <Download className="mr-1.5 h-3.5 w-3.5" />
                        Exportar
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>

          <Card className="border-destructive/20">
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-sm font-medium text-destructive">
                <Trash2 className="h-4 w-4" />
                Zona de perigo
              </CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground">
                Excluir permanentemente este tenant e todos os dados associados. Esta acao nao pode ser desfeita.
              </p>
              <Button variant="outline" size="sm" className="mt-4 border-destructive/30 text-destructive hover:bg-destructive/5 hover:text-destructive bg-transparent" onClick={() => setDeleteOpen(true)}>
                Excluir tenant
              </Button>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      {/* New API Key dialog */}
      <Dialog open={newKeyOpen} onOpenChange={setNewKeyOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Criar nova chave de API</DialogTitle>
            <DialogDescription>
              O secret sera exibido apenas uma vez. Copie e armazene em local seguro.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Label htmlFor="key-name">Nome da chave</Label>
            <Input id="key-name" placeholder="Ex: Producao, Staging, CI/CD" className="mt-1.5" value={newKeyName} onChange={(e) => setNewKeyName(e.target.value)} />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setNewKeyOpen(false)}>Cancelar</Button>
            <Button onClick={() => { setNewKeyOpen(false); setNewKeyName(""); toast.success("Chave de API criada com sucesso. Copie o secret agora.") }} disabled={!newKeyName}>
              Criar chave
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* New Webhook Endpoint dialog */}
      <Dialog open={newWhOpen} onOpenChange={setNewWhOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Novo endpoint de webhook</DialogTitle>
            <DialogDescription>
              Adicione uma URL para receber notificacoes de eventos do TDLN.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div>
              <Label htmlFor="wh-url">URL do endpoint</Label>
              <Input id="wh-url" placeholder="https://api.empresa.com/tdln/webhook" className="mt-1.5" />
            </div>
            <div>
              <Label>Eventos</Label>
              <div className="mt-1.5 flex flex-wrap gap-2">
                {["execution.created", "execution.completed", "execution.nack", "policy.updated"].map((ev) => (
                  <Badge key={ev} variant="secondary" className="cursor-pointer text-xs hover:bg-muted">{ev}</Badge>
                ))}
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setNewWhOpen(false)}>Cancelar</Button>
            <Button onClick={() => { setNewWhOpen(false); toast.success("Endpoint de webhook criado com sucesso.") }}>
              Criar endpoint
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete tenant dialog */}
      <Dialog open={deleteOpen} onOpenChange={setDeleteOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Excluir tenant permanentemente?</DialogTitle>
            <DialogDescription>
              Esta acao e irreversivel. Todos os dados, execucoes, recibos, evidencias, chaves e membros serao excluidos permanentemente. Recomendamos exportar seus dados antes de prosseguir.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteOpen(false)}>Cancelar</Button>
            <Button variant="destructive" onClick={() => { setDeleteOpen(false); toast.success("Solicitacao de exclusao registrada. Sua conta sera removida em 48h.") }}>
              Excluir permanentemente
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
