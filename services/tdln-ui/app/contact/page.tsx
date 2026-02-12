"use client"

import { useState } from "react"
import { MarketingHeader } from "@/components/marketing/header"
import { MarketingFooter } from "@/components/marketing/footer"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Card, CardContent } from "@/components/ui/card"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Mail, MessageCircle, MapPin } from "lucide-react"
import { toast } from "sonner"

const channels = [
  { icon: Mail, title: "Email", value: "suporte@tdln.io", description: "Resposta em ate 24h (Team) ou 4h (Enterprise)." },
  { icon: MessageCircle, title: "Chat ao vivo", value: "Disponivel 9h-18h BRT", description: "Para clientes Team e Enterprise." },
  { icon: MapPin, title: "Endereco", value: "Sao Paulo, SP - Brasil", description: "Faria Lima, 4100 - Itaim Bibi." },
]

export default function ContactPage() {
  const [sending, setSending] = useState(false)

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    setSending(true)
    setTimeout(() => {
      setSending(false)
      toast.success("Mensagem enviada com sucesso. Retornaremos em ate 24h.")
    }, 1000)
  }

  return (
    <div className="flex min-h-screen flex-col">
      <MarketingHeader />
      <main className="flex-1">
        <section className="border-b py-20">
          <div className="mx-auto max-w-3xl px-4 text-center lg:px-8">
            <h1 className="text-balance text-4xl font-bold tracking-tight text-foreground md:text-5xl">Contato</h1>
            <p className="mt-4 text-muted-foreground leading-relaxed">
              Duvidas sobre o produto, vendas Enterprise ou suporte tecnico. Estamos aqui.
            </p>
          </div>
        </section>

        <section className="py-16">
          <div className="mx-auto max-w-5xl px-4 lg:px-8">
            <div className="grid gap-12 lg:grid-cols-5">
              <div className="lg:col-span-3">
                <h2 className="text-lg font-bold text-foreground">Envie uma mensagem</h2>
                <form onSubmit={handleSubmit} className="mt-6 space-y-4">
                  <div className="grid gap-4 sm:grid-cols-2">
                    <div>
                      <Label htmlFor="name">Nome</Label>
                      <Input id="name" placeholder="Seu nome" className="mt-1.5" required />
                    </div>
                    <div>
                      <Label htmlFor="email">Email</Label>
                      <Input id="email" type="email" placeholder="nome@empresa.com" className="mt-1.5" required />
                    </div>
                  </div>
                  <div>
                    <Label htmlFor="subject">Assunto</Label>
                    <Select>
                      <SelectTrigger className="mt-1.5" id="subject">
                        <SelectValue placeholder="Selecione um assunto" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="sales">Vendas / Enterprise</SelectItem>
                        <SelectItem value="support">Suporte tecnico</SelectItem>
                        <SelectItem value="partnership">Parcerias</SelectItem>
                        <SelectItem value="general">Geral</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div>
                    <Label htmlFor="message">Mensagem</Label>
                    <textarea
                      id="message"
                      rows={5}
                      className="mt-1.5 w-full rounded-md border bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                      placeholder="Descreva como podemos ajudar..."
                      required
                    />
                  </div>
                  <Button type="submit" disabled={sending}>
                    {sending ? "Enviando..." : "Enviar mensagem"}
                  </Button>
                </form>
              </div>

              <div className="lg:col-span-2 space-y-4">
                <h2 className="text-lg font-bold text-foreground">Canais diretos</h2>
                {channels.map((ch) => (
                  <Card key={ch.title}>
                    <CardContent className="flex items-start gap-4 p-5">
                      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-muted">
                        <ch.icon className="h-5 w-5 text-foreground" />
                      </div>
                      <div>
                        <p className="text-sm font-semibold text-foreground">{ch.title}</p>
                        <p className="text-sm text-foreground">{ch.value}</p>
                        <p className="mt-0.5 text-xs text-muted-foreground">{ch.description}</p>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </div>
          </div>
        </section>
      </main>
      <MarketingFooter />
    </div>
  )
}
