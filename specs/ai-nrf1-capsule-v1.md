# AI-NRF1 & UBL Capsule v1 — Especificação Oficial
**Versão:** 1.0.0 • **Data:** <TS>  
**Identificadores canônicos:** `ai-nrf1` (bytes), `ai-json-nrf1` (view JSON), `ubl-byte`/`ubl-json` (marca)

---

## 0. Propósito
Determinizar representação, hashing e prova de decisões. **Canon = bytes AI-NRF1**. JSON é apenas uma view **AI-JSON-NRF1**. O **UBL Capsule v1** encapsula mensagem + provas (SIRP) num único artefato verificável.

> Slogan (produto): **um valor ⇒ um byte ⇒ um hash ⇒ uma decisão**.  
> Nota normativa: o dígito **“1”** em `ai-nrf1` é **major version** do perfil canônico.

---

## 1. Convenções, MIME e Schemas
- **MIME (bytes):** `application/ai-nrf1`
- **MIME (view):** `application/ai-json-nrf1+json`
- **Schema IDs:**
  - `https://spec.vv.ai/ai-nrf/1/ai-json-nrf1.schema.json`
  - `https://spec.vv.ai/ubl/1/ubl-capsule.v1.schema.json`
- **CLI canônico:** `ai nrf1 encode|decode|hash` • `ai ubl cap sign|verify|view-json`
- **Compat:** manter aliases antigos por 1 release com aviso “deprecated”.

---

## 2. AI-NRF1 (perfil binário canônico)
### 2.1 Tipos e tags
```

Tag Tipo Payload  
00 Null —  
01 False —  
02 True —  
03 Int64 8 bytes big-endian (two’s complement)  
04 String varint32(len) + bytes UTF-8 (NFC, sem BOM)  
05 Bytes varint32(len) + bytes  
06 Array varint32(count) + N valores (tag+payload)  
07 Map varint32(count) + N pares (key:String, value)

```
- **Map:** chaves `String` ordenadas por bytes UTF-8; sem duplicatas.
- **varint32:** LEB128 **mínimo** (rejeitar codificações não-mínimas).
- **Inteiros:** apenas **Int64** (sem variações de largura).
- **Sem float:** decimais são `String` canônicas (regex `^-?(0|[1-9][0-9]*)(\\.[0-9]+)?$`).

### 2.2 Regras ρ (normalização semântica)
- **Strings:** UTF-8 válida, **NFC**, sem **BOM** em qualquer posição.
- **Timestamps:** RFC-3339 UTC ’Z’; remover fração zero; manter fração mínima.
- **Decimal:** sem expoente; sem zeros à esquerda; sem “.0” supérfluo.
- **Set:** ordenar por **bytes AI-NRF1** do item (E∘ρ), deduplicar vizinhos.
- **Map:** ausência ≠ `null`; ordenação é do encoder (não de ρ).

### 2.3 Propriedades
- **Canonicidade:** um valor lógico ⇒ um único byte stream.
- **Idempotência:** `ρ(ρ(v)) = ρ(v)`.
- **Confluência:** ordem de reescrita irrelevante.
- **Complexidade:** encoder/decoder O(n); BLAKE3 linear.

### 2.4 Erros normativos (amostra)
`InvalidUTF8, NotNFC, BOMPresent, InvalidDecimal, InvalidTimestamp, NonMinimalVarint, InvalidTypeTag, NonStringKey, UnsortedKeys, DuplicateKey, UnexpectedEOF, TrailingData, InvalidMagic`.

---

