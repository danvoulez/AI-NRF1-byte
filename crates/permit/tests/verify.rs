use permit::*;
use ed25519_dalek::SigningKey;

fn make_test_permit() -> Permit {
    Permit {
        v: "permit-v1".into(),
        permit_cid: String::new(),
        request_cid: "b3:aaaa".into(),
        decision: "ALLOW".into(),
        input_hash: "b3:bbbb".into(),
        issuer_did: "did:ubl:authority".into(),
        issued_at: 1_700_000_000_000_000_000,
        expires_at: 1_800_000_000_000_000_000,
        act: "EVALUATE".into(),
        policy: Some("pack-compliance/eu-ai-act@1".into()),
        sig: None,
    }
}

#[test]
fn test_permit_cid_deterministic() {
    let p = make_test_permit();
    let c1 = p.compute_cid();
    let c2 = p.compute_cid();
    assert_eq!(c1, c2);
    assert!(c1.starts_with("b3:"));
}

#[test]
fn test_permit_sign_and_verify() {
    let mut rng = rand::thread_rng();
    let sk = SigningKey::generate(&mut rng);
    let vk = sk.verifying_key();

    let mut p = make_test_permit();
    p.permit_cid = p.compute_cid();
    p.sign(&sk);

    let now = 1_750_000_000_000_000_000; // between issued_at and expires_at
    assert!(verify_permit(&p, "b3:bbbb", now, &vk).is_ok());
}

#[test]
fn test_permit_expired() {
    let mut rng = rand::thread_rng();
    let sk = SigningKey::generate(&mut rng);
    let vk = sk.verifying_key();

    let mut p = make_test_permit();
    p.permit_cid = p.compute_cid();
    p.sign(&sk);

    let now = 1_900_000_000_000_000_000; // after expires_at
    let err = verify_permit(&p, "b3:bbbb", now, &vk).unwrap_err();
    assert!(matches!(err, PermitError::Expired));
}

#[test]
fn test_permit_input_mismatch() {
    let mut rng = rand::thread_rng();
    let sk = SigningKey::generate(&mut rng);
    let vk = sk.verifying_key();

    let mut p = make_test_permit();
    p.permit_cid = p.compute_cid();
    p.sign(&sk);

    let now = 1_750_000_000_000_000_000;
    let err = verify_permit(&p, "b3:wrong", now, &vk).unwrap_err();
    assert!(matches!(err, PermitError::InputMismatch));
}

#[test]
fn test_permit_wrong_key() {
    let mut rng = rand::thread_rng();
    let sk = SigningKey::generate(&mut rng);
    let wrong_sk = SigningKey::generate(&mut rng);
    let wrong_vk = wrong_sk.verifying_key();

    let mut p = make_test_permit();
    p.permit_cid = p.compute_cid();
    p.sign(&sk);

    let now = 1_750_000_000_000_000_000;
    let err = verify_permit(&p, "b3:bbbb", now, &wrong_vk).unwrap_err();
    assert!(matches!(err, PermitError::BadSignature));
}
