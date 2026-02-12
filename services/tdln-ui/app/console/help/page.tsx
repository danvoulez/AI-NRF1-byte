"use client"

import { Card, CardContent } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import {
  BookOpen,
  Code,
  MessageCircle,
  Rocket,
  ExternalLink,
  FileText,
  Webhook,
  Shield,
  Users,
  HelpCircle,
  ArrowRight,
} from "lucide-react"

const guides = [
  {
    icon: Rocket,
    title: "Quickstart",
    description: "Crie seu primeiro recibo verificavel em menos de 5 minutos.",
    href: "/docs",
  },
  {
    icon: Code,
    title: "SDKs & APIs",
    description: "Documentacao completa dos SDKs (Node.js, Python, Rust) e API REST.",
    href: "/console/integrations",
  },
  {
    icon: Webhook,
    title: "Webhooks",
    description: "Configure endpoints para receber eventos em tempo real.",
    href: "/console/integrations",
  },
  {
    icon: Shield,
    title: "Politicas & Compliance",
    description: "Gerencie packs de regras e entenda o fluxo SIRP.",
    href: "/console/policies",
  },
  {
    icon: Users,
    title: "Equipe & RBAC",
    description: "Convide membros, atribua papeis e configure MFA.",
    href: "/console/team",
  },
  {
    icon: FileText,
    title: "Verificacao Offline",
    description: "Como baixar e validar bundles sem conexao com o backend.",
    href: "/verify/offline",
  },
]

const supportChannels = [
  {
    title: "Documentacao",
    description: "Guias, referencias de API e exemplos de codigo.",
    action: "Abrir docs",
    href: "/docs",
    icon: BookOpen,
  },
  {
    title: "Suporte por email",
    description: "Resposta em ate 24h (Team) ou 4h (Enterprise).",
    action: "Enviar email",
    href: "mailto:suporte@tdln.io",
    icon: MessageCircle,
  },
  {
    title: "Status & Changelog",
    description: "Status da plataforma, incidentes e notas de versao.",
    action: "Ver status",
    href: "/changelog",
    icon: HelpCircle,
  },
]

export default function HelpPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-foreground">Ajuda & Docs</h1>
        <p className="mt-1 text-sm text-muted-foreground">Guias, documentacao e canais de suporte para aproveitar o TDLN ao maximo.</p>
      </div>

      {/* Quick guides */}
      <div>
        <h2 className="text-sm font-semibold text-foreground">Guias rapidos</h2>
        <div className="mt-3 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {guides.map((guide) => (
            <Link key={guide.title} href={guide.href}>
              <Card className="h-full transition-colors hover:bg-muted/50">
                <CardContent className="p-5">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-muted">
                    <guide.icon className="h-5 w-5 text-foreground" />
                  </div>
                  <h3 className="mt-3 text-sm font-semibold text-foreground">{guide.title}</h3>
                  <p className="mt-1 text-xs text-muted-foreground leading-relaxed">{guide.description}</p>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      </div>

      {/* Support channels */}
      <div>
        <h2 className="text-sm font-semibold text-foreground">Canais de suporte</h2>
        <div className="mt-3 grid gap-4 sm:grid-cols-3">
          {supportChannels.map((channel) => (
            <Card key={channel.title}>
              <CardContent className="flex flex-col items-start p-5">
                <channel.icon className="h-5 w-5 text-foreground" />
                <h3 className="mt-3 text-sm font-semibold text-foreground">{channel.title}</h3>
                <p className="mt-1 flex-1 text-xs text-muted-foreground leading-relaxed">{channel.description}</p>
                <Button variant="outline" size="sm" className="mt-4 bg-transparent" asChild>
                  <Link href={channel.href}>
                    {channel.action}
                    <ExternalLink className="ml-1.5 h-3 w-3" />
                  </Link>
                </Button>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>

      {/* FAQ */}
      <Card>
        <CardContent className="p-6">
          <h2 className="text-sm font-semibold text-foreground">Perguntas frequentes</h2>
          <div className="mt-4 space-y-4">
            {[
              {
                q: "O que acontece se eu exceder o limite de execucoes?",
                a: "Voce recebera um aviso no console e por email. Verificacoes existentes continuam acessiveis. Para novas execucoes, faca upgrade do plano.",
              },
              {
                q: "Como funciona a verificacao offline?",
                a: "Baixe o bundle .zip de qualquer recibo. Ele contem todas as provas, assinaturas e metadados necessarios para validacao independente, sem precisar de conexao com o TDLN.",
              },
              {
                q: "Posso exportar todos os meus dados?",
                a: "Sim, a qualquer momento. Va em Configuracoes > Exportar dados. Sem vendor lock-in.",
              },
              {
                q: "O TDLN armazena dados sensiveis?",
                a: "Evidencias sensiveis sao protegidas com AEAD (criptografia autenticada). No console, aparecem como \"conteudo protegido\". Voce controla as chaves.",
              },
              {
                q: "Como configuro SSO/SAML?",
                a: "SSO/SAML e SCIM estao disponiveis no plano Enterprise. Entre em contato com vendas para configuracao assistida.",
              },
            ].map((faq) => (
              <div key={faq.q} className="rounded-lg border p-4">
                <p className="text-sm font-medium text-foreground">{faq.q}</p>
                <p className="mt-1 text-xs text-muted-foreground leading-relaxed">{faq.a}</p>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* CTA */}
      <div className="flex items-center justify-between rounded-xl border bg-muted/30 p-6">
        <div>
          <p className="text-sm font-semibold text-foreground">Nao encontrou o que precisa?</p>
          <p className="mt-0.5 text-xs text-muted-foreground">Nossa equipe esta pronta para ajudar.</p>
        </div>
        <Button size="sm" asChild>
          <a href="mailto:suporte@tdln.io">
            Falar com suporte
            <ArrowRight className="ml-1.5 h-4 w-4" />
          </a>
        </Button>
      </div>
    </div>
  )
}