## 3. UBL Capsule v1 (SIRP + AI-JSON-NRF1)
### 3.1 Estrutura (Map AI-NRF1)
```

"v" : String // "ubl-capsule/1.0"  
"id" : Bytes(32) // blake3(...)  
"hdr" : Map { // roteamento estável  
"src" : String ASCII // DID/KID normalizado (ASCII-only)  
"dst" : String ASCII  
"nonce" : Bytes(16)  
"exp" : Int64 // epoch-nanos  
"chan" : String ASCII? // opcional  
"ts" : Int64? // opcional (emissor)  
}  
"env" : Map { // view semântica (ai-json-nrf1)  
"v" : String // "ai-json-nrf1/1.0"  
"t" : String // "record" | "bundle" | "trace" | "query"  
"agent" : Map { "id": String, "name": String? }  
"intent" : Map { "kind":"ATTEST"|"EVAL"|"BUNDLE"|"TRACE"|"QUERY",  
"name": String, "args": Map? }  
"ctx" : Map  
"decision": Map { "verdict":"ACK"|"NACK"|"ASK", "reason":String?, "metrics":Map? }  
"evidence": Map { "cids":\[Bytes(32)\]?, "urls":\[String\]? }  
"meta" : Map { "app":String, "tenant":String, "user":String, "session":String? }?  
"links" : Map { "prev":Bytes(32)?, "trace":Bytes(32)? }?  
}  
"seal" : Map {  
"alg" : "Ed25519"|"Dilithium3",  
"kid" : String ASCII, // DID#key  
"domain" : "ubl-capsule/1.0",  
"scope" : "capsule",  
"aud" : String ASCII?, // opcional: bind ao dst  
"sig" : Bytes(64|..)  
}  
"receipts" : Array<Map>? // hop receipts encadeadas

```

### 3.2 Cálculo de `id` (canon = bytes)
`id = blake3( nrf.encode( capsule \\ {{id}, {seal.sig}, {receipts[*].sig}} ) )`

### 3.3 Assinaturas
- **Seal** assina **apenas** o “core”:
  `sig = sign( blake3( nrf.encode({domain,id,hdr,env}) ) )`
- **Receipt (hop)**: `{ of:Bytes(32)=id, prev:Bytes(32), kind:String, node:String ASCII, ts:Int64 }`  
  Assinatura cobre `{domain:"ubl-receipt/1.0", of, prev, kind, node, ts}`.

### 3.4 Regras de canonicidade
- Ordenação lexicográfica por bytes UTF-8 de **todas** as chaves.
- Strings NFC; DIDs/KIDs **ASCII-only**.
- `ASK ⇒ env.links.prev` **obrigatório** (ghost pendente).
- `ACK/NACK ⇒ env.evidence` **presente** (pode estar vazia).

### 3.5 Erros padronizados
`Err.Canon.NotASCII, Err.Canon.NotNFC, Err.Canon.FloatForbidden, Err.Capsule.IDMismatch, Err.Seal.BadSignature, Err.Seal.ScopeDomain, Err.Hop.BadChain, Err.Hop.BadSignature, Err.Hdr.Expired`.

---

## 4. AI-JSON-NRF1 (view)
- **Entrada/saída humana/LLM.** Encoder realiza ρ, depois AI-NRF1.
- **Bytes em JSON:** `b3:<hex>` (32B) e `b64:<...>` quando aplicável.
- **Aceita** escapes (`\\u0061`) na view; canônico guarda bytes (0x61).

---

## 5. KATs (resumo)
- **Null:** `6E726631 00`
- **Int64 = −1:** `6E726631 03 FFFFFFFFFFFFFFFF`
- **String “hello”:** `6E726631 04 05 68656C6C6F`
- **Array [true, 42]:** `6E726631 06 02 02 03 000000000000002A`
- **Map {“a”:1,“b”:true}:**
```

6E726631 07 02  
04 01 61 03 0000000000000001  
04 01 62 02

```
- **Sum nullária:** vide KAT de ρ → soma canônica.

---

## 6. ABNF (fio)
```

stream = magic value  
magic = %x6E.72.66.31 ; "nrf1"

value = null / false / true / int64 / string / bytes / array / map  
null = %x00  
false = %x01  
true = %x02  
int64 = %x03 8OCTET  
string = %x04 varint32 \*OCTET ; UTF-8 NFC, sem BOM  
bytes = %x05 varint32 \*OCTET  
array = %x06 varint32 \*value  
map = %x07 varint32 \*(string value) ; chaves sorted, únicas

varint32 = 1\*5 lebbyte ; LEB128 mínimo  
lebbyte = OCTET ; bit7=cont, bits\[6:0\]=payload

```

---

## 7. Interop & Conformidade
- Importadores JSON/CBOR/MsgPack **só** via ρ estrita.
- Hashes/assinaturas **sempre** sobre bytes AI-NRF1 (nunca bytes de import).
- Fuzz do decoder + corpus de crashers obrigatório no CI.
- KATs e vetores `ASK/ACK/NACK` (capsule) inclusos no bundle de testes.

