import { Shield, Lock, Globe, Eye, Server, Key } from "lucide-react"

export const metadata = { title: "Seguranca" }

const practices = [
  { icon: Lock, title: "Criptografia end-to-end", description: "TLS 1.3 em transito. AES-256-GCM em repouso. AEAD (XChaCha20-Poly1305) para evidencias sensiveis. Chaves rotacionadas automaticamente." },
  { icon: Shield, title: "SOC 2 Type II & ISO 27001", description: "Infraestrutura auditada anualmente por auditores independentes. Relatorios disponiveis para clientes Enterprise sob NDA." },
  { icon: Globe, title: "Multi-regiao isolada", description: "Dados processados e armazenados na regiao escolhida (BR, US, EU). Sem transferencia automatica entre regioes." },
  { icon: Eye, title: "Auditoria imutavel", description: "Cada acao no console gera um registro imutavel no audit log. Logs nao podem ser alterados ou excluidos, mesmo por Owners." },
  { icon: Server, title: "Infraestrutura hardened", description: "Containers ephemeros, network isolation, zero trust. Acesso a infraestrutura requer MFA + VPN + aprovacao dual." },
  { icon: Key, title: "Zero custodia", description: "Suas chaves sao suas. Sem acesso a payloads em texto claro. Evidencias protegidas so podem ser descriptografadas pelo titular." },
]

export default function SecurityPage() {
  return (
    <div>
      <h1 className="text-3xl font-bold tracking-tight text-foreground">Seguranca</h1>
      <p className="mt-2 text-muted-foreground leading-relaxed">
        Seguranca nao e uma feature — e o fundamento de tudo que construimos. Conhea nossas praticas.
      </p>

      <div className="mt-12 grid gap-6 sm:grid-cols-2">
        {practices.map((p) => (
          <div key={p.title} className="rounded-xl border bg-card p-6">
            <p.icon className="h-5 w-5 text-foreground" />
            <h3 className="mt-3 text-sm font-semibold text-foreground">{p.title}</h3>
            <p className="mt-1 text-xs text-muted-foreground leading-relaxed">{p.description}</p>
          </div>
        ))}
      </div>

      <div className="mt-12 rounded-xl border bg-muted/30 p-6">
        <h2 className="text-lg font-bold text-foreground">Divulgacao responsavel</h2>
        <p className="mt-2 text-sm text-muted-foreground leading-relaxed">
          Encontrou uma vulnerabilidade? Envie para <strong className="text-foreground">security@tdln.io</strong>. Respondemos em ate 24h e temos um programa de recompensas ativo para pesquisadores de seguranca.
        </p>
      </div>
    </div>
  )
}
