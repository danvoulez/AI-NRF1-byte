"use client"

import { useState, useEffect } from "react"
import Link from "next/link"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { BadgeEstado } from "@/components/tdln/badge-estado"
import { CIDChip } from "@/components/tdln/cid-chip"
import { TimelineSIRP } from "@/components/tdln/timeline-sirp"
import { CardProva } from "@/components/tdln/card-prova"
import { PoIList } from "@/components/tdln/poi-list"
import { fetchReceipt, type Execution, type SIRPNode, type Proof, type Evidence } from "@/lib/api"
import { mockExecutions, mockSIRPNodes, mockProofs, mockEvidence } from "@/lib/mock-data"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Copy,
  Download,
  ExternalLink,
  QrCode,
  Printer,
  Shield,
  ArrowLeft,
  WifiOff,
  RefreshCw,
  Lock,
  Check,
  AlertTriangle,
  XCircle,
} from "lucide-react"
import { toast } from "sonner"
export default function ReceiptDetailPage({ params }: { params: { cid: string } }) {
  const { cid } = params
  const [copiedLink, setCopiedLink] = useState(false)
  const [qrOpen, setQrOpen] = useState(false)
  const [execution, setExecution] = useState<Execution>(mockExecutions.find((e) => e.cid === cid) || mockExecutions[0])
  const [sirpNodes, setSirpNodes] = useState<SIRPNode[]>(mockSIRPNodes)
  const [proofs, setProofs] = useState<Proof[]>(mockProofs)
  const [evidence, setEvidence] = useState<Evidence[]>(mockEvidence)

  useEffect(() => {
    fetchReceipt(cid).then((data) => {
      setExecution(data.execution)
      setSirpNodes(data.sirp)
      setProofs(data.proofs)
      setEvidence(data.evidence)
    })
  }, [cid])

  const isASK = execution.state === "ASK"
  const isNACK = execution.state === "NACK"

  const handleCopyLink = async () => {
    await navigator.clipboard.writeText(`${window.location.origin}/console/r/${cid}`)
    setCopiedLink(true)
    setTimeout(() => setCopiedLink(false), 2000)
  }

  return (
    <div className="space-y-6">
      {/* Back navigation */}
      <Button variant="ghost" size="sm" asChild>
        <Link href="/console/executions">
          <ArrowLeft className="mr-1 h-4 w-4" />
          Voltar para execucoes
        </Link>
      </Button>

      {/* State Banner */}
      <div className={`rounded-xl border p-6 transition-colors ${
        execution.state === "ACK" ? "border-emerald-500/20 bg-emerald-500/5" :
        execution.state === "ASK" ? "border-amber-500/20 bg-amber-500/5" :
        "border-red-500/20 bg-red-500/5"
      }`}>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex items-start gap-4">
            <div className={`flex h-12 w-12 shrink-0 items-center justify-center rounded-xl ${
              execution.state === "ACK" ? "bg-emerald-500/10" :
              execution.state === "ASK" ? "bg-amber-500/10" :
              "bg-red-500/10"
            }`}>
              {execution.state === "ACK" && <Check className="h-6 w-6 text-emerald-600" />}
              {execution.state === "ASK" && <AlertTriangle className="h-6 w-6 text-amber-600" />}
              {execution.state === "NACK" && <XCircle className="h-6 w-6 text-red-600" />}
            </div>
            <div className="flex-1">
              <div className="flex items-center gap-2">
                <h1 className="text-lg font-bold tracking-tight text-foreground">{execution.title}</h1>
                <BadgeEstado state={execution.state} size="default" />
              </div>
              <p className="mt-1 text-sm leading-relaxed text-muted-foreground">
                {execution.state === "ACK" && "Decisao aceita — verificacao concluida ✓"}
                {execution.state === "ASK" && `Faltam evidencias para concluir. Envie: ${execution.title}`}
                {execution.state === "NACK" && `Regras nao atendidas: ${execution.title} (politica ack-policy-id)`}
              </p>
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="outline" size="sm" className="bg-transparent" onClick={handleCopyLink}>
              {copiedLink ? <Check className="mr-1.5 h-3.5 w-3.5" /> : <Copy className="mr-1.5 h-3.5 w-3.5" />}
              {copiedLink ? "Copiado" : "Copiar Link"}
            </Button>
            <Button variant="outline" size="sm" className="bg-transparent" onClick={() => toast.success("Download do bundle .zip iniciado.")}>
              <Download className="mr-1.5 h-3.5 w-3.5" />
              Baixar Bundle
            </Button>
          </div>
        </div>
        <div className="mt-4">
          <span className="text-xs text-muted-foreground">CID</span>
          <div className="mt-1">
            <CIDChip cid={execution.cid} />
          </div>
        </div>
      </div>

      {/* Main content: 3 columns on wide, stacked on mobile */}
      <div className="grid gap-6 lg:grid-cols-12">
        {/* Left column: Timeline */}
        <div className="lg:col-span-4 space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium text-foreground">Timeline SIRP</CardTitle>
            </CardHeader>
            <CardContent>
              <TimelineSIRP nodes={sirpNodes} />
            </CardContent>
          </Card>

          {/* Trails */}
          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium text-foreground">Rastreabilidade</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3 text-sm">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Requisitante</span>
                  <span className="font-mono text-xs text-foreground">maria@empresa.com</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Origem</span>
                  <span className="text-xs text-foreground">{execution.origin}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Integracao</span>
                  <span className="text-xs text-foreground">{execution.integration}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Idempotency Key</span>
                  <span className="font-mono text-xs text-foreground">idk_a1b2c3</span>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Center column: Proofs */}
        <div className="lg:col-span-4 space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium text-foreground">Provas</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {proofs.map((proof) => (
                <CardProva key={proof.type} proof={proof} />
              ))}
            </CardContent>
          </Card>
        </div>

        {/* Right column: Evidence + ASK/NACK specific */}
        <div className="lg:col-span-4 space-y-6">
          {/* Evidence */}
          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium text-foreground">Evidencias</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                {evidence.map((ev) => (
                  <div key={ev.cid} className="rounded-lg border p-3">
                    <div className="flex items-center justify-between">
                      <CIDChip cid={ev.cid} />
                      <span className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium ${
                        ev.status === "fetched" ? "bg-emerald-500/10 text-emerald-600" :
                        ev.status === "protected" ? "bg-muted text-muted-foreground" :
                        ev.status === "pending" ? "bg-amber-500/10 text-amber-600" :
                        "bg-red-500/10 text-red-600"
                      }`}>
                        {ev.status === "protected" && <Lock className="h-3 w-3" />}
                        {ev.status === "fetched" ? "Obtido" : ev.status === "protected" ? "AEAD/protegido" : ev.status === "pending" ? "Pendente" : "Falha"}
                      </span>
                    </div>
                    <p className="mt-1.5 truncate text-xs text-muted-foreground">{ev.url}</p>
                    {ev.status === "failed" && (
                      <div className="mt-2">
                        <p className="text-xs text-red-600">Falha ao obter evidencia.</p>
                        <Button variant="outline" size="sm" className="mt-1 h-7 text-xs bg-transparent" onClick={() => toast.info("Tentando obter evidencia novamente...")}>
                          <RefreshCw className="mr-1 h-3 w-3" />
                          Tentar Novamente
                        </Button>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>

          {/* ASK - PoI */}
          {isASK && <PoIList />}

          {/* NACK explanation */}
          {isNACK && (
            <Card className="border-red-500/20">
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-sm font-medium text-red-600">
                  <XCircle className="h-4 w-4" />
                  Regras nao atendidas
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-2">
                  <div className="rounded-md border border-red-500/10 bg-red-500/5 p-3">
                    <p className="text-sm text-foreground">Threshold de risco excedido (0.92 {">"} 0.85)</p>
                    <p className="text-xs text-muted-foreground">Politica: Sanctions AML v2.3.1</p>
                    <Link href="/console/policies" className="mt-1 inline-flex items-center text-xs font-medium text-foreground hover:underline">
                      Ver politica
                      <ExternalLink className="ml-1 h-3 w-3" />
                    </Link>
                  </div>
                  <div className="rounded-md border border-red-500/10 bg-red-500/5 p-3">
                    <p className="text-sm text-foreground">Documento de identidade expirado</p>
                    <p className="text-xs text-muted-foreground">Politica: KYC Compliance v1.0.0</p>
                    <Link href="/console/policies" className="mt-1 inline-flex items-center text-xs font-medium text-foreground hover:underline">
                      Ver politica
                      <ExternalLink className="ml-1 h-3 w-3" />
                    </Link>
                  </div>
                </div>
              </CardContent>
            </Card>
          )}

          {/* Share / Audit */}
          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium text-foreground">Compartilhar & Auditar</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                <Button variant="outline" size="sm" className="w-full justify-start bg-transparent" onClick={() => setQrOpen(true)}>
                  <QrCode className="mr-2 h-4 w-4" />
                  Gerar QR Code
                </Button>
                <Button variant="outline" size="sm" className="w-full justify-start bg-transparent" asChild>
                  <Link href="/verify/offline">
                    <WifiOff className="mr-2 h-4 w-4" />
                    Abrir Verificador
                  </Link>
                </Button>
                <Button variant="outline" size="sm" className="w-full justify-start bg-transparent" onClick={() => { window.print(); }}>
                  <Printer className="mr-2 h-4 w-4" />
                  Imprimir PDF
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>

      {/* QR Code dialog */}
      <Dialog open={qrOpen} onOpenChange={setQrOpen}>
        <DialogContent className="sm:max-w-sm">
          <DialogHeader>
            <DialogTitle>QR Code para /r/{'{'}cid{'}'}</DialogTitle>
          </DialogHeader>
          <div className="flex flex-col items-center gap-4 py-6">
            <div className="flex h-48 w-48 items-center justify-center rounded-xl border-2 border-dashed border-muted-foreground/20 bg-muted/30">
              <div className="grid grid-cols-5 gap-1">
                {Array.from({ length: 25 }).map((_, i) => (
                  <div key={i} className={`h-6 w-6 rounded-sm ${Math.random() > 0.4 ? "bg-foreground" : "bg-transparent"}`} />
                ))}
              </div>
            </div>
            <p className="text-center text-xs text-muted-foreground">
              Escaneie para acessar /r/{execution.cid.slice(0, 12)}...
            </p>
            <Button variant="outline" size="sm" className="bg-transparent" onClick={() => toast.success("QR Code copiado para area de transferencia.")}>
              <Copy className="mr-1.5 h-3.5 w-3.5" />
              Copiar imagem
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  )
}
