import Link from "next/link"
import { Button } from "@/components/ui/button"
import { MarketingHeader } from "@/components/marketing/header"
import { MarketingFooter } from "@/components/marketing/footer"
import {
  Shield,
  ShieldCheck,
  WifiOff,
  KeyRound,
  ArrowRight,
  Send,
  CheckCircle2,
  FileCheck,
  Play,
  Zap,
  Lock,
  Globe,
} from "lucide-react"

const logos = ["Nubank", "iFood", "TOTVS", "Creditas", "Stone"]

const steps = [
  { icon: Send, title: "Enviar", description: "Sua aplicacao envia uma decisao via SDK ou gateway. Nenhum dado sensivel sai do seu controle." },
  { icon: CheckCircle2, title: "Decidir", description: "O motor TDLN avalia politicas, gera provas criptograficas e assina cada etapa do protocolo SIRP." },
  { icon: FileCheck, title: "Provar", description: "Um recibo verificavel e imutavel e gerado com CID unico. Baixe o bundle para verificacao offline." },
]

const features = [
  { icon: ShieldCheck, title: "Verificavel", description: "Cada decisao gera provas criptograficas assinadas e encadeadas via protocolo SIRP em 4 etapas." },
  { icon: WifiOff, title: "Bundle offline", description: "Baixe o bundle .zip e verifique sem backend. Auditoria funciona mesmo sem internet." },
  { icon: KeyRound, title: "Sem custodia", description: "Seus dados e chaves permanecem sob seu controle. Zero vendor lock-in, export completo a qualquer momento." },
]

const trustItems = [
  { icon: Lock, title: "AEAD encryption", description: "Evidencias sensiveis protegidas com criptografia autenticada." },
  { icon: Globe, title: "Multi-regiao", description: "Dados armazenados na regiao de sua escolha. Conformidade LGPD/GDPR nativa." },
  { icon: Zap, title: "p99 < 150ms", description: "Decisoes em tempo real com latencia sub-150ms globalmente." },
  { icon: Shield, title: "SOC 2 + ISO 27001", description: "Infraestrutura auditada e certificada para ambientes regulados." },
]

