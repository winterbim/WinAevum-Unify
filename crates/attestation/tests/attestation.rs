use aevum_attestation::{ActionAttestation, AttestationSigner, RiskClass, VerificationError};
use aevum_identity::Identity;

fn sample_attestation() -> ActionAttestation {
    ActionAttestation {
        schema_version: "aevum.action-attestation/v1".to_string(),
        attestation_id: "aat_01JCTEST000000000000000000".to_string(),
        action_id: "act_01JCTEST000000000000000000".to_string(),
        mission_id: "mis_01JCTEST000000000000000000".to_string(),
        constitution_version: 1,
        constitution_digest: "sha256:demo".to_string(),
        policy_bundle_digest: "sha256:demo".to_string(),
        principal_id: "spiffe://local.aevum/agent/producer-1".to_string(),
        agent_definition: "code-builder@1.2.0".to_string(),
        council_role: "producer".to_string(),
        capability: "git.branch.create".to_string(),
        resource: "github:winterbim/example-app".to_string(),
        parameters_digest: "sha256:demo".to_string(),
        expected_effects: vec!["branch aevum/sec-fix created".to_string()],
        forbidden_effects: vec!["main modified".to_string()],
        evidence_required: vec!["repo_state".to_string()],
        evidence_attached: vec!["evd_1".to_string()],
        evidence_completeness: 1.0,
        risk_class: RiskClass::R2,
        risk_score: 28,
        reversible: true,
        blast_radius: "single_repository".to_string(),
        approval_ids: vec![],
        not_before: "2026-08-02T10:00:00+02:00".to_string(),
        expires_at: "2099-08-02T10:10:00+02:00".to_string(),
        max_uses: 1,
        recovery_strategy: "delete_branch".to_string(),
        recovery_verified: true,
        nonce: "abcdef0123456789".to_string(),
        signature: None,
    }
}

#[test]
fn canonical_bytes_exclude_signature_field() {
    let mut b = sample_attestation();
    b.signature = Some("ed25519:fake".to_string());
    let a = sample_attestation();
    let ca = a.canonical_bytes().expect("canon a");
    let cb = b.canonical_bytes().expect("canon b");
    assert_eq!(ca, cb, "signature field must not influence canonical bytes");
}

#[test]
fn sign_then_verify_succeeds_for_valid_payload() {
    let signer = AttestationSigner::new(Identity::ephemeral("agent/producer-1"));
    let attestation = signer.sign(sample_attestation()).expect("sign");
    assert!(
        attestation.signature.is_some(),
        "signer must populate signature"
    );
    signer
        .verify(&attestation)
        .expect("verify ok on unchanged payload");
}

#[test]
fn verify_rejects_tampered_constitution_digest() {
    let signer = AttestationSigner::new(Identity::ephemeral("producer-1"));
    let signed = signer.sign(sample_attestation()).expect("sign");
    let mut a = signed.clone();
    a.constitution_digest = "sha256:tampered".to_string();
    let err = signer.verify(&a).unwrap_err();
    assert!(matches!(err, VerificationError::SignatureMismatch(_)));
}

#[test]
fn verify_rejects_expired_attestation() {
    let signer = AttestationSigner::new(Identity::ephemeral("producer-1"));
    let mut a = sample_attestation();
    a.not_before = "2025-01-01T00:00:00+00:00".to_string();
    let mut signed = signer.sign(a).expect("sign");
    signed.expires_at = "2025-01-02T09:00:00+00:00".to_string();
    let re_signed = signer.sign(signed.clone()).expect("re-sign");
    let err = signer.verify(&re_signed).unwrap_err();
    assert!(matches!(err, VerificationError::Expired { .. }));
}

#[test]
fn verify_rejects_pre_not_before_attestation() {
    let signer = AttestationSigner::new(Identity::ephemeral("producer-1"));
    let mut a = sample_attestation();
    a.not_before = "2099-01-01T00:00:00+00:00".to_string();
    a.expires_at = "2099-12-31T00:00:00+00:00".to_string();
    let signed = signer.sign(a).expect("sign");
    let err = signer.verify(&signed).unwrap_err();
    assert!(matches!(err, VerificationError::NotYetValid { .. }));
}
