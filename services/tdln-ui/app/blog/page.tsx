import Link from "next/link"
import { MarketingHeader } from "@/components/marketing/header"
import { MarketingFooter } from "@/components/marketing/footer"
import { ArrowRight } from "lucide-react"

export const metadata = { title: "Blog" }

const posts = [
  {
    title: "Apresentando o protocolo SIRP: 4 etapas para prova completa",
    excerpt: "Como o protocolo Send-Inspect-Receive-Prove cria uma cadeia de custodia criptografica verificavel e imutavel para cada decisao.",
    date: "2026-02-05",
    category: "Produto",
    slug: "introducing-sirp-protocol",
  },
  {
    title: "Verificacao offline: por que seus auditores vao agradecer",
    excerpt: "Bundles .zip auto-contidos permitem validar provas criptograficas sem conexao com o backend. Saiba como funciona.",
    date: "2026-01-22",
    category: "Engenharia",
    slug: "offline-verification-bundles",
  },
  {
    title: "TDLN + LGPD: conformidade nativa sem complexidade",
    excerpt: "Como a arquitetura multi-regiao e AEAD encryption do TDLN simplificam conformidade com a LGPD e GDPR.",
    date: "2026-01-10",
    category: "Compliance",
    slug: "tdln-lgpd-compliance",
  },
  {
    title: "De zero a primeiro recibo em 5 minutos com o SDK Node.js",
    excerpt: "Tutorial passo a passo para integrar verificacao criptografica na sua aplicacao Node.js usando o SDK oficial.",
    date: "2025-12-18",
    category: "Tutorial",
    slug: "quickstart-nodejs-sdk",
  },
  {
    title: "AML Sanctions Pack: como funciona a verificacao em tempo real",
    excerpt: "Detalhes tecnicos do pack de sancoes anti-lavagem: listas OFAC, EU, ONU e screening PEP com decisao sub-150ms.",
    date: "2025-12-05",
    category: "Produto",
    slug: "aml-sanctions-pack-deep-dive",
  },
]

export default function BlogPage() {
  return (
    <div className="flex min-h-screen flex-col">
      <MarketingHeader />
      <main className="flex-1">
        <section className="border-b py-20">
          <div className="mx-auto max-w-3xl px-4 text-center lg:px-8">
            <h1 className="text-balance text-4xl font-bold tracking-tight text-foreground md:text-5xl">Blog</h1>
            <p className="mt-4 text-muted-foreground leading-relaxed">
              Engenharia, produto e compliance. Acompanhe o que estamos construindo.
            </p>
          </div>
        </section>

        <section className="py-16">
          <div className="mx-auto max-w-4xl px-4 lg:px-8">
            <div className="divide-y">
              {posts.map((post) => (
                <article key={post.slug} className="py-8 first:pt-0 last:pb-0">
                  <div className="flex items-center gap-3 text-xs text-muted-foreground">
                    <span className="rounded-full bg-muted px-2.5 py-0.5 font-medium text-foreground">{post.category}</span>
                    <time dateTime={post.date}>{new Date(post.date).toLocaleDateString("pt-BR", { day: "numeric", month: "long", year: "numeric" })}</time>
                  </div>
                  <h2 className="mt-3 text-xl font-bold tracking-tight text-foreground">
                    <Link href={`/blog/${post.slug}`} className="hover:underline">{post.title}</Link>
                  </h2>
                  <p className="mt-2 text-sm text-muted-foreground leading-relaxed">{post.excerpt}</p>
                  <Link href={`/blog/${post.slug}`} className="mt-3 inline-flex items-center gap-1 text-sm font-medium text-foreground hover:underline">
                    Ler mais <ArrowRight className="h-3.5 w-3.5" />
                  </Link>
                </article>
              ))}
            </div>
          </div>
        </section>
      </main>
      <MarketingFooter />
    </div>
  )
}
