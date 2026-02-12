"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Shield, Globe, KeyRound, Rocket, Check, Copy, ArrowRight, ArrowLeft, Eye, EyeOff } from "lucide-react"
import Link from "next/link"

const STEPS = [
  { icon: Shield, title: "Criar tenant", description: "Configure seu espaco de trabalho" },
  { icon: Globe, title: "Escolher regiao", description: "Onde seus dados serao armazenados" },
  { icon: KeyRound, title: "Gerar credenciais", description: "Chave de API e secret" },
  { icon: Rocket, title: "Primeiro recibo", description: "Teste guiado Hello Receipt" },
]

const regions = [
  { id: "br-south", label: "Brasil Sul (Sao Paulo)", flag: "BR" },
  { id: "us-east", label: "US East (Virginia)", flag: "US" },
  { id: "eu-west", label: "EU West (Frankfurt)", flag: "EU" },
]

const snippets = {
  sdk: `import { TDLN } from '@tdln/sdk'

const tdln = new TDLN({
  apiKey: 'tdln_k_...',
  secret: 'tdln_s_...',
  region: 'br-south',
})

const receipt = await tdln.execute({
  title: 'Hello Receipt',
  payload: { message: 'Primeiro recibo!' },
})

console.log(receipt.cid)
// => bafybeig...`,
  nginx: `# nginx.conf - TDLN Gateway
location /api/ {
  proxy_pass http://upstream;
  
  # TDLN interception
  tdln_enabled on;
  tdln_api_key "tdln_k_...";
  tdln_secret "tdln_s_...";
  tdln_region "br-south";
}`,
}

