"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarHeader,
  SidebarFooter,
  SidebarSeparator,
} from "@/components/ui/sidebar"
import {
  Shield,
  LayoutDashboard,
  FileText,
  Search,
  ShieldCheck,
  Plug,
  ClipboardList,
  CreditCard,
  Settings,
  Users,
  HelpCircle,
  ChevronDown,
} from "lucide-react"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { product, hasPage } from "@/lib/product"

const allNavItems = [
  { label: "Visao Geral", href: "/console", icon: LayoutDashboard, page: "dashboard" },
  { label: "Execucoes", href: "/console/executions", icon: FileText, page: "executions" },
  { label: "Evidencias", href: "/console/evidence", icon: Search, page: "evidence" },
  { label: "Politicas", href: "/console/policies", icon: ShieldCheck, page: "policies" },
  { label: "Integracoes", href: "/console/integrations", icon: Plug, page: "integrations" },
  { label: "Auditorias", href: "/console/audits", icon: ClipboardList, page: "audits" },
  { label: "Faturamento", href: "/console/billing", icon: CreditCard, page: "billing" },
  { label: "Equipe & RBAC", href: "/console/team", icon: Users, page: "team" },
  { label: "Configuracoes", href: "/console/settings", icon: Settings, page: "settings" },
]

const navItems = allNavItems.filter((item) => !item.page || hasPage(item.page))

export function AppSidebar() {
  const pathname = usePathname()

  return (
    <Sidebar>
      <SidebarHeader className="p-4">
        <Link href="/console" className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-sidebar-primary">
            <Shield className="h-4 w-4 text-sidebar-primary-foreground" />
          </div>
          <span className="text-sm font-bold tracking-tight text-sidebar-primary">{product.name}</span>
        </Link>

        {/* Tenant Switcher */}
        <DropdownMenu>
          <DropdownMenuTrigger className="mt-3 flex w-full items-center justify-between rounded-md border border-sidebar-border bg-sidebar-accent px-3 py-2 text-xs text-sidebar-foreground hover:bg-sidebar-accent/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring">
            <span className="truncate">Empresa Corp</span>
            <ChevronDown className="h-3 w-3 shrink-0" />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" className="w-48">
            <DropdownMenuItem>Empresa Corp</DropdownMenuItem>
            <DropdownMenuItem>Projeto Sandbox</DropdownMenuItem>
            <DropdownMenuItem>+ Novo tenant</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>Plataforma</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.map((item) => (
                <SidebarMenuItem key={item.href}>
                  <SidebarMenuButton asChild isActive={pathname === item.href} tooltip={item.label}>
                    <Link href={item.href}>
                      <item.icon />
                      <span>{item.label}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      <SidebarSeparator />
      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton asChild tooltip="Ajuda">
              <Link href="/console/help">
                <HelpCircle />
                <span>Ajuda & Docs</span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  )
}
