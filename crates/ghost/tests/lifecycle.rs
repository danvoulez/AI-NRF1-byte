use ghost::*;
use ed25519_dalek::SigningKey;

fn make_test_ghost() -> Ghost {
    let wbe = Wbe {
        who: "did:ubl:actor".into(),
        what: "evaluate insurance claim".into(),
        when: 1_700_000_000_000_000_000,
        intent: "EVALUATE".into(),
    };
    Ghost::new_pending(wbe, vec![0u8; 16], "https://example.com/ghosts/test.json".into())
}

#[test]
fn test_ghost_new_pending() {
    let g = make_test_ghost();
    assert_eq!(g.status, GhostStatus::Pending);
    assert!(g.cause.is_none());
    assert!(g.ghost_cid.starts_with("b3:"));
}

#[test]
fn test_ghost_cid_deterministic() {
    let g = make_test_ghost();
    let c1 = g.compute_cid();
    let c2 = g.compute_cid();
    assert_eq!(c1, c2);
}

#[test]
fn test_ghost_integrity_pending_ok() {
    let g = make_test_ghost();
    assert!(g.verify_integrity().is_ok());
}

#[test]
fn test_ghost_expire() {
    let mut g = make_test_ghost();
    g.expire(ExpireCause::Timeout);
    assert_eq!(g.status, GhostStatus::Expired);
    assert_eq!(g.cause, Some(ExpireCause::Timeout));
    assert!(g.verify_integrity().is_ok());
}

#[test]
fn test_ghost_pending_with_cause_fails() {
    let mut g = make_test_ghost();
    // Force invalid state: pending but with cause
    g.cause = Some(ExpireCause::Canceled);
    g.ghost_cid = g.compute_cid(); // recompute so CID matches
    assert!(g.verify_integrity().is_err());
}

#[test]
fn test_ghost_expired_without_cause_fails() {
    let mut g = make_test_ghost();
    g.status = GhostStatus::Expired;
    g.cause = None; // invalid: expired must have cause
    g.ghost_cid = g.compute_cid();
    assert!(g.verify_integrity().is_err());
}

#[test]
fn test_ghost_sign_and_verify() {
    let mut rng = rand::thread_rng();
    let sk = SigningKey::generate(&mut rng);
    let vk = sk.verifying_key();

    let mut g = make_test_ghost();
    g.sign(&sk);
    assert!(g.sig.is_some());
    assert!(g.verify(&vk));
}

#[test]
fn test_ghost_verify_wrong_key() {
    let mut rng = rand::thread_rng();
    let sk = SigningKey::generate(&mut rng);
    let wrong_sk = SigningKey::generate(&mut rng);
    let wrong_vk = wrong_sk.verifying_key();

    let mut g = make_test_ghost();
    g.sign(&sk);
    assert!(!g.verify(&wrong_vk));
}

#[test]
fn test_ghost_expire_invalidates_sig() {
    let mut rng = rand::thread_rng();
    let sk = SigningKey::generate(&mut rng);
    let vk = sk.verifying_key();

    let mut g = make_test_ghost();
    g.sign(&sk);
    assert!(g.verify(&vk));

    // Expire changes the CID → old sig is invalid
    g.expire(ExpireCause::Drift);
    assert!(!g.verify(&vk));
    assert!(g.sig.is_none()); // expire clears sig
}

#[test]
fn test_ghost_as_ref() {
    let g = make_test_ghost();
    let r = g.as_ref("storage-id-123");
    assert_eq!(r.ghost_id, "storage-id-123");
    assert_eq!(r.ghost_cid, g.ghost_cid);
}
