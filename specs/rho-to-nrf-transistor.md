# ρ→NRF como Transistor Determinístico de Decisão

Uma formulação formal, auditável e LLM-first para dados, hashes e juízos

**Resumo.** Definimos um pipeline determinístico em três camadas:
(i) ρ (forma normal semântica) → (ii) NRF-1.1 (codificação binária canônica) → (iii) D (decisor puro com estados {ALLOW, DENY, REQUIRE, GHOST}).
Provamos propriedades de canonicidade (um valor lógico ⇒ um único byte stream), idempotência (ρ é estável), confluência (ordem de reescrita irrelevante), e composicionalidade (série/quorum/latch) sob Write-Before-Execute e HAL. Fornecemos gramática, regras normativas, erros, vetores KAT, e esboços de provas.

—

## 1. Modelo Formal

### 1.1 Universos e funções
- **Valores lógicos (pré-ρ):**  \mathcal{V} — JSON/UBL-JSON rico (strings, inteiros, decimais, arrays, mapas, somas/opções, conjuntos).
- **Forma normal (pós-ρ):**      \mathcal{N} ⊆ \mathcal{V} — subconjunto estrito com escolhas eliminadas.
- **Bytes canônicos:**           \mathcal{B} = \{0,1\}^*.
- **Decisor:**                   D : \mathcal{B} \to \{\textsf{ALLOW},\textsf{DENY},\textsf{REQUIRE},\textsf{GHOST}\}.

Compomos: \(E \circ \rho : \mathcal{V} \to \mathcal{N} \to \mathcal{B}\) onde \(\rho\) é a normalização semântica e \(E\) é a codificação NRF-1.1.

Definimos ainda:
\(\textsf{CID}(v) = \textsf{BLAKE3}(E(\rho(v)))\) e \(\textsf{SIG}(v,k) = \textsf{Ed25519}(\textsf{SHA256}(E(\rho(v)) \parallel \textsf{domain}))\).

### 1.2 Objetivos normativos (ordem de prioridade)
1. **Canonicidade** — um valor lógico ⇒ um byte stream; sem alternativas.
2. **Simplicidade** — regras pequenas, completas e mecanizáveis.
3. **LLM-first** — geração/validação correta a partir de um único documento.

—

## 2. Forma Normal ρ (Normativo)

### 2.1 Strings
- **MUST:** UTF-8 válida; NFC (Unicode Normalization Form C); sem BOM (U+FEFF) em qualquer posição.
- **Erro:** InvalidUTF8, NotNFC, BOMPresent.

### 2.2 Tempo
- Representação canônica: string RFC-3339 UTC ‘Z’.
- Regra: converter qualquer offset para UTC; remover fração quando zero; manter fração mínima necessária.
- Ex.: `2026-02-08T12:03:12Z`, `2026-02-08T12:03:12.123Z`.
- **Erro:** InvalidTimestamp (formato inválido/offset não normalizado).

### 2.3 Decimais (sem float)
Representados como string canônica (não usar IEEE-754):

```

^-?(0|\[1-9\]\[0-9\]\*)(.\[0-9\]+)?$

```

Sem expoente; sem +; sem zeros à esquerda; sem zeros fracionários supérfluos.

Ex.: **OK:** `"0"`, `"-12"`, `"3.1415"`. **INVÁLIDO:** `"01"`, `"1.0"`, `"1e3"`.
**Erro:** InvalidDecimal.

### 2.4 Soma/Variante (enums/union)
Forma ρ-canônica: `{"$case":"<ASCII/NFC>","$val":<ρ(v)>?}`  
— Nullária: sem `$val`. Com payload: `$val` presente.  
**Erro:** InvalidCase, MissingCase, UnexpectedVal.

### 2.5 Option
Açúcar de soma:  
`None` → `{"$case":"None"}`;  
`Some(x)` → `{"$case":"Some","$val":<ρ(x)>}`.

### 2.6 Set (conjunto)
Representação: Array com regra ρ:  
1. converta cada item para bytes NRF (via \(E\circ\rho\)),  
2. ordene por ordem lexicográfica de bytes,  
3. deduplique vizinhos idênticos.  
**Propriedades:** idempotente; independente da ordem de entrada.

### 2.7 Map/Objeto
- Chaves string NFC, sem BOM.
- Ausência ≠ null: não materialize campos ausentes com `null`.
- Ordenação por bytes é responsabilidade de NRF (Seção 3).

### 2.8 Regex (opcional)
Produto canônico: `{"pattern":"<NFC>","flags":"gim"}` — flags ordenadas alfabeticamente, sem duplicatas.  
**Erro:** InvalidRegex.

Invariante ρ: após ρ, não restam escolhas ao encoder. ρ é aplicada recursivamente.

—

## 3. Codificação NRF-1.1 (trechos essenciais)

### 3.1 Tipos e tags