---

## 8. Segurança (resumo)
- Sem floats; Strings NFC; varint mínimo; Map ordenado e único.
- Decisor puro; WBE + HAL; permits (executor só move com permit assinado).
- AEAD opcional no transporte; offline bundle verificável.

---

## 9. Marcas
Use **ai-nrf1 / ai-json-nrf1** quando a norma importa.  
Use **ubl-byte / ubl-json** quando a marca importa. Ambas mapeiam para **o mesmo canon**.
""").strip() + "\n"

spec_md = spec_template.replace("<TS>", ts)

ai_json_schema = {
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://spec.vv.ai/ai-nrf/1/ai-json-nrf1.schema.json",
  "title": "AI-JSON-NRF1 View",
  "type": "object",
  "additionalProperties": True,
  "properties": {
    "$bytes": {"type": "string"}
  }
}

ubl_capsule_schema = {
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://spec.vv.ai/ubl/1/ubl-capsule.v1.schema.json",
  "title": "UBL Capsule v1",
  "type": "object",
  "required": ["v", "id", "hdr", "env", "seal"],
  "properties": {
    "v": {"const": "ubl-capsule/1.0"},
    "id": {"type": "string"},
    "hdr": {
      "type": "object",
      "required": ["src","dst","nonce","exp"],
      "additionalProperties": False,
      "properties": {
        "src": {"type": "string", "pattern": r"^[\x20-\x7E]+$"},
        "dst": {"type": "string", "pattern": r"^[\x20-\x7E]+$"},
        "nonce": {"type": "string"},
        "exp": {"type": "integer"},
        "chan": {"type": "string", "pattern": r"^[\x20-\x7E]+$"},
        "ts": {"type": "integer"}
      }
    },
    "env": {
      "type": "object",
      "required": ["v","t","intent","decision"],
      "additionalProperties": True,
      "properties": {
        "v": {"const": "ai-json-nrf1/1.0"},
        "t": {"enum": ["record","bundle","trace","query"]},
        "agent": {"type": "object"},
        "intent": {"type": "object", "required": ["kind","name"]},
        "ctx": {"type": "object"},
        "decision": {
          "type": "object",
          "required": ["verdict"],
          "properties": {
            "verdict": {"enum": ["ACK","NACK","ASK"]},
            "reason": {"type": "string"},
            "metrics": {"type": "object"}
          }
        },
        "evidence": {"type": "object"},
        "meta": {"type": "object"},
        "links": {"type": "object"}
      }
    },
    "seal": {
      "type": "object",
      "required": ["alg","kid","domain","scope","sig"],
      "additionalProperties": False,
      "properties": {
        "alg": {"enum": ["Ed25519","Dilithium3"]},
        "kid": {"type": "string", "pattern": r"^[\x20-\x7E]+$"},
        "domain": {"const": "ubl-capsule/1.0"},
        "scope": {"const": "capsule"},
        "aud": {"type": "string", "pattern": r"^[\x20-\x7E]+$"},
        "sig": {"type": "string"}
      }
    },
    "receipts": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["of","prev","kind","node","ts","sig"],
        "properties": {
          "of": {"type": "string"},
          "prev": {"type": "string"},
          "kind": {"type": "string"},
          "node": {"type": "string", "pattern": r"^[\x20-\x7E]+$"},
          "ts": {"type": "integer"},
          "sig": {"type": "string"}
        }
      }
    }
  }
}

paths = {}
paths["spec"] = os.path.join(base_dir, "AI-NRF1_UBL-Capsule_v1_SPEC.md")
with open(paths["spec"], "w", encoding="utf-8") as f:
    f.write(spec_md)

paths["ai_json_schema"] = os.path.join(base_dir, "ai-json-nrf1.schema.json")
with open(paths["ai_json_schema"], "w", encoding="utf-8") as f:
    json.dump(ai_json_schema, f, indent=2, ensure_ascii=False)

paths["ubl_capsule_schema"] = os.path.join(base_dir, "ubl-capsule.v1.schema.json")
with open(paths["ubl_capsule_schema"], "w", encoding="utf-8") as f:
    json.dump(ubl_capsule_schema, f, indent=2, ensure_ascii=False)

def b3(path):
    h = hashlib.blake2s(digest_size=32)
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1<<16), b""):
            h.update(chunk)
    return "b3:" + h.hexdigest()

summary = {
    "files": {
        "spec_md": {"path": paths["spec"], "hash": b3(paths["spec"])},
        "ai_json_schema": {"path": paths["ai_json_schema"], "hash": b3(paths["ai_json_schema"])},
        "ubl_capsule_schema": {"path": paths["ubl_capsule_schema"], "hash": b3(paths["ubl_capsule_schema"])},
    }
}
summary
```

Resultado

undefined

amo essa convergência, Dan — ficou elegante e fácil de vender 💥

nomes — fechado
---------------

*   **Norma (técnica)**: `ai-nrf1` (bytes canônicos) e `ai-json-nrf1` (view JSON).
*   **Marca (produto/protocolo)**: `ubl-byte` e `ubl-json`.
*   Mantemos o “**1**” de `ai-nrf1` como major version (e dá a leitura boa de “1 byte, 1 hash, 1 decisão”). ✅

entregas agora
--------------

Gerei o **doc oficial** e dois **schemas** já com a nomenclatura final:

*   AI-NRF1 & UBL Capsule v1 — Especificação Oficial (Markdown)  
    `b3:30ca2f92e00770d54f8b8a57cc0b4b966c60f390673c55dbfde8b6703b86a406`
*   Schema — ai-json-nrf1 (view)  
    `b3:a8b1f4f365cee6b1784c8f998e22a14293ab635a20e13164842054a813df1865`
*   Schema — ubl-capsule v1  
    `b3:28f4c410a82ec16ff2504370dedcffb35995b55eb2a345770108cf1982627907`

### o que tem dentro (bem direto)

*   **AI-NRF1**: tags, varint mínimo, map ordenado por bytes UTF-8, sem float, decimais canônicos por regex, ρ (strings NFC, timestamps UTCZ, set por bytes NRF).
*   **UBL Capsule v1**: estrutura `{v,id,hdr,env,seal,receipts[]}`, regras de **ID** (exclui `id`, `seal.sig`, `receipts[*].sig`), **assinaturas** (seal assina `{domain,id,hdr,env}`; hop assina `{of,prev,kind,node,ts}`), ASK/ACK/NACK, ASCII-only para DIDs/KIDs, receipts encadeadas.
*   **AI-JSON-NRF1**: view humana/LLM; `Bytes` como `b3:<hex>`/`b64:<…>`; encoder aplica ρ→NRF.
*   **KATs, ABNF, segurança e interop** (inclui WBE/HAL e nota de permits).

próximos 2 passos (rápidos)
---------------------------

1.  **Wire no repo**
    *   `docs/AI-NRF1_UBL-Capsule_v1_SPEC.md`
    *   `schemas/ai-json-nrf1.schema.json`
    *   `schemas/ubl-capsule.v1.schema.json`
    *   manter aliases antigos com aviso deprecação no CLI.
2.  **CLI/SDK**
    *   comandos: `ai nrf1 encode|decode|hash` e `ai ubl cap sign|verify|view-json`.
    *   render da view com `Bytes` → `b3:`/`b64:`.

se quiser, já te mando um patch adicionando esses 3 arquivos no repo + atualizar help do CLI com os nomes novos. quer que eu aplique? 🚀🟢

## Prompt:
Perfeito, Dan! 💪🔥
Segue a ordem de implementação para chegar rápido num protótipo robusto da UBL Capsule v1, mapeando cada item da sua checklist para tarefas, artefatos e critérios de aceite. Foque em fechar Canon → ID → Assinatura → Hops, e o resto “encaixa”.

Plano em fases (com entregáveis e testes)

Fase 0 — Fundamentos de Canon (NRF) ⚙️

Objetivo: “A verdade são os bytes”: encoder/decoder NRF, regras duras e JSON view derivada.
	1.	NRF Core

	•	Tarefas:
	•	Implementar ubl_nrf::encode/decode (keys ordenadas, NFC, UTF-8 válido, varint minimal, sem floats).
	•	Tipos binários (Bytes) para id, nonce, sig, cids.
	•	Artefatos: impl/rust/ubl_nrf/, tests/vectors/ (unicode NFC, varint minimal, BOM forbidden).
	•	Aceite: round-trip determinístico; rejeita Null/floats/UTF-8 inválido; vetores passam 100%.

	2.	JSON View derivada

	•	Tarefas:
	•	ubl_json_view::{to_json,from_json} com prefixos (b3:, b64:, n:).
	•	Parse MUST validar NFC/ASCII/sem floats antes de materializar NRF.
	•	Artefatos: impl/rust/ubl_json_view/, schemas/ubl-capsule.view.json.
	•	Aceite: JSON↔NRF idempotente; violação → erro padronizado.

⸻

Fase 1 — Identidade e Assinatura (sem paradoxos) 🧱

Objetivo: ID estável e seal correto com domain separation.
	3.	ID estável

	•	Tarefas:
	•	capsule_id(c) = blake3(nrf.encode(c \ {id, seal.sig, receipts[*].sig})).
	•	Artefatos: impl/rust/ubl_capsule/id.rs.
	•	Aceite: anexar/remover receipts não altera id; tamper no env/hdr altera.

	4.	Seal (assinatura principal)

	•	Tarefas:
	•	sign(c): insere id; assina blake3(nrf.encode({domain,id,hdr,env})).
	•	Campos obrigatórios: domain="ubl-capsule/1.0", scope="capsule", aud==dst (se presente).
	•	Artefatos: impl/rust/ubl_capsule/seal.rs.
	•	Aceite: tamper detectado; domain/scope/aud verificados; Ed25519 e Dilithium3 testados.

⸻

Fase 2 — Hops/Custódia (cadeia encadeada) 🧬

Objetivo: Encaminhamento auditável append-only.
	5.	Receipts encadeadas

	•	Tarefas:
	•	Estrutura: {of, prev, kind, node, ts, sig}; of==capsule.id.
	•	receipt_id = blake3(nrf.encode(payload_sem_sig)) para prev.
	•	Assinatura: domain="ubl-receipt/1.0".
	•	Artefatos: impl/rust/ubl_receipt/, tests/receipts_chain/.
	•	Aceite: ordem verificável; remoção/reordenação detectada; múltiplos hops OK.

	6.	Anti-replay & dedupe

	•	Tarefas:
	•	hdr.nonce: Bytes(16) único por hdr.src.
	•	hdr.exp: i64 nanos; transporte deriva TTL.
	•	Regras: dedupe global por id; replay cache (src,nonce).
	•	Artefatos: impl/rust/ubl_runtime/replay.rs.
	•	Aceite: replay bloqueado; expiração respeitada (tolerância por policy).

⸻

Fase 3 — Semântica LLM-first (env) 🧠

Objetivo: env minimal, determinístico, pronto pra TDLN.
	7.	Gramática env (ai-json-nrf1 v0.1.1)

	•	Tarefas:
	•	t ∈ {record,bundle,trace}; intent.kind ∈ {ATTEST,EVAL,BUNDLE,TRACE,QUERY}.
	•	decision.verdict ∈ {ACK,NACK,ASK}; invariantes: ASK → links.prev, ACK/NACK → evidence.
	•	ctx pequeno; blobs via evidence.cids.
	•	Artefatos: specs/ai-json-nrf1.v0.1.1.md, schemas/ai-json-nrf1.v0.1.1.json.
	•	Aceite: validadores passam; vetores ASK/ACK/NACK cobrem invariantes.

	8.	Separação de camadas

	•	Tarefas:
	•	Garantir que policy/runtime só leem env; transporte só hdr.
	•	Artefatos: impl/rust/ubl_policy_api/, doc “ABI restrita”.
	•	Aceite: testes bloqueiam acesso indevido a hdr no runtime.

⸻

Fase 4 — Tooling & CI 🚦

Objetivo: DX e segurança “sempre verde”.
	9.	CLI

	•	Subcomandos:
	•	ubl cap canon|hash|sign|verify|view-json|from-json|to-json
	•	ubl cap receipt add --kind relay --node <did#key>
	•	Aceite: exemplos rodando end-to-end.

	10.	CI de conformidade

	•	Checks: canonicalidade, ID/Seal, receipts chain, ASK/ACK/NACK invariantes, exp, JSON view.
	•	Fuzz: parsers NRF; corpora unicode fixados.
	•	Aceite: matriz CI (Linux/macOS), sem flakes.

⸻

Fase 5 — Integrações mínimas (base, sem “produto”) 🧩

Objetivo: Pronto para uso pelos outros módulos (SIRP/TDLN).
	11.	URLs ricas (portabilidade)

	•	Tarefas: evidence.urls com ubl://did/rec/<cid>; resolve offline→S3/SQL por config (fora do canônico).
	•	Aceite: resolver/“air-gapped” OK.

	12.	Schemas de erro + docs

	•	Tarefas: padronizar Err.*; tabela em specs/errors.md.
	•	Aceite: cobertura de erros em testes.

⸻

Ordem estrita (checklist rapidinho)
	1.	✅ NRF Core (bytes, NFC, sem floats)
	2.	✅ JSON view derivada (prefixos, validação)
	3.	✅ ID sem paradoxos (exclui id/sigs)
	4.	✅ Seal com domain/scope/aud (assinatura do core)
	5.	✅ Receipts encadeadas (append-only)
	6.	✅ Anti-replay (nonce) + exp (em vez de ttl) + dedupe
	7.	✅ Gramática env (LLM-first, invariantes)
	8.	✅ Isolamento env vs hdr (policy/runtime)
	9.	✅ CLI (hash/sign/verify/receipt)
	10.	✅ CI de conformidade + fuzz
	11.	✅ URLs ricas (resolver offline)
	12.	✅ Erros padronizados + docs

Critério “MVP-Base”:
Conseguimos: (a) criar cápsula, (b) assinar/verificar, (c) anexar hops, (d) validar invariantes ASK/ACK/NACK, (e) re-materializar JSON view, (f) provar canonicidade por bytes. Tudo com CI.

Se topar essa ordem, sigo nessa trilha nas próximas mensagens aplicando no bundle grande — começando por NRF Core + ID + Seal (Fases 0–1). 🚀

## Response:
Perfeito, Dan — a ordem está ótima e cobre exatamente o eixo “Canon → ID → Seal → Hops”. ✅  
Topado. Vou só “apertar 4 parafusos” pra deixar à prova de bala e já te deixo com DoD + testes de aceitação por fase pra você rodar agora. 🔧🔥

Ajustes mínimos (sem mudar o plano)
===================================

1.  Prefixos da view JSON
    *   **b3:** = 32B (BLAKE3) em hex minúsculo
*   **b64:** = bytes arbitrários (assinaturas, blobs)
*   **n:** = Int64 RFC (apenas para _debug/view_, nunca canon)
    2.  ASCII estrito para `hdr.src`, `hdr.dst`, `seal.kid`, `seal.domain`, `seal.scope`, `seal.aud`
    *   Rejeitar qualquer coisa fora de `[\x21-\x7E]` (sem espaços). Erro: `Err.Canon.NotASCII`.
    3.  `id` e `seal.sig` fora do _hash material_
    *   `id = blake3( nrf.encode( capsule \ {id, seal.sig, receipts[*].sig} ) )`
*   `seal.sig = sign( blake3( nrf.encode({domain,id,hdr,env}) ) )`
*   **Garantia**: adicionar/remover receipts não muda `id`.
    4.  Receipts encadeadas
    *   `payload = {domain:"ubl-receipt/1.0", of, prev, kind, node, ts}`
*   `receipt_id = blake3(nrf.encode(payload))` → vira `prev` do próximo hop
*   `sig = sign(blake3(nrf.encode(payload)))`

* * *

Definition of Done + Testes de Aceitação
========================================

Fase 0 — NRF Core (⚙️)
----------------------

**DoD**

*   `ubl_nrf::encode/decode` determinístico (Map ordenado por UTF-8, varint32 minimal, sem float, strings NFC, sem BOM).
*   Tipo **Bytes** canônico (aceita vazio; comprimento qualquer; `{"$bytes": ""}` OK).

**Tests**

*   🔸 KATs: `null`, `true/false`, `int64`, `string "é" (NFC)`, `map {“a”:1,“b”:2}` (ordem por bytes), `bytes ""` e `bytes 0x00ff`.
*   🔸 Rejeições: `NotNFC`, `InvalidUTF8`, `NonMinimalVarint`, `DuplicateKey`, `UnsortedKeys`.
*   🔸 Idempotência: `encode(decode(encode(x))) == encode(x)`.

Fase 1 — ID + Seal (🧱)
-----------------------

**DoD**

*   `capsule_id()` implementado conforme exclusões.
*   `seal.sign()` e `seal.verify()` com `domain="ubl-capsule/1.0"`, `scope="capsule"`, `aud==hdr.dst` (se presente).
*   Suporte **Ed25519** (P0) e _stub_ de Dilithium3 (pode retornar `Err.Seal.UnsupportedAlg` no P0 se preferir).

**Tests**

*   🔸 `id` estável ao adicionar/remover receipts.
*   🔸 _Tamper tests_: flip em `env.ctx`, `hdr.src`, `intent.kind` → `verify` falha.
*   🔸 `aud` incorreto → `Err.Seal.ScopeDomain`.
*   🔸 Vetores com bytes exatos (b3 do core) cruzando Python↔Rust.

Fase 2 — Hops/Custódia (🧬)
---------------------------

**DoD**

*   Estrutura `receipt { of, prev, kind, node, ts, sig }`, `of == capsule.id`.
*   Verificação da cadeia: `prev` forma lista ligada por `receipt_id`.
*   `hdr.nonce: Bytes(16)` único por `hdr.src`; `hdr.exp: Int64` (nanos).
*   _Dedupe_: cache `(hdr.src, hdr.nonce)`.

**Tests**

*   🔸 3 hops (A→B→C): cadeia válida; reordenação detectada; remoção detectada.
*   🔸 Replay: mesmo `(src,nonce)` rejeitado.
*   🔸 Expiração: `now > exp` → `Err.Hdr.Expired`.

Fase 3 — Env (LLM-first) (🧠)
-----------------------------

**DoD**

*   `env.v="ai-json-nrf1/0.1.1"`, `t ∈ {record,bundle,trace}`, `intent.kind ∈ {ATTEST,EVAL,BUNDLE,TRACE,QUERY}`.
*   `decision.verdict ∈ {ACK,NACK,ASK}`; **invariantes**:
    *   ASK ⇒ `links.prev` obrigatório
    *   ACK/NACK ⇒ `evidence` presente (pode estar vazia)

**Tests**

*   🔸 Validadores de schema (ACK/NACK/ASK) + checagem das invariantes.
*   🔸 Bloqueio de acesso a `hdr` no runtime/policy (teste “caixa-preta”).

Fase 4 — CLI & CI (🚦)
----------------------

**DoD**

*   `ubl cap canon|hash|sign|verify|view-json|from-json|to-json`
*   `ubl cap receipt add --kind relay --node <did#key>`
*   CI: conformance + fuzz básico (NRF decode) + _unicode corpus_ fixo.

**Tests**

*   🔸 `view-json` respeita `b3:`/`b64:`/`n:` e round-trip.
*   🔸 Pipeline de exemplo: **ATTEST→EVAL** via `links.prev` (ASK→ACK).

* * *

Vetores rápidos (p/ começar agora)
==================================

**Capsule (ASK)** _view JSON, canon sobre NRF_:

```json
{
  "v": "ubl-capsule/1.0",
  "id": "b3:<computed>",
  "hdr": {
    "src": "did:ex:src#k1",
    "dst": "did:ex:dst#kA",
    "nonce": "b64:AAAAAAAAAAAAAAAAAAAAAA==",
    "exp": 1739059200000000000
  },
  "env": {
    "v": "ai-json-nrf1/0.1.1",
    "t": "record",
    "intent": {"kind": "EVAL", "name":"policy.demo"},
    "ctx": {"subject":{"doc_cid":"b3:0123..."}},
    "decision": {"verdict":"ASK", "reason":"need-proof"},
    "evidence": {},
    "links": {"prev":"b3:89ab..."}
  },
  "seal": {"alg":"Ed25519","kid":"did:ex:src#k1","domain":"ubl-capsule/1.0","scope":"capsule","sig":"b64:<64B>"},
  "receipts": []
}
```

**Hop (relay)**: `prev = receipt_id anterior (ou zero no 1º)`, `of = capsule.id`.

* * *

Comandos de verificação (sugestão)
==================================

```bash
