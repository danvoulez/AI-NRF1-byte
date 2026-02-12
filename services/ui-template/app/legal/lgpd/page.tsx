export const metadata = { title: "Conformidade LGPD" }

export default function LGPDPage() {
  return (
    <article className="prose prose-sm max-w-none text-foreground prose-headings:text-foreground prose-p:text-muted-foreground prose-strong:text-foreground prose-li:text-muted-foreground">
      <h1>Conformidade LGPD</h1>
      <p className="text-xs text-muted-foreground">Ultima atualizacao: 1 de fevereiro de 2026</p>

      <h2>Compromisso com a Lei Geral de Protecao de Dados</h2>
      <p>A TDLN foi projetada desde o inicio para conformidade com a LGPD (Lei 13.709/2018). Nossa arquitetura garante que seus dados pessoais sejam tratados com seguranca, transparencia e sob seu controle total.</p>

      <h2>Base legal para tratamento</h2>
      <ul>
        <li><strong>Execucao de contrato (Art. 7, V):</strong> Dados de conta e faturamento necessarios para prestacao do servico.</li>
        <li><strong>Obrigacao legal (Art. 7, II):</strong> Logs de auditoria mantidos para conformidade regulatoria.</li>
        <li><strong>Consentimento (Art. 7, I):</strong> Comunicacoes de marketing enviadas apenas com consentimento explicito.</li>
      </ul>

      <h2>Direitos do titular (Art. 18)</h2>
      <p>Voce pode exercer todos os direitos previstos na LGPD diretamente pelo Console ou por email:</p>
      <ul>
        <li><strong>Acesso:</strong> Exporte todos os seus dados via Configuracoes &gt; Exportar dados.</li>
        <li><strong>Correcao:</strong> Atualize seus dados de conta a qualquer momento.</li>
        <li><strong>Eliminacao:</strong> Solicite exclusao total via Console ou por email.</li>
        <li><strong>Portabilidade:</strong> Exporte em JSON/CSV/ZIP para transferencia a outro controlador.</li>
        <li><strong>Revogacao de consentimento:</strong> Gerencie preferencias de comunicacao no Console.</li>
      </ul>

      <h2>Medidas tecnicas</h2>
      <ul>
        <li><strong>Residencia de dados:</strong> Escolha a regiao de armazenamento (BR, US, EU). Dados nao sao transferidos entre regioes sem sua autorizacao.</li>
        <li><strong>Criptografia:</strong> TLS 1.3 em transito, AES-256 em repouso, AEAD para evidencias sensiveis.</li>
        <li><strong>Minimizacao:</strong> Payloads sao processados em memoria e nao armazenados. Apenas metadados e provas criptograficas sao retidos.</li>
        <li><strong>Retencao:</strong> Dados removidos automaticamente apos o periodo contratado.</li>
      </ul>

      <h2>Encarregado de dados (DPO)</h2>
      <p>Para exercer seus direitos ou esclarecer duvidas sobre tratamento de dados:</p>
      <p><strong>Email:</strong> dpo@tdln.io</p>
      <p><strong>Prazo de resposta:</strong> Ate 15 dias uteis, conforme Art. 18 da LGPD.</p>
    </article>
  )
}
