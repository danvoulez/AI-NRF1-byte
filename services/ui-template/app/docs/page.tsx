"use client"

import { useState, useEffect, Suspense } from "react"
import { useSearchParams } from "next/navigation"
import Link from "next/link"
import { MarketingHeader } from "@/components/marketing/header"
import { MarketingFooter } from "@/components/marketing/footer"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import {
  Rocket,
  Code,
  Globe,
  Webhook,
  Copy,
  Check,
  ArrowRight,
  BookOpen,
  Terminal,
  FileCode2,
  Zap,
} from "lucide-react"

const sidebarSections = [
  {
    title: "Comecar",
    items: [
      { label: "Quickstart", id: "quickstart", icon: Rocket },
      { label: "Conceitos", id: "concepts", icon: BookOpen },
    ],
  },
  {
    title: "SDKs",
    items: [
      { label: "Node.js / TypeScript", id: "sdk-node", icon: Terminal },
      { label: "Python", id: "sdk-python", icon: FileCode2 },
      { label: "Rust", id: "sdk-rust", icon: Code },
    ],
  },
  {
    title: "Referencia",
    items: [
      { label: "API REST", id: "api-rest", icon: Globe },
      { label: "Webhooks", id: "webhooks", icon: Webhook },
      { label: "Exemplos", id: "examples", icon: Zap },
    ],
  },
  {
    title: "Produto & Metricas",
    items: [
      { label: "Telemetria", id: "telemetry", icon: Terminal },
      { label: "Testes UX de Aceite", id: "ux-tests", icon: BookOpen },
    ],
  },
]

const codeExamples: Record<string, string> = {
  node: `import { TDLN } from '@tdln/sdk'

const tdln = new TDLN({
  apiKey: process.env.TDLN_API_KEY!,
  secret: process.env.TDLN_SECRET!,
  region: 'br-south',
})

// Criar uma execucao
const receipt = await tdln.execute({
  title: 'Transacao financeira',
  payload: { amount: 1500, currency: 'BRL' },
  policies: ['sanctions_aml'],
})

// Verificar resultado
if (receipt.state === 'ACK') {
  console.log('Aprovado:', receipt.cid)
} else if (receipt.state === 'ASK') {
  console.log('Evidencias pendentes:', receipt.poi)
} else {
  console.log('Rejeitado:', receipt.violations)
}`,
  python: `from tdln import TDLN

tdln = TDLN(
    api_key=os.environ["TDLN_API_KEY"],
    secret=os.environ["TDLN_SECRET"],
    region="br-south",
)

# Criar uma execucao
receipt = tdln.execute(
    title="Transacao financeira",
    payload={"amount": 1500, "currency": "BRL"},
    policies=["sanctions_aml"],
)

print(f"Estado: {receipt.state}")
print(f"CID: {receipt.cid}")`,
  rust: `use tdln_rs::TDLN;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tdln = TDLN::new(
        std::env::var("TDLN_API_KEY")?,
        std::env::var("TDLN_SECRET")?,
        "br-south",
    );

    let receipt = tdln.execute(
        "Transacao financeira",
        serde_json::json!({
            "amount": 1500,
            "currency": "BRL"
        }),
        vec!["sanctions_aml"],
    ).await?;

    println!("Estado: {:?}", receipt.state);
    println!("CID: {}", receipt.cid);
    Ok(())
}`,
  rest: `# Criar execucao
curl -X POST https://api.tdln.io/v1/executions \\
  -H "Authorization: Bearer tdln_k_live_..." \\
  -H "X-TDLN-Signature: ..." \\
  -H "Content-Type: application/json" \\
  -d '{
    "title": "Transacao financeira",
    "payload": {"amount": 1500, "currency": "BRL"},
    "policies": ["sanctions_aml"]
  }'

# Resposta
{
  "id": "exec_abc123",
  "state": "ACK",
  "cid": "bafybeig...",
  "sirp": {
    "intent": { "signer": "client:sdk-node@2.1.0", ... },
    "delivery": { ... },
    "execution": { ... },
    "result": { ... }
  }
}`,
  webhook: `// Configurar webhook endpoint
POST /v1/webhooks
{
  "url": "https://api.empresa.com/tdln/events",
  "events": ["execution.created", "execution.completed", "execution.nack"],
  "secret": "whs_..." // gerado automaticamente
}

// Payload recebido
{
  "event": "execution.completed",
  "data": {
    "id": "exec_abc123",
    "state": "ACK",
    "cid": "bafybeig...",
    "timestamp": "2026-02-10T14:32:00Z"
  },
  "signature": "sha256=..." // HMAC para verificacao
}

// Verificar assinatura (Node.js)
import crypto from 'crypto'
const expected = crypto
  .createHmac('sha256', webhookSecret)
  .update(rawBody)
  .digest('hex')

if (expected !== receivedSignature) {
  throw new Error('Assinatura invalida')
}`,
}

