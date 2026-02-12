export const metadata = { title: "Politica de Privacidade" }

export default function PrivacyPage() {
  return (
    <article className="prose prose-sm max-w-none text-foreground prose-headings:text-foreground prose-p:text-muted-foreground prose-strong:text-foreground prose-li:text-muted-foreground">
      <h1>Politica de Privacidade</h1>
      <p className="text-xs text-muted-foreground">Ultima atualizacao: 1 de fevereiro de 2026</p>

      <h2>1. Dados que coletamos</h2>
      <p>A TDLN coleta apenas os dados necessarios para fornecer o servico de verificacao criptografica:</p>
      <ul>
        <li><strong>Dados de conta:</strong> Nome, email, empresa e metodo de pagamento.</li>
        <li><strong>Dados de execucao:</strong> Metadados das decisoes processadas (titulo, origem, timestamps). O payload e processado em memoria e nao e armazenado.</li>
        <li><strong>Evidencias:</strong> Provas criptograficas geradas pelo protocolo SIRP. Evidencias sensiveis sao protegidas com AEAD encryption.</li>
        <li><strong>Logs de auditoria:</strong> Acoes realizadas no console (logins, alteracoes, exportacoes).</li>
      </ul>

      <h2>2. Como usamos seus dados</h2>
      <p>Seus dados sao usados exclusivamente para:</p>
      <ul>
        <li>Processar e armazenar execucoes e provas criptograficas.</li>
        <li>Gerar recibos verificaveis e bundles offline.</li>
        <li>Manter logs de auditoria imutaveis.</li>
        <li>Comunicacao sobre o servico (faturamento, alertas de uso, incidentes).</li>
      </ul>

      <h2>3. Retencao de dados</h2>
      <p>Dados sao retidos conforme o plano contratado (7, 30, 90 ou 365 dias). Apos o periodo, sao removidos permanentemente. Voce pode exportar todos os dados a qualquer momento via Console.</p>

      <h2>4. Compartilhamento</h2>
      <p>Nao vendemos, alugamos ou compartilhamos seus dados com terceiros. Dados so sao compartilhados quando exigido por lei ou ordem judicial.</p>

      <h2>5. Seguranca</h2>
      <p>Utilizamos criptografia em transito (TLS 1.3) e em repouso (AES-256). Evidencias sensiveis sao protegidas com AEAD. Infraestrutura auditada SOC 2 e ISO 27001.</p>

      <h2>6. Seus direitos</h2>
      <p>Voce tem direito a acessar, corrigir, exportar e excluir seus dados a qualquer momento. Para exercer seus direitos, acesse Configuracoes no Console ou entre em contato com privacidade@tdln.io.</p>

      <h2>7. Contato</h2>
      <p>Para questoes sobre privacidade: <strong>privacidade@tdln.io</strong></p>
    </article>
  )
}
