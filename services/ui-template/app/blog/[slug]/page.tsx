import Link from "next/link"
import { MarketingHeader } from "@/components/marketing/header"
import { MarketingFooter } from "@/components/marketing/footer"
import { ArrowLeft } from "lucide-react"
import { Button } from "@/components/ui/button"

const posts: Record<string, { title: string; date: string; category: string; content: string[] }> = {
  "introducing-sirp-protocol": {
    title: "Apresentando o protocolo SIRP: 4 etapas para prova completa",
    date: "2026-02-05",
    category: "Produto",
    content: [
      "O protocolo SIRP (Send-Inspect-Receive-Prove) e o nucleo do TDLN. Cada decisao processada passa por 4 etapas que geram uma cadeia de custodia criptografica completa e verificavel.",
      "Na etapa INTENT, o cliente SDK assina a requisicao com Ed25519, garantindo autenticidade da origem. Na etapa DELIVERY, o gateway (Nginx ou Envoy) gera um recibo HMAC-SHA256 confirmando o transporte.",
      "A etapa EXECUTION registra o processamento pelo motor TDLN com prova de computacao. Finalmente, RESULT encadeia todos os hashes anteriores em um recibo final assinado com CID unico.",
      "O resultado e um bundle verificavel offline que contem todas as provas, assinaturas e metadados necessarios para auditoria independente — sem precisar consultar o backend do TDLN.",
    ],
  },
  "offline-verification-bundles": {
    title: "Verificacao offline: por que seus auditores vao agradecer",
    date: "2026-01-22",
    category: "Engenharia",
    content: [
      "Um dos diferenciais mais valorizados do TDLN e a capacidade de verificar provas criptograficas completamente offline. O bundle .zip contem tudo que um auditor precisa.",
      "Cada bundle inclui: metadados da execucao, timeline SIRP completa, provas criptograficas individuais, evidencias indexadas e um script de verificacao auto-contido.",
      "Para verificar, basta extrair o ZIP e executar o verificador incluido — ou usar nossa ferramenta web offline em /verify/offline. Nenhuma conexao com a internet e necessaria.",
      "Isso e particularmente valioso em auditorias regulatorias onde o auditor nao tem (e nao deveria ter) acesso ao seu backend.",
    ],
  },
}

export default async function BlogPostPage({ params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params
  const post = posts[slug]

  if (!post) {
    return (
      <div className="flex min-h-screen flex-col">
        <MarketingHeader />
        <main className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <h1 className="text-2xl font-bold text-foreground">Post nao encontrado</h1>
            <p className="mt-2 text-muted-foreground">Este artigo ainda nao foi publicado.</p>
            <Button variant="outline" className="mt-4 bg-transparent" asChild>
              <Link href="/blog"><ArrowLeft className="mr-2 h-4 w-4" />Voltar ao blog</Link>
            </Button>
          </div>
        </main>
        <MarketingFooter />
      </div>
    )
  }

  return (
    <div className="flex min-h-screen flex-col">
      <MarketingHeader />
      <main className="flex-1">
        <article className="mx-auto max-w-3xl px-4 py-16 lg:px-8">
          <Link href="/blog" className="inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground">
            <ArrowLeft className="h-3.5 w-3.5" />
            Voltar ao blog
          </Link>
          <div className="mt-6 flex items-center gap-3 text-xs text-muted-foreground">
            <span className="rounded-full bg-muted px-2.5 py-0.5 font-medium text-foreground">{post.category}</span>
            <time dateTime={post.date}>{new Date(post.date).toLocaleDateString("pt-BR", { day: "numeric", month: "long", year: "numeric" })}</time>
          </div>
          <h1 className="mt-4 text-balance text-3xl font-bold tracking-tight text-foreground md:text-4xl">{post.title}</h1>
          <div className="mt-8 space-y-4">
            {post.content.map((p, i) => (
              <p key={i} className="text-muted-foreground leading-relaxed">{p}</p>
            ))}
          </div>
        </article>
      </main>
      <MarketingFooter />
    </div>
  )
}
