import Link from "next/link"
import { MarketingHeader } from "@/components/marketing/header"
import { MarketingFooter } from "@/components/marketing/footer"
import { Shield, Users, Globe, Zap } from "lucide-react"

export const metadata = { title: "Sobre" }

const values = [
  { icon: Shield, title: "Confianca por design", description: "Cada decisao arquitetural prioriza seguranca e verificabilidade acima de conveniencia." },
  { icon: Users, title: "Transparencia radical", description: "Codigo aberto no nucleo, auditoria publica e zero custodia sobre seus dados." },
  { icon: Globe, title: "Global, local", description: "Infraestrutura multi-regiao com conformidade nativa a LGPD, GDPR e regulacoes locais." },
  { icon: Zap, title: "Performance e simplicidade", description: "Decisoes em sub-150ms. APIs intuitivas. Integracao em minutos, nao semanas." },
]

const team = [
  { name: "Rafael Costa", role: "CEO & Co-founder", bio: "Ex-Engenheiro Principal em fintechs de pagamentos. 15 anos em criptografia aplicada." },
  { name: "Marina Alves", role: "CTO & Co-founder", bio: "Ph.D. em Ciencia da Computacao. Contribuidora de protocolos de verificacao distribuida." },
  { name: "Diego Ferreira", role: "VP Engineering", bio: "Ex-lider de plataforma em scale-ups. Especialista em sistemas de alta disponibilidade." },
  { name: "Ana Rodrigues", role: "Head of Product", bio: "Background em compliance regulatorio e UX para ferramentas de auditoria." },
]

export default function AboutPage() {
  return (
    <div className="flex min-h-screen flex-col">
      <MarketingHeader />
      <main className="flex-1">
        <section className="border-b py-20 lg:py-28">
          <div className="mx-auto max-w-3xl px-4 text-center lg:px-8">
            <h1 className="text-balance text-4xl font-bold tracking-tight text-foreground md:text-5xl">
              Prova criptografica acessivel para todos
            </h1>
            <p className="mt-6 text-lg text-muted-foreground leading-relaxed">
              TDLN nasceu da necessidade de provar decisoes de forma verificavel, auditavel e sem custodia. Construimos a infraestrutura para que qualquer equipe possa gerar provas criptograficas sem precisar ser especialista em criptografia.
            </p>
          </div>
        </section>

        <section className="py-20 lg:py-28">
          <div className="mx-auto max-w-7xl px-4 lg:px-8">
            <h2 className="text-center text-2xl font-bold tracking-tight text-foreground">Nossos valores</h2>
            <div className="mt-12 grid gap-8 sm:grid-cols-2 lg:grid-cols-4">
              {values.map((v) => (
                <div key={v.title} className="rounded-xl border bg-card p-6">
                  <v.icon className="h-5 w-5 text-foreground" />
                  <h3 className="mt-3 text-sm font-semibold text-foreground">{v.title}</h3>
                  <p className="mt-1 text-xs text-muted-foreground leading-relaxed">{v.description}</p>
                </div>
              ))}
            </div>
          </div>
        </section>

        <section className="border-t bg-muted/30 py-20 lg:py-28">
          <div className="mx-auto max-w-7xl px-4 lg:px-8">
            <h2 className="text-center text-2xl font-bold tracking-tight text-foreground">Lideranca</h2>
            <div className="mt-12 grid gap-8 sm:grid-cols-2 lg:grid-cols-4">
              {team.map((t) => (
                <div key={t.name} className="rounded-xl border bg-card p-6">
                  <div className="flex h-12 w-12 items-center justify-center rounded-full bg-muted text-sm font-bold text-foreground">
                    {t.name.split(" ").map((n) => n[0]).join("")}
                  </div>
                  <h3 className="mt-4 text-sm font-semibold text-foreground">{t.name}</h3>
                  <p className="text-xs font-medium text-muted-foreground">{t.role}</p>
                  <p className="mt-2 text-xs text-muted-foreground leading-relaxed">{t.bio}</p>
                </div>
              ))}
            </div>
          </div>
        </section>
      </main>
      <MarketingFooter />
    </div>
  )
}