export default function OnboardingPage() {
  const [currentStep, setCurrentStep] = useState(0)
  const [tenantName, setTenantName] = useState("")
  const [selectedRegion, setSelectedRegion] = useState("")
  const [showSecret, setShowSecret] = useState(false)
  const [copiedKey, setCopiedKey] = useState(false)
  const [copiedSecret, setCopiedSecret] = useState(false)
  const [testComplete, setTestComplete] = useState(false)
  const [selectedSnippet, setSelectedSnippet] = useState<"sdk" | "nginx">("sdk")

  const mockApiKey = "tdln_k_live_a1b2c3d4e5f6g7h8i9j0"
  const mockSecret = "tdln_s_live_z9y8x7w6v5u4t3s2r1q0p9o8n7m6l5k4"

  const canProceed = () => {
    if (currentStep === 0) return tenantName.length >= 3
    if (currentStep === 1) return selectedRegion !== ""
    if (currentStep === 2) return true
    return true
  }

  const handleCopy = async (text: string, type: "key" | "secret") => {
    await navigator.clipboard.writeText(text)
    if (type === "key") { setCopiedKey(true); setTimeout(() => setCopiedKey(false), 2000) }
    else { setCopiedSecret(true); setTimeout(() => setCopiedSecret(false), 2000) }
  }

  return (
    <div className="flex min-h-screen flex-col bg-muted/30">
      {/* Header */}
      <header className="border-b bg-background">
        <div className="mx-auto flex h-16 max-w-4xl items-center justify-between px-4">
          <Link href="/" className="flex items-center gap-2">
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-foreground">
              <Shield className="h-4 w-4 text-background" />
            </div>
            <span className="text-lg font-bold tracking-tight text-foreground">TDLN</span>
          </Link>
          <span className="text-sm text-muted-foreground">Passo {currentStep + 1} de {STEPS.length}</span>
        </div>
      </header>

      <main className="flex-1 py-12">
        <div className="mx-auto max-w-4xl px-4">
          {/* Progress */}
          <div className="mb-12 flex items-center justify-center gap-0">
            {STEPS.map((step, i) => (
              <div key={step.title} className="flex items-center">
                <div className="flex flex-col items-center gap-2">
                  <div className={`flex h-10 w-10 items-center justify-center rounded-full border-2 transition-colors ${
                    i < currentStep ? "border-emerald-500 bg-emerald-500 text-background" :
                    i === currentStep ? "border-foreground bg-foreground text-background" :
                    "border-border bg-background text-muted-foreground"
                  }`}>
                    {i < currentStep ? <Check className="h-4 w-4" /> : <step.icon className="h-4 w-4" />}
                  </div>
                  <span className={`text-xs font-medium ${i <= currentStep ? "text-foreground" : "text-muted-foreground"}`}>
                    {step.title}
                  </span>
                </div>
                {i < STEPS.length - 1 && (
                  <div className={`mx-3 mb-6 h-0.5 w-16 ${i < currentStep ? "bg-emerald-500" : "bg-border"}`} />
                )}
              </div>
            ))}
          </div>

          {/* Step content */}
          <Card className="mx-auto max-w-2xl">
            <CardHeader>
              <CardTitle className="text-xl font-bold text-foreground">{STEPS[currentStep].title}</CardTitle>
              <p className="text-sm text-muted-foreground">{STEPS[currentStep].description}</p>
            </CardHeader>
            <CardContent>
              {currentStep === 0 && (
                <div className="space-y-4">
                  <div>
                    <label htmlFor="tenant-name" className="text-sm font-medium text-foreground">Nome do tenant</label>
                    <Input
                      id="tenant-name"
                      placeholder="Ex: Minha Empresa"
                      value={tenantName}
                      onChange={(e) => setTenantName(e.target.value)}
                      className="mt-1.5"
                    />
                    <p className="mt-1 text-xs text-muted-foreground">Minimo 3 caracteres. Pode ser alterado depois.</p>
                  </div>
                </div>
              )}

              {currentStep === 1 && (
                <div className="space-y-3">
                  {regions.map((region) => (
                    <button
                      key={region.id}
                      onClick={() => setSelectedRegion(region.id)}
                      className={`flex w-full items-center gap-3 rounded-lg border p-4 text-left transition-colors ${
                        selectedRegion === region.id ? "border-foreground bg-muted ring-1 ring-foreground" : "hover:bg-muted/50"
                      }`}
                    >
                      <span className="text-lg">{region.flag}</span>
                      <div>
                        <p className="text-sm font-medium text-foreground">{region.label}</p>
                        <p className="text-xs text-muted-foreground">Dados armazenados nesta regiao</p>
                      </div>
                      {selectedRegion === region.id && <Check className="ml-auto h-4 w-4 text-foreground" />}
                    </button>
                  ))}
                </div>
              )}

              {currentStep === 2 && (
                <div className="space-y-6">
                  <div className="rounded-lg border border-amber-500/20 bg-amber-500/5 p-3">
                    <p className="text-sm font-medium text-amber-600">Atencao: o secret so sera exibido uma vez.</p>
                    <p className="text-xs text-muted-foreground">Salve-o em um local seguro antes de continuar.</p>
                  </div>

                  <div className="space-y-4">
                    <div>
                      <label className="text-sm font-medium text-foreground">API Key</label>
                      <div className="mt-1.5 flex items-center gap-2">
                        <code className="flex-1 rounded-md border bg-muted px-3 py-2 font-mono text-xs text-foreground">{mockApiKey}</code>
                        <Button variant="outline" size="icon" className="h-9 w-9 bg-transparent" onClick={() => handleCopy(mockApiKey, "key")}>
                          {copiedKey ? <Check className="h-3.5 w-3.5 text-emerald-500" /> : <Copy className="h-3.5 w-3.5" />}
                        </Button>
                      </div>
                    </div>
                    <div>
                      <label className="text-sm font-medium text-foreground">Secret</label>
                      <div className="mt-1.5 flex items-center gap-2">
                        <code className="flex-1 rounded-md border bg-muted px-3 py-2 font-mono text-xs text-foreground">
                          {showSecret ? mockSecret : "••••••••••••••••••••••••••••••"}
                        </code>
                        <Button variant="outline" size="icon" className="h-9 w-9 bg-transparent" onClick={() => setShowSecret(!showSecret)}>
                          {showSecret ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
                        </Button>
                        <Button variant="outline" size="icon" className="h-9 w-9 bg-transparent" onClick={() => handleCopy(mockSecret, "secret")}>
                          {copiedSecret ? <Check className="h-3.5 w-3.5 text-emerald-500" /> : <Copy className="h-3.5 w-3.5" />}
                        </Button>
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {currentStep === 3 && (
                <div className="space-y-6">
                  {!testComplete ? (
                    <>
                      <p className="text-sm text-muted-foreground">
                        Clique para simular sua primeira execucao. Isso criara um recibo verificavel de teste.
                      </p>
                      <Button onClick={() => setTestComplete(true)} className="w-full">
                        <Rocket className="mr-2 h-4 w-4" />
                        Gerar Hello Receipt
                      </Button>
                    </>
                  ) : (
                    <>
                      <div className="rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-4">
                        <div className="flex items-center gap-2 text-emerald-600">
                          <Check className="h-4 w-4" />
                          <span className="text-sm font-semibold">Recibo criado com sucesso!</span>
                        </div>
                        <p className="mt-1 text-xs text-muted-foreground">
                          CID: bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi
                        </p>
                        <Link href="/console/r/bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi" className="mt-2 inline-flex items-center text-xs font-medium text-foreground hover:underline">
                          Ver recibo /r/{'<cid>'}
                          <ArrowRight className="ml-1 h-3 w-3" />
                        </Link>
                      </div>

                      <div>
                        <p className="mb-3 text-sm font-medium text-foreground">Integre na sua aplicacao:</p>
                        <div className="flex gap-2 mb-3">
                          <button
                            onClick={() => setSelectedSnippet("sdk")}
                            className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${selectedSnippet === "sdk" ? "bg-foreground text-background" : "bg-muted text-muted-foreground hover:text-foreground"}`}
                          >
                            SDK Node.js
                          </button>
                          <button
                            onClick={() => setSelectedSnippet("nginx")}
                            className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${selectedSnippet === "nginx" ? "bg-foreground text-background" : "bg-muted text-muted-foreground hover:text-foreground"}`}
                          >
                            Nginx Gateway
                          </button>
                        </div>
                        <pre className="overflow-x-auto rounded-lg border bg-foreground/95 p-4 text-xs text-background/90">
                          <code>{snippets[selectedSnippet]}</code>
                        </pre>
                      </div>
                    </>
                  )}
                </div>
              )}

              {/* Navigation */}
              <div className="mt-8 flex items-center justify-between">
                <Button
                  variant="outline"
                  onClick={() => setCurrentStep(Math.max(0, currentStep - 1))}
                  disabled={currentStep === 0}
                >
                  <ArrowLeft className="mr-2 h-4 w-4" />
                  Voltar
                </Button>
                {currentStep < STEPS.length - 1 ? (
                  <Button
                    onClick={() => setCurrentStep(currentStep + 1)}
                    disabled={!canProceed()}
                  >
                    Proximo
                    <ArrowRight className="ml-2 h-4 w-4" />
                  </Button>
                ) : (
                  <Button asChild disabled={!testComplete}>
                    <Link href="/console">
                      Ir para o console
                      <ArrowRight className="ml-2 h-4 w-4" />
                    </Link>
                  </Button>
                )}
              </div>
            </CardContent>
          </Card>
        </div>
      </main>
    </div>
  )
}