| Tag | Tipo   | Payload                                 |
|-----|--------|------------------------------------------|
| 00  | Null   | —                                        |
| 01  | False  | —                                        |
| 02  | True   | —                                        |
| 03  | Int64  | 8 bytes big-endian (two’s complement)    |
| 04  | String | varint32(len) + bytes UTF-8 (NFC, sem BOM) |
| 05  | Bytes  | varint32(len) + bytes                    |
| 06  | Array  | varint32(count) + N valores (tag+payload)|
| 07  | Map    | varint32(count) + N pares (key:String, value) |

- Map: chaves String; ordenadas por bytes UTF-8; sem duplicatas.  
- varint32: LEB128 mínimo, 1–5 bytes (rejeitar não-minimal).  
- Sem floats. Decimais são strings ρ-canônicas.

### 3.2 Canonicidade por construção
- Um único tipo para inteiros (Int64 fixo).
- Strings validadas como ρ.
- varint32 minimal ⇒ sem ambiguidade de comprimento/contagem.
- Map ordenado e sem repetição de chaves.

### 3.3 Erros de decodificação NRF
`InvalidMagic, InvalidTypeTag, NonMinimalVarint, UnexpectedEOF, InvalidUTF8, NotNFC, BOMPresent, NonStringKey, UnsortedKeys, DuplicateKey, TrailingData.`

—

## 4. Recibos e URLs Ricas

Um **Recibo** é um Map NRF com chaves bem-conhecidas (mínimo):
- `"v"` (String) — ex.: `"receipt-v1"`.
- `"t"` (Int64) — nanos desde Unix epoch do signatário.
- `"body"` (qualquer NRF) — objeto sob juízo.
- `"nonce"` (Bytes, 16B) — aleatório para unicidade.
- `"sig"` (Bytes, 64B) — assinatura Ed25519 sobre `SHA256(nrf_bytes||domain)`.
- `"urls"` (Array[String]) — URLs ricas com cid/did/runtime/app/tenant.

**Regra do Hash:** hashes/assinaturas **sempre** sobre os bytes NRF de `recibo_sem_sig`.  
Tempo forte (se necessário) é camada superior (ex.: âncora externa).

—

## 5. Decisor Puro D e Composição

### 5.1 Decisor
- \(D : \mathcal{B}\to\{\textsf{ALLOW},\textsf{DENY},\textsf{REQUIRE},\textsf{GHOST}\}\).
- **MUST** ser determinístico, sem I/O, relógio, aleatoriedade.
- Parâmetros (thresholds, versão de regra) vêm no próprio contexto NRF.

### 5.2 Álgebra de composição
- **Série (AND):** \(\bigwedge_i D_i(b)\) — `DENY` domina; `REQUIRE/GHOST` bloqueiam promoção.
- **Paralelo/Quorum (OR/MAJORITY):** `ALLOW` se ≥k de n decidem `ALLOW`.
- **Latch (pendência):** estado `GHOST` persiste até `C(b)` ⇒ `PROMOTE(GHOST→ALLOW)`, com recibo de promoção idempotente.
- **WBE:** escreva sempre o recibo antes de executar efeitos.
- **HAL:** executor nega efeitos fora do perfil (CPU/mem/IO/tempo).

—

## 6. Propriedades e Esboços de Prova
(Como no texto da conversa: Idempotência de ρ; Confluência; Canonicidade de \(E∘ρ\); Hash-Identidade; Determinismo do Decisor; Monotonicidade de Latch.)

—

## 7. Complexidade
- ρ: O(n) no tamanho do valor (sort/dedup de sets é dominante).  
- E (NRF): O(n).  
- Hash/Assinatura: BLAKE3/Ed25519.

—

## 8. Segurança (Resumo)
Sem floats; Strings NFC sem BOM; varint32 minimal; Decisor puro; WBE+HAL; domínios pequenos para sal/pepper; timestamp afirma signatário.

—

## 9. Interoperabilidade
Importadores (JSON/CBOR/MsgPack) apenas via ρ estrita; dados não 1:1 mapeáveis ⇒ **REJECT**. Assinaturas/CIDs sempre sobre bytes NRF.

—

## 10. Conformidade (KATs & Erros)
Vetores resumidos (nullária, set canônico, decimal, tempo, map ordenado) e tabela de erros.

—

## 11. Especificação de Fio (ABNF-like)
Como na conversa: `stream = magic value`, `magic = "nrf1"`, etc.

—

## 12. Pseudocódigo de Referência (ρ)
Inclui regra especial de `Set` ordenado por bytes NRF dos itens.

—

## 13. Operacionalização (CLI/CI)
Comandos `nrf1 json encode/hash`, `receipt sign`, `decision run/compose`; KATs obrigatórios e fuzz de decoder.

—

## 14. Trabalhos Relacionados
CBOR determinístico, X.690 DER/CER, in-toto/SLSA/SPDX.

—

## 15. Conclusão
Mesma semântica ⇒ mesmos bytes ⇒ mesmo hash ⇒ mesma decisão — sempre.
"""

with open(spec_path, "w", encoding="utf-8") as f:
    f.write(spec_text.strip() + "\n")

