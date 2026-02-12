"use client"

import { useState } from "react"
import Link from "next/link"
import { Button } from "@/components/ui/button"
import { Shield, Menu, X } from "lucide-react"

const navItems = [
  { label: "Produto", href: "/#produto" },
  { label: "Precos", href: "/pricing" },
  { label: "Verificador", href: "/verify/offline" },
  { label: "Docs", href: "/docs" },
  { label: "Changelog", href: "/changelog" },
]

export function MarketingHeader() {
  const [mobileOpen, setMobileOpen] = useState(false)

  return (
    <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="mx-auto flex h-16 max-w-7xl items-center justify-between px-4 lg:px-8">
        <Link href="/" className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-foreground">
            <Shield className="h-4 w-4 text-background" />
          </div>
          <span className="text-lg font-bold tracking-tight text-foreground">TDLN</span>
        </Link>

        <nav className="hidden items-center gap-6 md:flex" aria-label="Principal">
          {navItems.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              className="text-sm font-medium text-muted-foreground transition-colors hover:text-foreground"
            >
              {item.label}
            </Link>
          ))}
        </nav>

        <div className="hidden items-center gap-3 md:flex">
          <Button variant="ghost" size="sm" asChild>
            <Link href="/console">Entrar</Link>
          </Button>
          <Button size="sm" asChild>
            <Link href="/onboarding">Comecar gratis</Link>
          </Button>
        </div>

        <button
          className="flex h-10 w-10 items-center justify-center rounded-md md:hidden"
          onClick={() => setMobileOpen(!mobileOpen)}
          aria-label="Abrir menu"
          aria-expanded={mobileOpen}
        >
          {mobileOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
        </button>
      </div>

      {mobileOpen && (
        <div className="border-t bg-background px-4 pb-4 md:hidden">
          <nav className="flex flex-col gap-2 pt-4" aria-label="Menu mobile">
            {navItems.map((item) => (
              <Link
                key={item.href}
                href={item.href}
                className="rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                onClick={() => setMobileOpen(false)}
              >
                {item.label}
              </Link>
            ))}
            <div className="mt-2 flex flex-col gap-2">
              <Button variant="outline" size="sm" asChild>
                <Link href="/console">Entrar</Link>
              </Button>
              <Button size="sm" asChild>
                <Link href="/onboarding">Comecar gratis</Link>
              </Button>
            </div>
          </nav>
        </div>
      )}
    </header>
  )
}
