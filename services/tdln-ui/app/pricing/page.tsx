import Link from "next/link"
import { Button } from "@/components/ui/button"
import { MarketingHeader } from "@/components/marketing/header"
import { MarketingFooter } from "@/components/marketing/footer"
import { Check, ArrowRight } from "lucide-react"

const plans = [
  {
    name: "Free",
    price: "R$ 0",
    period: "/mes",
    description: "Para experimentar e projetos pessoais.",
    cta: "Comecar gratis",
    ctaVariant: "outline" as const,
    highlighted: false,
    features: [
      "500 execucoes/mes",
      "1 integracao",
      "Retencao 7 dias",
      "Verificacao offline (com watermark)",
      "SDK Node.js",
      "Suporte comunidade",
    ],
  },
  {
    name: "Team",
    price: "R$ 990",
    period: "/mes",
    description: "Para equipes que precisam de compliance e auditoria.",
    cta: "Iniciar trial de 14 dias",
    ctaVariant: "default" as const,
    highlighted: true,
    features: [
      "10.000 execucoes/mes",
      "Integracoes ilimitadas",
      "Retencao 30 dias",
      "Verificacao offline (sem watermark)",
      "SDKs Node.js, Python, Rust",
      "RBAC (Owner, Admin, Operator, Auditor)",
      "Webhooks + replays",
      "Audit log exportavel",
      "Suporte prioritario",
    ],
  },
  {
    name: "Enterprise",
    price: "Sob consulta",
    period: "",
    description: "Para organizacoes reguladas com requisitos avancados.",
    cta: "Falar com vendas",
    ctaVariant: "outline" as const,
    highlighted: false,
    features: [
      "Execucoes ilimitadas",
      "SSO/SAML + SCIM",
      "Regiao dedicada",
      "Retencao 365+ dias",
      "Integracao SIEM",
      "SLA 99.99%",
      "Suporte 24/7 dedicado",
      "Treinamento e onboarding",
      "Verificacao offline completa",
      "Auditoria sem vendor lock-in",
    ],
  },
]

export default function PricingPage() {
  return (
    <div className="flex min-h-screen flex-col">
      <MarketingHeader />
      <main className="flex-1">
        <section className="py-20 lg:py-28">
          <div className="mx-auto max-w-7xl px-4 lg:px-8">
            <div className="mx-auto max-w-2xl text-center">
              <h1 className="text-balance text-3xl font-bold tracking-tight text-foreground md:text-5xl">
                Precos simples, verificacao poderosa
              </h1>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                Comece gratis. Escale conforme sua necessidade. Todos os planos incluem verificacao offline e auditoria sem vendor lock-in.
              </p>
            </div>
            <div className="mt-16 grid gap-8 lg:grid-cols-3">
              {plans.map((plan) => (
                <div
                  key={plan.name}
                  className={`relative rounded-xl border p-8 ${
                    plan.highlighted
                      ? "border-foreground bg-card shadow-lg ring-1 ring-foreground"
                      : "bg-card"
                  }`}
                >
                  {plan.highlighted && (
                    <span className="absolute -top-3 left-1/2 -translate-x-1/2 rounded-full bg-foreground px-3 py-1 text-xs font-semibold text-background">
                      Mais popular
                    </span>
                  )}
                  <h3 className="text-lg font-semibold text-foreground">{plan.name}</h3>
                  <div className="mt-4 flex items-baseline gap-1">
                    <span className="text-4xl font-bold tracking-tight text-foreground">{plan.price}</span>
                    {plan.period && <span className="text-sm text-muted-foreground">{plan.period}</span>}
                  </div>
                  <p className="mt-2 text-sm text-muted-foreground">{plan.description}</p>
                  <Button
                    className="mt-6 w-full"
                    variant={plan.ctaVariant}
                    asChild
                  >
                    <Link href={plan.name === "Enterprise" ? "/contact" : "/onboarding"}>
                      {plan.cta}
                      <ArrowRight className="ml-2 h-4 w-4" />
                    </Link>
                  </Button>
                  <ul className="mt-8 space-y-3">
                    {plan.features.map((feature) => (
                      <li key={feature} className="flex items-start gap-3 text-sm">
                        <Check className="mt-0.5 h-4 w-4 shrink-0 text-emerald-500" />
                        <span className="text-muted-foreground">{feature}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              ))}
            </div>

            {/* Comparison highlights */}
            <div className="mx-auto mt-20 max-w-3xl">
              <h2 className="text-center text-xl font-semibold text-foreground">Todos os planos incluem</h2>
              <div className="mt-8 grid gap-4 sm:grid-cols-2">
                {[
                  "Verificacao offline via bundle",
                  "Auditoria sem vendor lock-in",
                  "Protocolo SIRP completo",
                  "Export de dados a qualquer momento",
                  "Criptografia Ed25519",
                  "API REST + Webhooks",
                ].map((item) => (
                  <div key={item} className="flex items-center gap-3 rounded-lg border bg-card p-4">
                    <Check className="h-4 w-4 shrink-0 text-emerald-500" />
                    <span className="text-sm text-foreground">{item}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </section>
      </main>
      <MarketingFooter />
    </div>
  )
}
