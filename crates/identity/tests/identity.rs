use aevum_identity::{Identity, KeyError, KeyMaterial};

#[test]
fn generated_key_produces_correct_sizes() {
    let km = KeyMaterial::generate();
    assert_eq!(km.secret_bytes().len(), 32);
    assert_eq!(km.public_bytes().len(), 32);
}

#[test]
fn sign_then_verify_succeeds() {
    let km = KeyMaterial::generate();
    let sig = km.sign(b"hello aevum");
    km.verify(&sig, b"hello aevum").expect("sig must verify");
}

#[test]
fn signature_does_not_verify_against_tampered_message() {
    let km = KeyMaterial::generate();
    let sig = km.sign(b"hello aevum");
    let err = km.verify(&sig, b"hello aevun").unwrap_err();
    assert!(matches!(err, KeyError::SignatureInvalid));
}

#[test]
fn identity_ephemeral_uses_spiffe_compatible_id() {
    let id = Identity::ephemeral("agent/producer-7");
    assert!(id.spiffe_id.starts_with("spiffe://local.aevum/"));
    assert!(id.spiffe_id.ends_with("/agent/producer-7"));
}

#[test]
fn secret_hex_round_trip_is_deterministic() {
    let km = KeyMaterial::generate();
    let hex = km.secret_hex();
    let restored = KeyMaterial::from_secret_hex(&hex).expect("parse");
    assert_eq!(km.secret_bytes(), restored.secret_bytes());
    // round-tripped key produces the same signature
    assert_eq!(km.sign(b"x"), restored.sign(b"x"));
}