const sectionMap: Record<string, string> = {
  sdks: "sdk-node",
  "api-rest": "api-rest",
  webhooks: "webhooks",
  quickstart: "quickstart",
  concepts: "concepts",
  examples: "examples",
  telemetry: "telemetry",
  "ux-tests": "ux-tests",
}

function DocsPageInner() {
  const searchParams = useSearchParams()
  const sectionParam = searchParams.get("section")
  const initialSection = (sectionParam && sectionMap[sectionParam]) || "quickstart"
  const [activeSection, setActiveSection] = useState(initialSection)
  const [copiedCode, setCopiedCode] = useState<string | null>(null)

  useEffect(() => {
    if (sectionParam && sectionMap[sectionParam]) {
      setActiveSection(sectionMap[sectionParam])
    }
  }, [sectionParam])

  const handleCopy = async (code: string, id: string) => {
    await navigator.clipboard.writeText(code)
    setCopiedCode(id)
    setTimeout(() => setCopiedCode(null), 2000)
  }

  return (
    <div className="flex min-h-screen flex-col">
      <MarketingHeader />
      <main className="flex-1">
        <div className="mx-auto max-w-7xl px-4 py-12 lg:px-8">
          <div className="grid gap-8 lg:grid-cols-12">
            {/* Sidebar */}
            <aside className="lg:col-span-3">
              <nav className="sticky top-24 space-y-6" aria-label="Documentacao">
                {sidebarSections.map((section) => (
                  <div key={section.title}>
                    <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">{section.title}</h3>
                    <ul className="mt-2 space-y-1">
                      {section.items.map((item) => (
                        <li key={item.id}>
                          <button
                            onClick={() => setActiveSection(item.id)}
                            className={`flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors ${
                              activeSection === item.id
                                ? "bg-muted font-medium text-foreground"
                                : "text-muted-foreground hover:bg-muted/50 hover:text-foreground"
                            }`}
                          >
                            <item.icon className="h-4 w-4 shrink-0" />
                            {item.label}
                          </button>
                        </li>
                      ))}
                    </ul>
                  </div>
                ))}
              </nav>
            </aside>

            {/* Content */}
            <div className="lg:col-span-9">
              {/* Quickstart */}
              {activeSection === "quickstart" && (
                <article className="space-y-8">
                  <div>
                    <Badge variant="secondary" className="text-xs">Guia</Badge>
                    <h1 className="mt-3 text-3xl font-bold tracking-tight text-foreground">Quickstart</h1>
                    <p className="mt-2 text-muted-foreground leading-relaxed">
                      Crie seu primeiro recibo verificavel em 3 passos. Tempo estimado: 5 minutos.
                    </p>
                  </div>

                  <div className="space-y-6">
                    {[
                      {
                        step: 1,
                        title: "Instale o SDK",
                        code: "npm install @tdln/sdk",
                        desc: "Disponivel para Node.js, Python e Rust.",
                      },
                      {
                        step: 2,
                        title: "Configure as credenciais",
                        code: `export TDLN_API_KEY="tdln_k_live_..."
export TDLN_SECRET="tdln_s_live_..."`,
                        desc: "Obtenha suas credenciais no console em Configuracoes > Chaves de API.",
                      },
                      {
                        step: 3,
                        title: "Crie sua primeira execucao",
                        code: codeExamples.node,
                        desc: "O SDK envia a requisicao, o motor TDLN avalia as politicas e retorna um recibo com CID unico.",
                      },
                    ].map((item) => (
                      <div key={item.step} className="space-y-3">
                        <div className="flex items-center gap-3">
                          <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-foreground text-sm font-bold text-background">
                            {item.step}
                          </div>
                          <h2 className="text-lg font-semibold text-foreground">{item.title}</h2>
                        </div>
                        <p className="ml-11 text-sm text-muted-foreground">{item.desc}</p>
                        <div className="ml-11 relative">
                          <pre className="overflow-x-auto rounded-lg border bg-foreground/95 p-4 text-xs text-background/90">
                            <code>{item.code}</code>
                          </pre>
                          <button
                            onClick={() => handleCopy(item.code, `step-${item.step}`)}
                            className="absolute right-3 top-3 rounded-md bg-background/10 p-1.5 text-background/60 transition-colors hover:bg-background/20 hover:text-background"
                            aria-label="Copiar codigo"
                          >
                            {copiedCode === `step-${item.step}` ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>

                  <Card>
                    <CardContent className="flex items-center justify-between p-4">
                      <div>
                        <p className="text-sm font-medium text-foreground">Proximo: Configure sua primeira integracao</p>
                        <p className="text-xs text-muted-foreground">Conecte via gateway Nginx/Envoy ou use webhooks.</p>
                      </div>
                      <Button size="sm" asChild>
                        <Link href="/console/integrations">
                          Ir para integracoes
                          <ArrowRight className="ml-1.5 h-4 w-4" />
                        </Link>
                      </Button>
                    </CardContent>
                  </Card>
                </article>
              )}

              {/* Concepts */}
              {activeSection === "concepts" && (
                <article className="space-y-8">
                  <div>
                    <Badge variant="secondary" className="text-xs">Conceitos</Badge>
                    <h1 className="mt-3 text-3xl font-bold tracking-tight text-foreground">Conceitos fundamentais</h1>
                    <p className="mt-2 text-muted-foreground leading-relaxed">
                      Entenda os principais conceitos do TDLN e do protocolo SIRP.
                    </p>
                  </div>

                  <div className="space-y-6">
                    {[
                      {
                        title: "Execucao",
                        desc: "Uma decisao submetida ao motor TDLN para verificacao. Cada execucao gera um recibo com estado ACK (aprovada), NACK (rejeitada) ou ASK (evidencias pendentes).",
                      },
                      {
                        title: "CID (Content Identifier)",
                        desc: "Identificador unico baseado em hash do conteudo, seguindo o padrao IPFS/CID. Garante que o recibo e imutavel e verificavel independentemente.",
                      },
                      {
                        title: "Protocolo SIRP",
                        desc: "Send-Inspect-Receive-Prove. Protocolo de 4 etapas (INTENT, DELIVERY, EXECUTION, RESULT) que encadeia assinaturas criptograficas em cada fase do fluxo.",
                      },
                      {
                        title: "Bundle Offline",
                        desc: "Pacote .zip contendo todas as provas, assinaturas e metadados de um recibo. Pode ser verificado sem conexao ao backend do TDLN.",
                      },
                      {
                        title: "Proof of Incompleteness (PoI)",
                        desc: "No estado ASK, o PoI lista exatamente quais evidencias estao faltando para que a verificacao possa ser concluida.",
                      },
                      {
                        title: "AEAD",
                        desc: "Authenticated Encryption with Associated Data. Evidencias sensiveis sao criptografadas com AEAD, garantindo confidencialidade e integridade.",
                      },
                    ].map((concept) => (
                      <div key={concept.title} className="rounded-lg border p-5">
                        <h2 className="text-base font-semibold text-foreground">{concept.title}</h2>
                        <p className="mt-2 text-sm text-muted-foreground leading-relaxed">{concept.desc}</p>
                      </div>
                    ))}
                  </div>
                </article>
              )}

              {/* SDK pages */}
              {(activeSection === "sdk-node" || activeSection === "sdk-python" || activeSection === "sdk-rust") && (
                <article className="space-y-8">
                  <div>
                    <Badge variant="secondary" className="text-xs">SDK</Badge>
                    <h1 className="mt-3 text-3xl font-bold tracking-tight text-foreground">
                      {activeSection === "sdk-node" ? "Node.js / TypeScript" : activeSection === "sdk-python" ? "Python" : "Rust"}
                    </h1>
                    <p className="mt-2 text-muted-foreground leading-relaxed">
                      Instalacao, configuracao e exemplos para o SDK {activeSection === "sdk-node" ? "Node.js" : activeSection === "sdk-python" ? "Python" : "Rust"}.
                    </p>
                  </div>

                  <div className="space-y-4">
                    <h2 className="text-lg font-semibold text-foreground">Instalacao</h2>
                    <div className="relative">
                      <pre className="overflow-x-auto rounded-lg border bg-foreground/95 p-4 text-xs text-background/90">
                        <code>
                          {activeSection === "sdk-node" ? "npm install @tdln/sdk" :
                           activeSection === "sdk-python" ? "pip install tdln-sdk" :
                           "cargo add tdln-rs"}
                        </code>
                      </pre>
                    </div>
                  </div>

                  <div className="space-y-4">
                    <h2 className="text-lg font-semibold text-foreground">Exemplo completo</h2>
                    <div className="relative">
                      <pre className="overflow-x-auto rounded-lg border bg-foreground/95 p-4 text-xs text-background/90">
                        <code>
                          {activeSection === "sdk-node" ? codeExamples.node :
                           activeSection === "sdk-python" ? codeExamples.python :
                           codeExamples.rust}
                        </code>
                      </pre>
                      <button
                        onClick={() => handleCopy(
                          activeSection === "sdk-node" ? codeExamples.node :
                          activeSection === "sdk-python" ? codeExamples.python :
                          codeExamples.rust,
                          "sdk-example"
                        )}
                        className="absolute right-3 top-3 rounded-md bg-background/10 p-1.5 text-background/60 transition-colors hover:bg-background/20 hover:text-background"
                        aria-label="Copiar codigo"
                      >
                        {copiedCode === "sdk-example" ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
                      </button>
                    </div>
                  </div>
                </article>
              )}

              {/* REST API */}
              {activeSection === "api-rest" && (
                <article className="space-y-8">
                  <div>
                    <Badge variant="secondary" className="text-xs">Referencia</Badge>
                    <h1 className="mt-3 text-3xl font-bold tracking-tight text-foreground">API REST</h1>
                    <p className="mt-2 text-muted-foreground leading-relaxed">
                      Referencia completa da API REST do TDLN. Base URL: <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-sm text-foreground">https://api.tdln.io/v1</code>
                    </p>
                  </div>

                  <div className="space-y-4">
                    {[
                      { method: "POST", path: "/executions", desc: "Criar nova execucao" },
                      { method: "GET", path: "/executions/:id", desc: "Obter execucao por ID" },
                      { method: "GET", path: "/executions/:id/bundle", desc: "Baixar bundle offline" },
                      { method: "GET", path: "/receipts/:cid", desc: "Obter recibo por CID" },
                      { method: "GET", path: "/evidence/:cid", desc: "Obter evidencia por CID" },
                      { method: "GET", path: "/policies", desc: "Listar politicas ativas" },
                      { method: "POST", path: "/webhooks", desc: "Criar webhook endpoint" },
                    ].map((endpoint) => (
                      <div key={endpoint.path} className="flex items-center gap-3 rounded-lg border p-4">
                        <Badge className={`shrink-0 text-xs font-mono ${
                          endpoint.method === "POST" ? "bg-emerald-500/10 text-emerald-600 border-emerald-500/20" : "bg-muted text-foreground"
                        }`}>
                          {endpoint.method}
                        </Badge>
                        <code className="font-mono text-sm text-foreground">{endpoint.path}</code>
                        <span className="ml-auto text-sm text-muted-foreground">{endpoint.desc}</span>
                      </div>
                    ))}
                  </div>

                  <div className="space-y-4">
                    <h2 className="text-lg font-semibold text-foreground">Exemplo cURL</h2>
                    <div className="relative">
                      <pre className="overflow-x-auto rounded-lg border bg-foreground/95 p-4 text-xs text-background/90">
                        <code>{codeExamples.rest}</code>
                      </pre>
                      <button
                        onClick={() => handleCopy(codeExamples.rest, "rest")}
                        className="absolute right-3 top-3 rounded-md bg-background/10 p-1.5 text-background/60 transition-colors hover:bg-background/20 hover:text-background"
                        aria-label="Copiar codigo"
                      >
                        {copiedCode === "rest" ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
                      </button>
                    </div>
                  </div>
                </article>
              )}

              {/* Webhooks */}
              {activeSection === "webhooks" && (
                <article className="space-y-8">
                  <div>
                    <Badge variant="secondary" className="text-xs">Referencia</Badge>
                    <h1 className="mt-3 text-3xl font-bold tracking-tight text-foreground">Webhooks</h1>
                    <p className="mt-2 text-muted-foreground leading-relaxed">
                      Receba notificacoes em tempo real sobre eventos no TDLN via HTTP POST.
                    </p>
                  </div>

                  <div className="space-y-4">
                    <h2 className="text-lg font-semibold text-foreground">Eventos disponiveis</h2>
                    <div className="space-y-2">
                      {[
                        { event: "execution.created", desc: "Disparado quando uma nova execucao e criada" },
                        { event: "execution.completed", desc: "Disparado quando a execucao finaliza (ACK/NACK/ASK)" },
                        { event: "execution.nack", desc: "Disparado especificamente para rejeicoes" },
                        { event: "evidence.fetched", desc: "Disparado quando uma evidencia e obtida" },
                        { event: "policy.updated", desc: "Disparado quando uma politica e alterada" },
                      ].map((ev) => (
                        <div key={ev.event} className="flex items-center justify-between rounded-md border p-3">
                          <code className="font-mono text-sm text-foreground">{ev.event}</code>
                          <span className="text-xs text-muted-foreground">{ev.desc}</span>
                        </div>
                      ))}
                    </div>
                  </div>

                  <div className="space-y-4">
                    <h2 className="text-lg font-semibold text-foreground">Configuracao e verificacao</h2>
                    <div className="relative">
                      <pre className="overflow-x-auto rounded-lg border bg-foreground/95 p-4 text-xs text-background/90">
                        <code>{codeExamples.webhook}</code>
                      </pre>
                      <button
                        onClick={() => handleCopy(codeExamples.webhook, "webhook")}
                        className="absolute right-3 top-3 rounded-md bg-background/10 p-1.5 text-background/60 transition-colors hover:bg-background/20 hover:text-background"
                        aria-label="Copiar codigo"
                      >
                        {copiedCode === "webhook" ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
                      </button>
                    </div>
                  </div>
                </article>
              )}

              {/* Examples */}
              {activeSection === "examples" && (
                <article className="space-y-8">
                  <div>
                    <Badge variant="secondary" className="text-xs">Exemplos</Badge>
                    <h1 className="mt-3 text-3xl font-bold tracking-tight text-foreground">Exemplos & Snippets</h1>
                    <p className="mt-2 text-muted-foreground leading-relaxed">
                      Exemplos praticos para os cenarios mais comuns.
                    </p>
                  </div>

                  <Tabs defaultValue="node">
                    <TabsList>
                      <TabsTrigger value="node">Node.js</TabsTrigger>
                      <TabsTrigger value="python">Python</TabsTrigger>
                      <TabsTrigger value="rust">Rust</TabsTrigger>
                    </TabsList>
                    {(["node", "python", "rust"] as const).map((lang) => (
                      <TabsContent key={lang} value={lang} className="mt-4">
                        <div className="relative">
                          <pre className="overflow-x-auto rounded-lg border bg-foreground/95 p-4 text-xs text-background/90">
                            <code>{codeExamples[lang]}</code>
                          </pre>
                          <button
                            onClick={() => handleCopy(codeExamples[lang], `example-${lang}`)}
                            className="absolute right-3 top-3 rounded-md bg-background/10 p-1.5 text-background/60 transition-colors hover:bg-background/20 hover:text-background"
                            aria-label="Copiar codigo"
                          >
                            {copiedCode === `example-${lang}` ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
                          </button>
                        </div>
                      </TabsContent>
                    ))}
                  </Tabs>
                </article>
              )}

              {/* Telemetry */}
              {activeSection === "telemetry" && (
                <article className="space-y-8">
                  <div>
                    <Badge variant="secondary" className="text-xs">Produto & Metricas</Badge>
                    <h1 className="mt-3 text-3xl font-bold tracking-tight text-foreground">Telemetria</h1>
                    <p className="mt-2 text-muted-foreground leading-relaxed">
                      Metricas-chave para acompanhar ativacao, sucesso, adocao, saude e monetizacao do produto.
                    </p>
                  </div>

                  <div className="space-y-6">
                    {[
                      {
                        category: "Ativacao",
                        metrics: [
                          "Time to first receipt - Tempo ate gerar primeiro recibo",
                          "% com bundle downloaded - % de usuarios que baixaram bundle offline",
                        ],
                      },
                      {
                        category: "Sucesso",
                        metrics: [
                          "% ACK - Taxa de decisoes aceitas",
                          "p95 decision time - Latencia de decisao (percentil 95)",
                          "ASK→ACK resolution rate - Taxa de conversao de ASK para ACK apos correcao",
                        ],
                      },
                      {
                        category: "Adocao",
                        metrics: [
                          "Active integrations - Integracoes ativas por tenant",
                          "MAU - Monthly Active Users",
                          "Expansion by team - Crescimento de usuarios por time/tenant",
                        ],
                      },
                      {
                        category: "Saude",
                        metrics: [
                          "Errors by region - Taxa de erro por regiao",
                          "Latency p50/p95 - Latencia de API",
                          "Evidence fetch failures - Falhas ao obter evidencias",
                        ],
                      },
                      {
                        category: "Monetizacao",
                        metrics: [
                          "Execs per plan - Execucoes por plano (Free/Team/Enterprise)",
                          "Conversions - Taxa de conversao entre planos",
                          "Churn signals - Sinais de churn (reducao de uso, feedback negativo)",
                        ],
                      },
                    ].map((section) => (
                      <Card key={section.category}>
                        <CardContent className="p-5">
                          <h2 className="text-sm font-bold text-foreground">{section.category}</h2>
                          <ul className="mt-3 space-y-2">
                            {section.metrics.map((metric) => (
                              <li key={metric} className="flex items-start gap-2 text-sm text-muted-foreground">
                                <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-foreground/40" />
                                <span>{metric}</span>
                              </li>
                            ))}
                          </ul>
                        </CardContent>
                      </Card>
                    ))}
                  </div>
                </article>
              )}

              {/* UX Tests */}
              {activeSection === "ux-tests" && (
                <article className="space-y-8">
                  <div>
                    <Badge variant="secondary" className="text-xs">Produto & Metricas</Badge>
                    <h1 className="mt-3 text-3xl font-bold tracking-tight text-foreground">Testes UX de Aceite</h1>
                    <p className="mt-2 text-muted-foreground leading-relaxed">
                      Cenarios obrigatorios que todo fluxo UX deve cobrir para garantir uma experiencia completa e acessivel.
                    </p>
                  </div>

                  <div className="space-y-4">
                    {[
                      {
                        test: "ACK - Fluxo completo",
                        steps: [
                          "Gerar /r/<cid>",
                          "Ver SIRP timeline",
                          "Baixar bundle",
                          "Copiar link",
                        ],
                      },
                      {
                        test: "ASK - Proof of Interest",
                        steps: [
                          "Ver PoI + passos de correcao",
                          "Apos correcao → ACK",
                        ],
                      },
                      {
                        test: "NACK - Regras quebradas",
                        steps: [
                          "Ver broken rules",
                          "Link para politica",
                        ],
                      },
                      {
                        test: "Offline - Verificacao sem backend",
                        steps: [
                          "Abrir bundle sem backend",
                          "Verificacao passa",
                        ],
                      },
                      {
                        test: "A11y - Acessibilidade",
                        steps: [
                          "100% navegacao por teclado",
                          "Screen reader states funcionais",
                        ],
                      },
                      {
                        test: "RBAC - Permissoes",
                        steps: [
                          "Auditor pode ver /r/<cid>",
                          "Auditor NAO pode alterar policies",
                        ],
                      },
                      {
                        test: "Billing - Exceder quota",
                        steps: [
                          "Exceder quota mostra upsell (soft)",
                          "NAO bloqueia verificacao existente",
                        ],
                      },
                      {
                        test: "Integrations - Primeiro recibo",
                        steps: [
                          "Colar Nginx snippet",
                          "Primeiro recibo gerado <5min (guiado)",
                        ],
                      },
                    ].map((testCase) => (
                      <Card key={testCase.test}>
                        <CardContent className="p-5">
                          <h2 className="text-sm font-bold text-foreground">{testCase.test}</h2>
                          <ul className="mt-3 space-y-1.5">
                            {testCase.steps.map((step, idx) => (
                              <li key={idx} className="flex items-start gap-2 text-sm text-muted-foreground">
                                <span className="mt-1 flex h-4 w-4 shrink-0 items-center justify-center rounded-full bg-muted text-[10px] font-medium text-foreground">
                                  {idx + 1}
                                </span>
                                <span>{step}</span>
                              </li>
                            ))}
                          </ul>
                        </CardContent>
                      </Card>
                    ))}
                  </div>
                </article>
              )}
            </div>
          </div>
        </div>
      </main>
      <MarketingFooter />
    </div>
  )
}

export default function DocsPage() {
  return (
    <Suspense>
      <DocsPageInner />
    </Suspense>
  )
}
