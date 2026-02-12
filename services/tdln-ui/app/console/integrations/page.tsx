"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Copy, Check, Plus, ExternalLink, RefreshCw, Circle } from "lucide-react"
import Link from "next/link"
import { toast } from "sonner"

const sdks = [
  { name: "Node.js / TypeScript", pkg: "@tdln/sdk", version: "2.1.0", install: "npm install @tdln/sdk" },
  { name: "Python", pkg: "tdln-sdk", version: "1.3.0", install: "pip install tdln-sdk" },
  { name: "Rust", pkg: "tdln-rs", version: "0.9.0", install: 'cargo add tdln-rs' },
]

const gatewaySnippets = {
  nginx: `# nginx.conf
location /api/ {
  proxy_pass http://upstream;
  tdln_enabled on;
  tdln_api_key "$TDLN_API_KEY";
  tdln_region "br-south";
}`,
  envoy: `# envoy.yaml
http_filters:
  - name: tdln.filter
    typed_config:
      "@type": type.googleapis.com/tdln.FilterConfig
      api_key: "\${TDLN_API_KEY}"
      region: "br-south"`,
}

const mockWebhooks = [
  { id: "wh_001", url: "https://api.empresa.com/tdln/webhook", status: "active", lastDelivery: "2026-02-10T14:32:00Z", successRate: "100%" },
  { id: "wh_002", url: "https://slack.empresa.com/tdln", status: "active", lastDelivery: "2026-02-10T14:28:00Z", successRate: "98.5%" },
]

