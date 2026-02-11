use modules_core::Cid;
use ubl_capsule::{Capsule, capsule_id, capsule_sign, Receipt, ReceiptPayload};

/// Monta a cápsula final (hdr/env fornecidos), calcula id, assina seal e encadeia receipts.
pub fn finalize_capsule(
    mut base: Capsule,                 // { v,hdr,env,seal{alg,kid,domain,scope}, receipts:[] }
    mut hop_ids: Vec<Cid>,             // ids calculados no runner (payloads sem sig)
    signer: &dyn Fn(&[u8]) -> anyhow::Result<Vec<u8>>, // assinatura do seal/receipt
    node_did: &str,
    ts_nanos: i64,
) -> anyhow::Result<Capsule> {
    // 1) id (exclui id e *.sig)
    let id = capsule_id(&base)?;
    base.id = id;

    // 2) seal.sig = sign(blake3(nrf.encode({domain,id,hdr,env})))
    capsule_sign(&mut base, signer)?;

    // 3) receipts -> of=id, prev encadeado
    let mut prev: Option<Cid> = None;
    for hop in hop_ids.drain(..) {
        let payload = ReceiptPayload {
            domain: "ubl-receipt/1.0".into(),
            of: id,
            prev,
            kind: "pipeline-hop".into(),
            node: node_did.into(),
            ts: ts_nanos,
        };
        let sig = signer(&payload.hash_bytes()?)?;
        base.receipts.push(Receipt { payload, sig });
        prev = Some(payload.hash());
    }
    Ok(base)
}
