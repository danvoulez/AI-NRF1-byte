import Link from "next/link"
import { MarketingHeader } from "@/components/marketing/header"
import { MarketingFooter } from "@/components/marketing/footer"

const legalNav = [
  { label: "Privacidade", href: "/legal/privacy" },
  { label: "Termos de Uso", href: "/legal/terms" },
  { label: "LGPD", href: "/legal/lgpd" },
  { label: "Seguranca", href: "/legal/security" },
]

export default function LegalLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex min-h-screen flex-col">
      <MarketingHeader />
      <main className="flex-1">
        <div className="border-b py-12">
          <div className="mx-auto max-w-4xl px-4 lg:px-8">
            <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground">Legal</p>
            <nav className="mt-4 flex flex-wrap gap-2">
              {legalNav.map((item) => (
                <Link
                  key={item.href}
                  href={item.href}
                  className="rounded-md border bg-card px-3 py-1.5 text-sm font-medium text-foreground transition-colors hover:bg-muted"
                >
                  {item.label}
                </Link>
              ))}
            </nav>
          </div>
        </div>
        <div className="mx-auto max-w-4xl px-4 py-16 lg:px-8">
          {children}
        </div>
      </main>
      <MarketingFooter />
    </div>
  )
}