export default function LandingPage() {
  return (
    <div className="flex min-h-screen flex-col">
      <MarketingHeader />

      <main className="flex-1">
        {/* Hero */}
        <section className="relative overflow-hidden bg-foreground text-background">
          <div className="mx-auto max-w-7xl px-4 py-24 lg:px-8 lg:py-32">
            <div className="mx-auto max-w-3xl text-center">
              <div className="mb-6 inline-flex items-center gap-2 rounded-full border border-background/10 bg-background/5 px-4 py-1.5 text-sm text-background/80">
                <span className="flex h-2 w-2 rounded-full bg-emerald-400" />
                Protocolo SIRP — verificacao criptografica em 4 etapas
              </div>
              <h1 className="text-balance text-4xl font-bold tracking-tight md:text-6xl lg:text-7xl">
                Prova criptografica para cada decisao
              </h1>
              <p className="mt-6 text-pretty text-lg text-background/70 leading-relaxed md:text-xl">
                Gere recibos verificaveis, imutaveis e auditaveis para qualquer decisao da sua aplicacao. Verificacao offline, sem custodia e sem vendor lock-in.
              </p>
              <div className="mt-8 flex flex-col items-center justify-center gap-3 sm:flex-row">
                <Button size="lg" className="bg-background text-foreground shadow-lg hover:bg-background/90 hover:shadow-xl transition-shadow" asChild>
                  <Link href="/onboarding">
                    Comecar gratis
                    <ArrowRight className="ml-2 h-4 w-4" />
                  </Link>
                </Button>
                <Button size="lg" variant="outline" className="border-background/20 bg-transparent text-background hover:bg-background/10" asChild>
                  <Link href="#demo">
                    <Play className="mr-2 h-4 w-4" />
                    Ver demo
                  </Link>
                </Button>
              </div>
            </div>
          </div>
          {/* Stats row */}
          <div className="mx-auto max-w-7xl px-4 pb-16 lg:px-8">
            <div className="mx-auto grid max-w-3xl grid-cols-2 gap-6 sm:grid-cols-4">
              {[
                { value: "2M+", label: "execucoes/mes" },
                { value: "99.97%", label: "uptime" },
                { value: "<142ms", label: "latencia p99" },
                { value: "3", label: "regioes globais" },
              ].map((stat) => (
                <div key={stat.label} className="text-center">
                  <p className="text-2xl font-bold tracking-tight text-background md:text-3xl">{stat.value}</p>
                  <p className="mt-1 text-xs text-background/50">{stat.label}</p>
                </div>
              ))}
            </div>
          </div>

          {/* Subtle grid pattern */}
          <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(rgba(255,255,255,0.02)_1px,transparent_1px),linear-gradient(90deg,rgba(255,255,255,0.02)_1px,transparent_1px)] bg-[size:64px_64px]" aria-hidden="true" />
        </section>

        {/* Logos */}
        <section className="border-b bg-muted/30 py-8">
          <div className="mx-auto max-w-7xl px-4 lg:px-8">
            <p className="mb-6 text-center text-xs font-medium uppercase tracking-widest text-muted-foreground">
              Confiado por equipes de engenharia e compliance
            </p>
            <div className="flex flex-wrap items-center justify-center gap-8 lg:gap-16">
              {logos.map((name) => (
                <span key={name} className="text-lg font-bold text-muted-foreground/40">{name}</span>
              ))}
            </div>
          </div>
        </section>

        {/* Features - 3 bullets */}
        <section className="py-20 lg:py-28" id="produto">
          <div className="mx-auto max-w-7xl px-4 lg:px-8">
            <div className="mx-auto max-w-2xl text-center">
              <h2 className="text-balance text-3xl font-bold tracking-tight text-foreground md:text-4xl">
                Verificacao que voce controla
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                Tres pilares que diferenciam TDLN de qualquer solucao de auditoria tradicional.
              </p>
            </div>
            <div className="mt-16 grid gap-8 md:grid-cols-3">
              {features.map((feature) => (
                <div key={feature.title} className="rounded-xl border bg-card p-6">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-foreground">
                    <feature.icon className="h-5 w-5 text-background" />
                  </div>
                  <h3 className="mt-4 text-lg font-semibold text-foreground">{feature.title}</h3>
                  <p className="mt-2 text-sm text-muted-foreground leading-relaxed">{feature.description}</p>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* SIRP callout */}
        <section className="border-y bg-muted/30 py-20 lg:py-28">
          <div className="mx-auto max-w-7xl px-4 lg:px-8">
            <div className="grid items-center gap-12 lg:grid-cols-2">
              <div>
                <span className="inline-flex items-center gap-2 rounded-full bg-foreground px-3 py-1 text-xs font-semibold text-background">
                  SIRP Protocol
                </span>
                <h2 className="mt-4 text-balance text-3xl font-bold tracking-tight text-foreground md:text-4xl">
                  Quatro etapas. Prova completa.
                </h2>
                <p className="mt-4 text-muted-foreground leading-relaxed">
                  O protocolo SIRP (Send-Inspect-Receive-Prove) encadeia assinaturas criptograficas em cada etapa do fluxo de decisao, criando uma cadeia de custodia verificavel e imutavel.
                </p>
                <Button className="mt-6" asChild>
                  <Link href="/docs">
                    Ler documentacao
                    <ArrowRight className="ml-2 h-4 w-4" />
                  </Link>
                </Button>
              </div>
              <div className="rounded-xl border bg-card p-6">
                <div className="space-y-4">
                  {["INTENT", "DELIVERY", "EXECUTION", "RESULT"].map((step, i) => (
                    <div key={step} className="flex items-center gap-4">
                      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border-2 border-emerald-500 bg-emerald-500/10 text-xs font-bold text-emerald-600">
                        {i + 1}
                      </div>
                      <div className="flex-1">
                        <p className="text-sm font-semibold text-foreground">{step}</p>
                        <p className="text-xs text-muted-foreground">
                          {step === "INTENT" && "Requisicao assinada pelo cliente SDK"}
                          {step === "DELIVERY" && "Recibo do gateway (Nginx/Envoy)"}
                          {step === "EXECUTION" && "Prova de processamento do motor"}
                          {step === "RESULT" && "Decisao final com hash encadeado"}
                        </p>
                      </div>
                      <CheckCircle2 className="h-5 w-5 shrink-0 text-emerald-500" />
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* How it works - 3 steps */}
        <section className="py-20 lg:py-28">
          <div className="mx-auto max-w-7xl px-4 lg:px-8">
            <div className="mx-auto max-w-2xl text-center">
              <h2 className="text-balance text-3xl font-bold tracking-tight text-foreground md:text-4xl">
                Como funciona
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                De uma chamada de API a um recibo verificavel em menos de 150ms.
              </p>
            </div>
            <div className="mt-16 grid gap-8 md:grid-cols-3">
              {steps.map((step, i) => (
                <div key={step.title} className="relative text-center">
                  <div className="mx-auto flex h-14 w-14 items-center justify-center rounded-2xl bg-foreground text-background">
                    <step.icon className="h-6 w-6" />
                  </div>
                  <span className="mt-3 block text-xs font-medium text-muted-foreground">Passo {i + 1}</span>
                  <h3 className="mt-1 text-lg font-semibold text-foreground">{step.title}</h3>
                  <p className="mt-2 text-sm text-muted-foreground leading-relaxed">{step.description}</p>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Video placeholder */}
        <section className="border-y bg-muted/30 py-20" id="demo">
          <div className="mx-auto max-w-4xl px-4 lg:px-8">
            <div className="mx-auto max-w-2xl text-center">
              <h2 className="text-balance text-2xl font-bold tracking-tight text-foreground">
                Veja o TDLN em acao
              </h2>
              <p className="mt-2 text-sm text-muted-foreground">Demo de 30 segundos: do SDK ao recibo verificavel.</p>
            </div>
            <div className="mt-8 overflow-hidden rounded-xl border bg-foreground/95">
              <div className="flex aspect-video items-center justify-center">
                <button className="flex h-16 w-16 items-center justify-center rounded-full bg-background/10 text-background transition-transform hover:scale-110 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" aria-label="Reproduzir video demo">
                  <Play className="ml-1 h-6 w-6" />
                </button>
              </div>
            </div>
          </div>
        </section>

        {/* Trust / Security */}
        <section className="py-20 lg:py-28" id="seguranca">
          <div className="mx-auto max-w-7xl px-4 lg:px-8">
            <div className="mx-auto max-w-2xl text-center">
              <h2 className="text-balance text-3xl font-bold tracking-tight text-foreground md:text-4xl">
                Confianca por design
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                Seguranca e privacidade sao fundamentos, nao features opcionais.
              </p>
            </div>
            <div className="mt-16 grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
              {trustItems.map((item) => (
                <div key={item.title} className="rounded-xl border bg-card p-5">
                  <item.icon className="h-5 w-5 text-foreground" />
                  <h3 className="mt-3 text-sm font-semibold text-foreground">{item.title}</h3>
                  <p className="mt-1 text-xs text-muted-foreground leading-relaxed">{item.description}</p>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Social proof */}
        <section className="border-t bg-muted/30 py-20">
          <div className="mx-auto max-w-7xl px-4 lg:px-8">
            <div className="mx-auto max-w-3xl text-center">
              <blockquote className="text-xl font-medium text-foreground leading-relaxed md:text-2xl">
                {'"'}TDLN nos deu a prova criptografica que precisavamos para compliance sem mudar nossa infra. O bundle offline foi decisivo para nossos auditores.{'"'}
              </blockquote>
              <div className="mt-6">
                <p className="text-sm font-semibold text-foreground">Rafael Costa</p>
                <p className="text-sm text-muted-foreground">Head of Engineering, Fintech Corp</p>
              </div>
            </div>
          </div>
        </section>

        {/* CTA */}
        <section className="bg-foreground py-20 text-background">
          <div className="mx-auto max-w-3xl px-4 text-center lg:px-8">
            <h2 className="text-balance text-3xl font-bold tracking-tight md:text-4xl">
              Comece a gerar provas verificaveis hoje
            </h2>
            <p className="mt-4 text-background/70 leading-relaxed">
              Plano gratuito com ate 500 execucoes/mes. Sem cartao de credito.
            </p>
            <div className="mt-8 flex flex-col items-center justify-center gap-3 sm:flex-row">
              <Button size="lg" className="bg-background text-foreground hover:bg-background/90" asChild>
                <Link href="/onboarding">
                  Comecar gratis
                  <ArrowRight className="ml-2 h-4 w-4" />
                </Link>
              </Button>
              <Button size="lg" variant="outline" className="border-background/20 bg-transparent text-background hover:bg-background/10" asChild>
                <Link href="/pricing">Ver precos</Link>
              </Button>
            </div>
          </div>
        </section>
      </main>

      <MarketingFooter />
    </div>
  )
}
