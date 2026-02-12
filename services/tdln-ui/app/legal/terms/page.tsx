export const metadata = { title: "Termos de Uso" }

export default function TermsPage() {
  return (
    <article className="prose prose-sm max-w-none text-foreground prose-headings:text-foreground prose-p:text-muted-foreground prose-strong:text-foreground prose-li:text-muted-foreground">
      <h1>Termos de Uso</h1>
      <p className="text-xs text-muted-foreground">Ultima atualizacao: 1 de fevereiro de 2026</p>

      <h2>1. Aceitacao</h2>
      <p>Ao acessar ou utilizar os servicos da TDLN, voce concorda com estes Termos de Uso. Se nao concordar, nao utilize o servico.</p>

      <h2>2. Descricao do servico</h2>
      <p>TDLN fornece infraestrutura de verificacao criptografica para decisoes de software, incluindo: protocolo SIRP, geracao de recibos verificaveis, bundles offline, gerenciamento de politicas e integracao via SDKs e APIs.</p>

      <h2>3. Contas</h2>
      <p>Voce e responsavel por manter a confidencialidade das suas credenciais e chaves de API. Atividades realizadas sob sua conta sao de sua responsabilidade. Recomendamos fortemente a ativacao de MFA.</p>

      <h2>4. Uso aceitavel</h2>
      <p>Voce concorda em nao utilizar o TDLN para:</p>
      <ul>
        <li>Atividades ilegais ou fraudulentas.</li>
        <li>Interferir com a seguranca ou disponibilidade da plataforma.</li>
        <li>Engenharia reversa do protocolo SIRP ou SDKs.</li>
        <li>Exceder os limites do seu plano de forma intencional e repetida.</li>
      </ul>

      <h2>5. Propriedade intelectual</h2>
      <p>Os recibos e provas criptograficas gerados pertencem a voce. O protocolo SIRP, marca TDLN, SDKs e infraestrutura sao propriedade da TDLN.</p>

      <h2>6. Disponibilidade</h2>
      <p>Nos comprometemos com 99.9% de uptime (SLA). Manutencoes programadas serao comunicadas com 48h de antecedencia. O SLA detalhado esta disponivel para clientes Enterprise.</p>

      <h2>7. Limitacao de responsabilidade</h2>
      <p>O TDLN nao se responsabiliza por decisoes tomadas com base nos recibos gerados. O servico fornece provas criptograficas — a interpretacao e de responsabilidade do usuario.</p>

      <h2>8. Cancelamento</h2>
      <p>Voce pode cancelar sua conta a qualquer momento. Dados serao retidos pelo periodo contratado e depois removidos. Exporte seus dados antes do cancelamento.</p>

      <h2>9. Alteracoes</h2>
      <p>Podemos atualizar estes termos com aviso previo de 30 dias. O uso continuado apos alteracoes constitui aceitacao.</p>

      <h2>10. Foro</h2>
      <p>Estes termos sao regidos pelas leis do Brasil. Fica eleito o foro da Comarca de Sao Paulo/SP.</p>
    </article>
  )
}
