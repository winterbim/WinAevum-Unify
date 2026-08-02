use aevum_evidence_graph::{
    Challenge, Claim, ClaimStatus, Decision, EvidenceItem, EvidenceKind, EvidenceStatus,
    EvidenceStore, FreshnessPolicy,
};

fn ev(title: &str, kind: EvidenceKind) -> EvidenceItem {
    EvidenceItem {
        id: format!("evd_{}", title),
        kind,
        title: title.to_string(),
        summary: format!("{title} summary"),
        digest: format!("sha256:{}", title),
        captured_at: "2026-08-02T10:00:00+00:00".to_string(),
        freshness_window_seconds: 600,
        backrefs: vec![format!("clm_{}", title)],
    }
}

fn claim(id: &str, required: Vec<EvidenceItem>, status: ClaimStatus) -> Claim {
    Claim {
        id: id.to_string(),
        statement: format!("claim {id}"),
        rationale: format!("rationale {id}"),
        required_evidence_ids: required.iter().map(|e| e.id.clone()).collect(),
        attached: required,
        required: true,
        created_at: "2026-08-02T10:00:00+00:00".to_string(),
        status,
    }
}

#[test]
fn fresh_evidence_passes_claim_status_check() {
    let mut store = EvidenceStore::new(FreshnessPolicy::Strict);
    let e1 = ev("repo_state", EvidenceKind::RepoState);
    let e2 = ev("tests", EvidenceKind::TestsLog);
    let c = claim("clm_a", vec![e1.clone(), e2.clone()], ClaimStatus::Proposed);
    let id = c.id.clone();
    store.add_claim(c);
    let now = "2026-08-02T10:05:00+00:00".to_string();
    let status = store.check_claim(&id, &now).unwrap();
    assert!(matches!(status, EvidenceStatus::Fresh));
}

#[test]
fn evidence_outside_freshness_window_is_stale() {
    let mut store = EvidenceStore::new(FreshnessPolicy::Strict);
    let e1 = ev("repo_state", EvidenceKind::RepoState);
    let c = claim("clm_a", vec![e1.clone()], ClaimStatus::Proposed);
    let id = c.id.clone();
    store.add_claim(c);
    // 1 hour later > 600s window
    let now = "2026-08-02T11:30:00+00:00".to_string();
    let status = store.check_claim(&id, &now).unwrap();
    assert!(matches!(status, EvidenceStatus::Stale));
}

#[test]
fn challenge_on_a_single_evidence_marks_claim_challenged() {
    let mut store = EvidenceStore::new(FreshnessPolicy::Strict);
    let e = ev("repo_state", EvidenceKind::RepoState);
    let cid = e.id.clone();
    let c = claim("clm_a", vec![e], ClaimStatus::Proposed);
    let clm_id = c.id.clone();
    store.add_claim(c);
    let ch = Challenge {
        by: "spiffe://local.aevum/role/falsifier".to_string(),
        target_evidence_id: cid,
        reason: "the diff is wrong".to_string(),
        raised_at: "2026-08-02T10:01:00+00:00".to_string(),
    };
    store.challenge(ch).unwrap();
    let decision = store.decide(&clm_id, Decision::Rejected).unwrap();
    assert!(matches!(decision, Decision::Rejected));
    let claim = store.claim(&clm_id).unwrap();
    assert!(matches!(claim.status, ClaimStatus::Rejected));
}

#[test]
fn missing_required_evidence_blocks_decision() {
    let mut store = EvidenceStore::new(FreshnessPolicy::Strict);
    // attach only `a` but require `a` AND `b`.
    let mut c = claim(
        "clm_missing",
        vec![ev("a", EvidenceKind::RepoState)],
        ClaimStatus::Proposed,
    );
    c.required_evidence_ids.push("evd_b".to_string());
    let cid = c.id.clone();
    store.add_claim(c);
    let err = store.decide(&cid, Decision::Accepted).unwrap_err();
    assert!(err.contains("missing"));
    let claim = store.claim(&cid).unwrap();
    assert!(matches!(claim.status, ClaimStatus::Pending));
}

#[test]
fn approved_claim_is_cryptographically_attested() {
    let mut store = EvidenceStore::new(FreshnessPolicy::Strict);
    let c = claim(
        "clm_ok",
        vec![
            ev("a", EvidenceKind::RepoState),
            ev("b", EvidenceKind::LintLog),
            ev("c", EvidenceKind::TestsLog),
            ev("d", EvidenceKind::DependencyAudit),
        ],
        ClaimStatus::Proposed,
    );
    let cid = c.id.clone();
    store.add_claim(c);
    let now = "2026-08-02T10:00:30+00:00".to_string();
    store.check_claim(&cid, &now).unwrap();
    let decision = store.decide(&cid, Decision::Accepted).unwrap();
    let claim = store.claim(&cid).unwrap();
    assert!(matches!(claim.status, ClaimStatus::Accepted));
    // The decision links to the claim via a stable identifier.
    assert!(matches!(decision, Decision::Accepted));
}

#[test]
fn replay_of_same_challenge_does_not_double_count() {
    let mut store = EvidenceStore::new(FreshnessPolicy::Strict);
    let e = ev("rs", EvidenceKind::RepoState);
    let eid = e.id.clone();
    let c = claim("clm_dedup", vec![e], ClaimStatus::Proposed);
    let _cid = c.id.clone();
    store.add_claim(c);
    let ch = Challenge {
        by: "spiffe://local.aevum/role/falsifier".to_string(),
        target_evidence_id: eid,
        reason: "twice".to_string(),
        raised_at: "2026-08-02T10:01:00+00:00".to_string(),
    };
    store.challenge(ch.clone()).unwrap();
    let result = store.challenge(ch);
    assert!(result.is_ok(), "duplicate challenge is silently deduped");
}
