import Link from "next/link"
import { Shield } from "lucide-react"

const footerLinks = {
  Produto: [
    { label: "Funcionalidades", href: "/#produto" },
    { label: "Precos", href: "/pricing" },
    { label: "Seguranca", href: "/#seguranca" },
    { label: "Changelog", href: "/changelog" },
  ],
  Desenvolvedores: [
    { label: "Documentacao", href: "/docs" },
    { label: "SDKs", href: "/docs?section=sdks" },
    { label: "API REST", href: "/docs?section=api-rest" },
    { label: "Webhooks", href: "/docs?section=webhooks" },
  ],
  Empresa: [
    { label: "Sobre", href: "/about" },
    { label: "Blog", href: "/blog" },
    { label: "Contato", href: "/contact" },
    { label: "Status", href: "/changelog" },
  ],
  Legal: [
    { label: "Privacidade", href: "/legal/privacy" },
    { label: "Termos", href: "/legal/terms" },
    { label: "LGPD", href: "/legal/lgpd" },
    { label: "Seguranca", href: "/legal/security" },
  ],
}

export function MarketingFooter() {
  return (
    <footer className="border-t bg-muted/30">
      <div className="mx-auto max-w-7xl px-4 py-12 lg:px-8">
        <div className="grid gap-8 md:grid-cols-2 lg:grid-cols-5">
          <div className="lg:col-span-1">
            <Link href="/" className="flex items-center gap-2" aria-label="TDLN Home">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-foreground">
                <Shield className="h-4 w-4 text-background" />
              </div>
              <span className="text-lg font-bold tracking-tight text-foreground">TDLN</span>
            </Link>
            <p className="mt-3 text-sm text-muted-foreground leading-relaxed">
              Prova criptografica verificavel para cada decisao. Sem custodia, sem vendor lock-in.
            </p>
          </div>
          {Object.entries(footerLinks).map(([category, links]) => (
            <div key={category}>
              <h3 className="text-sm font-semibold text-foreground">{category}</h3>
              <ul className="mt-3 space-y-2">
                {links.map((link) => (
                  <li key={link.label}>
                    <Link href={link.href} className="text-sm text-muted-foreground transition-colors hover:text-foreground">
                      {link.label}
                    </Link>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
        <div className="mt-12 border-t pt-6">
          <p className="text-xs text-muted-foreground">
            &copy; 2026 TDLN. Todos os direitos reservados.
          </p>
        </div>
      </div>
    </footer>
  )
}