export default function IntegrationsPage() {
  const [copiedIdx, setCopiedIdx] = useState<number | null>(null)
  const [gatewayTab, setGatewayTab] = useState("nginx")

  const handleCopy = async (text: string, idx: number) => {
    await navigator.clipboard.writeText(text)
    setCopiedIdx(idx)
    setTimeout(() => setCopiedIdx(null), 2000)
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-foreground">Integracoes</h1>
        <p className="mt-1 text-sm text-muted-foreground">SDKs, gateways e webhooks para conectar TDLN a sua stack.</p>
      </div>

      <Tabs defaultValue="sdks">
        <TabsList>
          <TabsTrigger value="sdks">SDKs</TabsTrigger>
          <TabsTrigger value="gateways">Gateways</TabsTrigger>
          <TabsTrigger value="ci">CI/CD</TabsTrigger>
          <TabsTrigger value="siem">SIEM</TabsTrigger>
          <TabsTrigger value="webhooks">Webhooks</TabsTrigger>
        </TabsList>

        <TabsContent value="sdks" className="mt-6">
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {sdks.map((sdk, i) => (
              <Card key={sdk.pkg}>
                <CardContent className="p-5">
                  <div className="flex items-start justify-between">
                    <div>
                      <h3 className="text-sm font-semibold text-foreground">{sdk.name}</h3>
                      <p className="mt-0.5 font-mono text-xs text-muted-foreground">{sdk.pkg}</p>
                    </div>
                    <Badge variant="secondary" className="text-xs">v{sdk.version}</Badge>
                  </div>
                  <div className="mt-4">
                    <div className="flex items-center gap-2">
                      <code className="flex-1 rounded-md border bg-muted px-3 py-2 font-mono text-xs text-foreground">{sdk.install}</code>
                      <Button variant="outline" size="icon" className="h-9 w-9 shrink-0 bg-transparent" onClick={() => handleCopy(sdk.install, i)}>
                        {copiedIdx === i ? <Check className="h-3.5 w-3.5 text-emerald-500" /> : <Copy className="h-3.5 w-3.5" />}
                      </Button>
                    </div>
                  </div>
                  <Button variant="outline" size="sm" className="mt-3 w-full text-xs bg-transparent" asChild>
                    <Link href="/docs">
                      <ExternalLink className="mr-1.5 h-3 w-3" />
                      Ver documentacao
                    </Link>
                  </Button>
                </CardContent>
              </Card>
            ))}
          </div>

          {/* Quickstart */}
          <Card className="mt-6">
            <CardHeader>
              <CardTitle className="text-sm font-medium text-foreground">Quickstart — Node.js</CardTitle>
            </CardHeader>
            <CardContent>
              <pre className="overflow-x-auto rounded-lg border bg-foreground/95 p-4 text-xs text-background/90">
                <code>{`import { TDLN } from '@tdln/sdk'

const tdln = new TDLN({
  apiKey: process.env.TDLN_API_KEY,
  secret: process.env.TDLN_SECRET,
  region: 'br-south',
})

// Criar uma execucao
const receipt = await tdln.execute({
  title: 'Transacao financeira',
  payload: { amount: 1500, currency: 'BRL' },
  policies: ['sanctions_aml'],
})

console.log(receipt.state) // 'ACK' | 'NACK' | 'ASK'
console.log(receipt.cid)   // 'bafybeig...'`}</code>
              </pre>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="gateways" className="mt-6">
          <div className="flex gap-2 mb-4">
            {["nginx", "envoy"].map((gw) => (
              <button
                key={gw}
                onClick={() => setGatewayTab(gw)}
                className={`rounded-md px-3 py-1.5 text-xs font-medium capitalize transition-colors ${
                  gatewayTab === gw ? "bg-foreground text-background" : "bg-muted text-muted-foreground hover:text-foreground"
                }`}
              >
                {gw}
              </button>
            ))}
          </div>
          <Card>
            <CardContent className="p-0">
              <pre className="overflow-x-auto rounded-lg p-4 text-xs text-foreground">
                <code>{gatewaySnippets[gatewayTab as keyof typeof gatewaySnippets]}</code>
              </pre>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="ci" className="mt-6">
          <div className="grid gap-6 lg:grid-cols-2">
            {/* GitHub Actions */}
            <Card>
              <CardHeader>
                <div className="flex items-center gap-2">
                  <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-muted">
                    <svg viewBox="0 0 24 24" className="h-4 w-4 fill-foreground">
                      <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/>
                    </svg>
                  </div>
                  <CardTitle className="text-sm font-medium">GitHub Actions</CardTitle>
                </div>
              </CardHeader>
              <CardContent>
                <p className="text-xs text-muted-foreground mb-3">Adicione verificacao TDLN aos seus workflows de CI.</p>
                <pre className="overflow-x-auto rounded-lg border bg-foreground/95 p-3 text-[11px] leading-relaxed text-background/90">
                  <code>{`# .github/workflows/tdln.yml
name: TDLN Verification

on: [push, pull_request]

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: tdln/action@v2
        with:
          api_key: \${{ secrets.TDLN_API_KEY }}
          secret: \${{ secrets.TDLN_SECRET }}
          policy: "ci_compliance"`}</code>
                </pre>
              </CardContent>
            </Card>

            {/* GitLab CI */}
            <Card>
              <CardHeader>
                <div className="flex items-center gap-2">
                  <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-muted">
                    <svg viewBox="0 0 24 24" className="h-4 w-4 fill-foreground">
                      <path d="M23.546 10.93L13.067.452c-.604-.603-1.582-.603-2.188 0L.452 10.93c-.6.605-.6 1.584 0 2.188l10.427 10.43c.606.603 1.582.603 2.188 0l10.48-10.43c.6-.604.6-1.583 0-2.188z"/>
                    </svg>
                  </div>
                  <CardTitle className="text-sm font-medium">GitLab CI</CardTitle>
                </div>
              </CardHeader>
              <CardContent>
                <p className="text-xs text-muted-foreground mb-3">Integre TDLN no seu pipeline GitLab.</p>
                <pre className="overflow-x-auto rounded-lg border bg-foreground/95 p-3 text-[11px] leading-relaxed text-background/90">
                  <code>{`# .gitlab-ci.yml
tdln_verify:
  stage: test
  image: tdln/cli:latest
  script:
    - tdln verify \\
        --api-key=$TDLN_API_KEY \\
        --secret=$TDLN_SECRET \\
        --policy=ci_compliance
  only:
    - merge_requests
    - main`}</code>
                </pre>
              </CardContent>
            </Card>
          </div>

          <Card className="mt-6">
            <CardContent className="p-4">
              <p className="text-xs text-muted-foreground">
                <strong>Dica:</strong> Configure as secrets <code className="rounded bg-muted px-1.5 py-0.5">TDLN_API_KEY</code> e <code className="rounded bg-muted px-1.5 py-0.5">TDLN_SECRET</code> no seu repositorio.
              </p>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="siem" className="mt-6">
          <p className="mb-6 text-sm text-muted-foreground">
            Envie logs de auditoria e execucoes para sua plataforma SIEM.
          </p>

          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {[
              { name: "Splunk", steps: 3, status: "available" },
              { name: "Elastic / Logstash", steps: 4, status: "available" },
              { name: "Datadog", steps: 2, status: "available" },
              { name: "Sumo Logic", steps: 3, status: "soon" },
              { name: "AWS CloudWatch", steps: 3, status: "available" },
              { name: "Azure Sentinel", steps: 4, status: "soon" },
            ].map((siem) => (
              <Card key={siem.name} className={siem.status === "soon" ? "opacity-60" : ""}>
                <CardContent className="p-4">
                  <div className="flex items-start justify-between">
                    <div>
                      <h3 className="text-sm font-semibold text-foreground">{siem.name}</h3>
                      <p className="mt-0.5 text-xs text-muted-foreground">{siem.steps} passos de setup</p>
                    </div>
                    {siem.status === "soon" ? (
                      <Badge variant="secondary" className="text-xs">Em breve</Badge>
                    ) : (
                      <Badge variant="default" className="text-xs">Disponivel</Badge>
                    )}
                  </div>
                  {siem.status === "available" && (
                    <Button 
                      variant="outline" 
                      size="sm" 
                      className="mt-3 w-full text-xs bg-transparent"
                      asChild
                    >
                      <Link href="/docs?section=sdks">
                        <ExternalLink className="mr-1.5 h-3 w-3" />
                        Ver guia
                      </Link>
                    </Button>
                  )}
                </CardContent>
              </Card>
            ))}
          </div>

          <Card className="mt-6">
            <CardHeader>
              <CardTitle className="text-sm font-medium">Exemplo: Splunk HTTP Event Collector</CardTitle>
            </CardHeader>
            <CardContent>
              <pre className="overflow-x-auto rounded-lg border bg-foreground/95 p-4 text-xs text-background/90">
                <code>{`# Console TDLN → Settings → Integrations → SIEM
# 1. Ativar Splunk HEC
# 2. Adicionar token HEC
# 3. Escolher eventos (executions, audits, evidence_failures)

curl -X POST https://api.tdln.com/v1/integrations/siem \\
  -H "Authorization: Bearer $TDLN_API_KEY" \\
  -d '{
    "provider": "splunk",
    "endpoint": "https://splunk.empresa.com:8088",
    "token": "YOUR_HEC_TOKEN",
    "events": ["executions", "audits"]
  }'`}</code>
              </pre>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="webhooks" className="mt-6 space-y-4">
          <div className="flex items-center justify-between">
            <p className="text-sm text-muted-foreground">Configure endpoints para receber eventos em tempo real.</p>
            <Button size="sm" onClick={() => toast.info("Configure webhooks em Configuracoes > Webhooks.")} asChild>
              <Link href="/console/settings">
                <Plus className="mr-1.5 h-4 w-4" />
                Novo webhook
              </Link>
            </Button>
          </div>

          {mockWebhooks.map((wh) => (
            <Card key={wh.id}>
              <CardContent className="flex items-center justify-between p-4">
                <div className="flex items-center gap-3 min-w-0">
                  <Circle className="h-2.5 w-2.5 shrink-0 fill-emerald-500 text-emerald-500" />
                  <div className="min-w-0">
                    <p className="truncate font-mono text-sm text-foreground">{wh.url}</p>
                    <p className="text-xs text-muted-foreground">
                      Ultima entrega: {new Date(wh.lastDelivery).toLocaleString("pt-BR")} &middot; Taxa: {wh.successRate}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  <Button variant="outline" size="sm" className="h-8 text-xs bg-transparent" onClick={() => toast.success(`Replay enviado para ${wh.url}. Status: 200 OK.`)}>
                    <RefreshCw className="mr-1 h-3 w-3" />
                    Replay
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </TabsContent>
      </Tabs>
    </div>
  )
}
