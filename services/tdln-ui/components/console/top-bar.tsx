"use client"

import { useState } from "react"
import Link from "next/link"
import { SidebarTrigger } from "@/components/ui/sidebar"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover"
import { Search, Plus, Bell } from "lucide-react"

const notifications = [
  { id: 1, text: "Execucao #1247 concluida com ACK", time: "2min atras", read: false },
  { id: 2, text: "Nova integracao GitHub configurada", time: "1h atras", read: false },
  { id: 3, text: "Limite de execucoes em 80%", time: "3h atras", read: true },
  { id: 4, text: "Politica Sanctions AML atualizada para v2.3.1", time: "1d atras", read: true },
]

export function TopBar() {
  const [searchQuery, setSearchQuery] = useState("")
  const unreadCount = notifications.filter((n) => !n.read).length

  const handleSearch = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && searchQuery.trim()) {
      window.location.href = `/console/executions?q=${encodeURIComponent(searchQuery.trim())}`
    }
  }

  return (
    <header className="flex h-14 shrink-0 items-center gap-3 border-b bg-background px-4">
      <SidebarTrigger />

      {/* Global search */}
      <div className="relative flex-1 max-w-md">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          placeholder="Buscar por CID, b3:trace, titulo..."
          className="h-9 pl-9 text-sm"
          aria-label="Busca global"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onKeyDown={handleSearch}
        />
      </div>

      <div className="flex items-center gap-2 ml-auto">
        <div className="hidden items-center gap-1.5 rounded-md bg-muted px-2.5 py-1 text-xs text-muted-foreground sm:flex">
          <span className="h-1.5 w-1.5 rounded-full bg-emerald-500" />
          <span className="font-medium">Empresa Corp</span>
        </div>
        <Button variant="outline" size="sm" className="h-9 bg-transparent" asChild>
          <Link href="/console/executions">
            <Plus className="mr-1 h-3.5 w-3.5" />
            Nova execucao
          </Link>
        </Button>

        {/* Notifications */}
        <Popover>
          <PopoverTrigger asChild>
            <Button variant="ghost" size="icon" className="relative h-9 w-9" aria-label="Notificacoes">
              <Bell className="h-4 w-4" />
              {unreadCount > 0 && (
                <span className="absolute right-1.5 top-1.5 flex h-3.5 w-3.5 items-center justify-center rounded-full bg-destructive text-[9px] font-bold text-destructive-foreground">
                  {unreadCount}
                </span>
              )}
            </Button>
          </PopoverTrigger>
          <PopoverContent align="end" className="w-80 p-0">
            <div className="border-b px-4 py-3">
              <h3 className="text-sm font-semibold text-foreground">Notificacoes</h3>
            </div>
            <div className="max-h-72 overflow-y-auto">
              {notifications.map((n) => (
                <div key={n.id} className={`border-b px-4 py-3 text-sm last:border-b-0 ${n.read ? "opacity-60" : ""}`}>
                  <div className="flex items-start gap-2">
                    {!n.read && <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-foreground" />}
                    <div className={!n.read ? "" : "pl-3.5"}>
                      <p className="text-foreground">{n.text}</p>
                      <p className="mt-0.5 text-xs text-muted-foreground">{n.time}</p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
            <div className="border-t px-4 py-2">
              <Button variant="ghost" size="sm" className="w-full text-xs" asChild>
                <Link href="/console/audits">Ver historico completo</Link>
              </Button>
            </div>
          </PopoverContent>
        </Popover>

        <div className="flex h-8 w-8 items-center justify-center rounded-full bg-foreground text-xs font-bold text-background">
          MS
        </div>
      </div>
    </header>
  )
}
