"use client"

import React from "react"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Upload, Check, FileArchive, Shield, WifiOff, AlertCircle } from "lucide-react"
import { BadgeEstado } from "@/components/tdln/badge-estado"
import { TimelineSIRP } from "@/components/tdln/timeline-sirp"
import { CardProva } from "@/components/tdln/card-prova"
import { CIDChip } from "@/components/tdln/cid-chip"
import { mockSIRPNodes, mockProofs } from "@/lib/mock-data"

export default function OfflineVerifierPage() {
  const [file, setFile] = useState<File | null>(null)
  const [verifying, setVerifying] = useState(false)
  const [verified, setVerified] = useState(false)
  const [verificationResult, setVerificationResult] = useState<{
    state: "ACK" | "NACK" | "ASK"
    cid: string
    title: string
    timestamp: string
  } | null>(null)

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const uploadedFile = e.target.files?.[0]
    if (uploadedFile && uploadedFile.name.endsWith(".zip")) {
      setFile(uploadedFile)
      setVerified(false)
      setVerificationResult(null)
    }
  }

  const handleVerify = async () => {
    if (!file) return
    
    setVerifying(true)
    
    // Simulate verification
    await new Promise((resolve) => setTimeout(resolve, 1500))
    
    setVerified(true)
    setVerifying(false)
    setVerificationResult({
      state: "ACK",
      cid: "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
      title: "Transacao financeira verificada offline",
      timestamp: new Date().toISOString(),
    })
  }

  const handleReset = () => {
    setFile(null)
    setVerified(false)
    setVerificationResult(null)
  }

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="border-b bg-card">
        <div className="mx-auto flex h-16 max-w-7xl items-center justify-between px-4 lg:px-8">
          <div className="flex items-center gap-2">
            <Shield className="h-6 w-6 text-foreground" />
            <span className="text-lg font-bold text-foreground">TDLN</span>
            <Badge variant="secondary" className="ml-2 text-xs">Verificador Offline</Badge>
          </div>
          <Button variant="outline" size="sm" onClick={handleReset}>
            Verificar Outro
          </Button>
        </div>
      </header>

      <main className="mx-auto max-w-7xl px-4 py-8 lg:px-8">
        {!file ? (
          /* Upload state */
          <div className="mx-auto max-w-2xl">
            <div className="text-center">
              <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-2xl bg-muted">
                <WifiOff className="h-8 w-8 text-muted-foreground" />
              </div>
              <h1 className="mt-6 text-3xl font-bold tracking-tight text-foreground">
                Verificador Offline
              </h1>
              <p className="mt-3 text-lg leading-relaxed text-muted-foreground">
                Verifique recibos TDLN sem conexao com internet. Upload do bundle.zip e verificacao criptografica local.
              </p>
            </div>

            <Card className="mt-8">
              <CardContent className="p-8">
                <label
                  htmlFor="bundle-upload"
                  className="flex cursor-pointer flex-col items-center justify-center rounded-xl border-2 border-dashed border-border bg-muted/30 p-12 transition-colors hover:bg-muted/50"
                >
                  <FileArchive className="h-12 w-12 text-muted-foreground" />
                  <p className="mt-4 text-sm font-medium text-foreground">
                    Clique para fazer upload do bundle.zip
                  </p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    ou arraste e solte o arquivo aqui
                  </p>
                  <input
                    id="bundle-upload"
                    type="file"
                    accept=".zip"
                    className="sr-only"
                    onChange={handleFileUpload}
                  />
                </label>

                <div className="mt-6 rounded-lg border bg-card p-4">
                  <div className="flex items-start gap-3">
                    <AlertCircle className="h-5 w-5 shrink-0 text-muted-foreground" />
                    <div>
                      <p className="text-sm font-medium text-foreground">Verificacao sem Backend</p>
                      <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                        O bundle.zip contem todas as provas criptograficas, evidencias e assinaturas necessarias.
                        A verificacao acontece 100% no seu navegador, sem comunicacao com servidores externos.
                      </p>
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>

            <div className="mt-6 text-center">
              <p className="text-xs text-muted-foreground">
                Bundles gerados em <code className="rounded bg-muted px-1.5 py-0.5">/console/r/{"<cid>"}</code>
              </p>
            </div>
          </div>
        ) : !verified ? (
          /* Ready to verify */
          <div className="mx-auto max-w-2xl">
            <Card>
              <CardContent className="p-8">
                <div className="flex items-start gap-4">
                  <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-lg bg-emerald-500/10">
                    <Check className="h-6 w-6 text-emerald-600" />
                  </div>
                  <div className="flex-1">
                    <h2 className="text-lg font-bold text-foreground">Bundle carregado</h2>
                    <p className="mt-1 text-sm text-muted-foreground">{file.name}</p>
                    <p className="mt-0.5 text-xs text-muted-foreground">
                      Tamanho: {(file.size / 1024).toFixed(2)} KB
                    </p>
                  </div>
                </div>

                <Button
                  className="mt-6 w-full"
                  size="lg"
                  onClick={handleVerify}
                  disabled={verifying}
                >
                  {verifying ? (
                    <>
                      <span className="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-background border-t-transparent" />
                      Verificando...
                    </>
                  ) : (
                    <>
                      <Shield className="mr-2 h-5 w-5" />
                      Verificar Bundle
                    </>
                  )}
                </Button>

                <p className="mt-4 text-center text-xs text-muted-foreground">
                  A verificacao pode levar alguns segundos dependendo do tamanho do bundle.
                </p>
              </CardContent>
            </Card>
          </div>
        ) : (
          /* Verification result */
          <div className="space-y-6">
            {/* Result banner */}
            <div className="rounded-xl border border-emerald-500/20 bg-emerald-500/5 p-6">
              <div className="flex items-start gap-4">
                <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-emerald-500/10">
                  <Check className="h-6 w-6 text-emerald-600" />
                </div>
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <h1 className="text-lg font-bold tracking-tight text-foreground">
                      {verificationResult?.title}
                    </h1>
                    <BadgeEstado state={verificationResult?.state || "ACK"} size="default" />
                  </div>
                  <p className="mt-1 text-sm leading-relaxed text-muted-foreground">
                    Decisao aceita — verificacao concluida ✓
                  </p>
                </div>
              </div>
              <div className="mt-4">
                <span className="text-xs text-muted-foreground">CID</span>
                <div className="mt-1">
                  <CIDChip cid={verificationResult?.cid || ""} />
                </div>
              </div>
            </div>

            {/* Details */}
            <div className="grid gap-6 lg:grid-cols-2">
              {/* Timeline */}
              <Card>
                <CardHeader>
                  <CardTitle className="text-sm font-medium text-foreground">Timeline SIRP</CardTitle>
                </CardHeader>
                <CardContent>
                  <TimelineSIRP nodes={mockSIRPNodes} />
                </CardContent>
              </Card>

              {/* Proofs */}
              <Card>
                <CardHeader>
                  <CardTitle className="text-sm font-medium text-foreground">Provas Verificadas</CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  {mockProofs.map((proof) => (
                    <CardProva key={proof.type} proof={proof} />
                  ))}
                </CardContent>
              </Card>
            </div>

            {/* Verification details */}
            <Card>
              <CardHeader>
                <CardTitle className="text-sm font-medium text-foreground">Detalhes da Verificacao</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="grid gap-3 sm:grid-cols-3">
                  <div className="rounded-lg border bg-muted/30 p-3">
                    <p className="text-xs text-muted-foreground">Modo</p>
                    <p className="mt-1 flex items-center gap-1.5 text-sm font-medium text-foreground">
                      <WifiOff className="h-3.5 w-3.5" />
                      Offline
                    </p>
                  </div>
                  <div className="rounded-lg border bg-muted/30 p-3">
                    <p className="text-xs text-muted-foreground">Timestamp</p>
                    <p className="mt-1 text-sm font-medium text-foreground">
                      {new Date(verificationResult?.timestamp || "").toLocaleString("pt-BR")}
                    </p>
                  </div>
                  <div className="rounded-lg border bg-muted/30 p-3">
                    <p className="text-xs text-muted-foreground">Bundle</p>
                    <p className="mt-1 truncate text-sm font-medium text-foreground">{file?.name}</p>
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>
        )}
      </main>
    </div>
  )
}
