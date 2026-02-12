"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { mockTeam, type TeamMember } from "@/lib/mock-data"
import {
  Users,
  UserPlus,
  Shield,
  ShieldCheck,
  Key,
  Lock,
  Mail,
  MoreHorizontal,
  Check,
  X,
  AlertTriangle,
} from "lucide-react"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu"
import Link from "next/link"
import { toast } from "sonner"

const roleDescriptions: Record<string, string> = {
  Owner: "Acesso total, incluindo faturamento e exclusao de tenant.",
  Admin: "Gerencia equipe, politicas e integracoes. Sem acesso a faturamento.",
  Operator: "Cria e gerencia execucoes. Sem acesso a politicas ou equipe.",
  Auditor: "Acesso somente leitura a execucoes, evidencias e auditorias.",
}

const roleBadgeVariants: Record<string, string> = {
  Owner: "bg-foreground text-background",
  Admin: "bg-muted text-foreground",
  Operator: "bg-muted text-foreground",
  Auditor: "bg-muted text-muted-foreground",
}

export default function TeamPage() {
  const [members] = useState<TeamMember[]>(mockTeam)
  const [inviteEmail, setInviteEmail] = useState("")
  const [inviteRole, setInviteRole] = useState("Operator")
  const [inviteOpen, setInviteOpen] = useState(false)
  const [enforceMFA, setEnforceMFA] = useState(false)

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-foreground">Equipe & RBAC</h1>
          <p className="mt-1 text-sm text-muted-foreground">Gerencie membros, papeis e controles de acesso.</p>
        </div>
        <Dialog open={inviteOpen} onOpenChange={setInviteOpen}>
          <DialogTrigger asChild>
            <Button size="sm">
              <UserPlus className="mr-1.5 h-4 w-4" />
              Convidar membro
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Convidar membro</DialogTitle>
              <DialogDescription>
                Envie um convite por email. O membro recebera acesso conforme o papel selecionado.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div>
                <Label htmlFor="invite-email">Email</Label>
                <Input
                  id="invite-email"
                  type="email"
                  placeholder="nome@empresa.com"
                  value={inviteEmail}
                  onChange={(e) => setInviteEmail(e.target.value)}
                  className="mt-1.5"
                />
              </div>
              <div>
                <Label htmlFor="invite-role">Papel</Label>
                <Select value={inviteRole} onValueChange={setInviteRole}>
                  <SelectTrigger className="mt-1.5" id="invite-role">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="Admin">Admin</SelectItem>
                    <SelectItem value="Operator">Operator</SelectItem>
                    <SelectItem value="Auditor">Auditor</SelectItem>
                  </SelectContent>
                </Select>
                <p className="mt-1 text-xs text-muted-foreground">
                  {roleDescriptions[inviteRole]}
                </p>
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setInviteOpen(false)}>Cancelar</Button>
              <Button onClick={() => setInviteOpen(false)} disabled={!inviteEmail}>
                <Mail className="mr-1.5 h-4 w-4" />
                Enviar convite
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>

      {/* Role overview cards */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {(["Owner", "Admin", "Operator", "Auditor"] as const).map((role) => {
          const count = members.filter((m) => m.role === role).length
          return (
            <Card key={role}>
              <CardContent className="flex items-center gap-4 p-4">
                <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-muted">
                  {role === "Owner" ? <Shield className="h-5 w-5 text-foreground" /> :
                   role === "Admin" ? <ShieldCheck className="h-5 w-5 text-foreground" /> :
                   role === "Operator" ? <Users className="h-5 w-5 text-foreground" /> :
                   <Key className="h-5 w-5 text-muted-foreground" />}
                </div>
                <div>
                  <p className="text-sm font-semibold text-foreground">{role}</p>
                  <p className="text-xs text-muted-foreground">{count} membro{count !== 1 ? "s" : ""}</p>
                </div>
              </CardContent>
            </Card>
          )
        })}
      </div>

      {/* Members table */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium text-foreground">Membros da equipe</CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Membro</TableHead>
                <TableHead>Papel</TableHead>
                <TableHead>MFA</TableHead>
                <TableHead>Ultimo acesso</TableHead>
                <TableHead className="w-[60px]" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {members.map((member) => (
                <TableRow key={member.id}>
                  <TableCell>
                    <div className="flex items-center gap-3">
                      <div className="flex h-8 w-8 items-center justify-center rounded-full bg-muted text-xs font-bold text-foreground">
                        {member.name.split(" ").map((n) => n[0]).join("")}
                      </div>
                      <div>
                        <p className="text-sm font-medium text-foreground">{member.name}</p>
                        <p className="text-xs text-muted-foreground">{member.email}</p>
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <span className={`inline-flex items-center rounded-md px-2 py-0.5 text-xs font-semibold ${roleBadgeVariants[member.role]}`}>
                      {member.role}
                    </span>
                  </TableCell>
                  <TableCell>
                    {member.mfa ? (
                      <span className="inline-flex items-center gap-1 text-xs text-emerald-600">
                        <Check className="h-3.5 w-3.5" />
                        Ativo
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                        <X className="h-3.5 w-3.5" />
                        Inativo
                      </span>
                    )}
                  </TableCell>
                  <TableCell>
                    <span className="text-xs text-muted-foreground">
                      {new Date(member.lastActive).toLocaleString("pt-BR")}
                    </span>
                  </TableCell>
                  <TableCell>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button variant="ghost" size="icon" className="h-8 w-8">
                          <MoreHorizontal className="h-4 w-4" />
                          <span className="sr-only">Acoes para {member.name}</span>
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => toast.success(`Papel de ${member.name} sera atualizado. Selecione o novo papel na proxima tela.`)}>Alterar papel</DropdownMenuItem>
                        <DropdownMenuItem onClick={() => toast.success(`MFA resetado para ${member.name}. Sera solicitado novo setup no proximo login.`)}>Resetar MFA</DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem className="text-destructive" onClick={() => toast.success(`${member.name} removido da equipe.`)}>Remover membro</DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {/* Security settings */}
      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="text-sm font-medium text-foreground">Seguranca de acesso</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between rounded-lg border p-4">
              <div className="flex items-center gap-3">
                <Lock className="h-5 w-5 text-foreground" />
                <div>
                  <p className="text-sm font-medium text-foreground">Exigir MFA para todos</p>
                  <p className="text-xs text-muted-foreground">Membros sem MFA serao forcados a configurar no proximo login.</p>
                </div>
              </div>
              <Switch
                checked={enforceMFA}
                onCheckedChange={setEnforceMFA}
                aria-label="Exigir MFA para todos os membros"
              />
            </div>
            {!members.every((m) => m.mfa) && enforceMFA && (
              <div className="flex items-start gap-3 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3">
                <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-600" />
                <p className="text-xs text-muted-foreground">
                  {members.filter((m) => !m.mfa).length} membro(s) ainda nao configurou MFA e sera(ao) notificado(s).
                </p>
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm font-medium text-foreground">SSO & SAML</CardTitle>
              <Badge variant="secondary" className="text-xs">Enterprise</Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="text-sm text-muted-foreground">
              Configure Single Sign-On com SAML 2.0 para login centralizado. Inclui provisionamento automatico via SCIM.
            </p>
            <div className="space-y-2">
              <div className="flex items-center justify-between rounded-md border bg-muted/50 p-3">
                <span className="text-sm text-foreground">SAML SSO</span>
                <Badge variant="outline" className="text-xs">Requer Enterprise</Badge>
              </div>
              <div className="flex items-center justify-between rounded-md border bg-muted/50 p-3">
                <span className="text-sm text-foreground">SCIM Provisioning</span>
                <Badge variant="outline" className="text-xs">Requer Enterprise</Badge>
              </div>
            </div>
            <Button variant="outline" size="sm" className="w-full bg-transparent" asChild>
              <Link href="/contact">
                Falar com vendas para Enterprise
              </Link>
            </Button>
          </CardContent>
        </Card>
      </div>

      {/* Role permissions reference */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium text-foreground">Permissoes por papel</CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Recurso</TableHead>
                <TableHead className="text-center">Owner</TableHead>
                <TableHead className="text-center">Admin</TableHead>
                <TableHead className="text-center">Operator</TableHead>
                <TableHead className="text-center">Auditor</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {[
                { resource: "Execucoes", owner: true, admin: true, operator: true, auditor: true },
                { resource: "Criar execucoes", owner: true, admin: true, operator: true, auditor: false },
                { resource: "Evidencias", owner: true, admin: true, operator: true, auditor: true },
                { resource: "Politicas", owner: true, admin: true, operator: false, auditor: false },
                { resource: "Integracoes", owner: true, admin: true, operator: false, auditor: false },
                { resource: "Auditorias", owner: true, admin: true, operator: false, auditor: true },
                { resource: "Equipe", owner: true, admin: true, operator: false, auditor: false },
                { resource: "Faturamento", owner: true, admin: false, operator: false, auditor: false },
                { resource: "Configuracoes", owner: true, admin: true, operator: false, auditor: false },
              ].map((row) => (
                <TableRow key={row.resource}>
                  <TableCell className="text-sm text-foreground">{row.resource}</TableCell>
                  {[row.owner, row.admin, row.operator, row.auditor].map((allowed, i) => (
                    <TableCell key={i} className="text-center">
                      {allowed ? (
                        <Check className="mx-auto h-4 w-4 text-emerald-500" />
                      ) : (
                        <X className="mx-auto h-4 w-4 text-muted-foreground/40" />
                      )}
                    </TableCell>
                  ))}
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  )
}
