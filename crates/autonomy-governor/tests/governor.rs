use aevum_autonomy_governor::{ActorProfile, ApprovalRequirement, AutonomyGovernor, RiskClass};

#[test]
fn r0_actions_require_no_approval() {
    let g = AutonomyGovernor::default();
    let req = g.requirement_for(RiskClass::R0);
    assert!(matches!(req, ApprovalRequirement::None));
}

#[test]
fn r2_actions_require_acknowledgement() {
    let g = AutonomyGovernor::default();
    let req = g.requirement_for(RiskClass::R2);
    assert!(matches!(req, ApprovalRequirement::Acknowledgement { .. }));
}

#[test]
fn r3_and_above_require_explicit_human_approval() {
    let g = AutonomyGovernor::default();
    let req = g.requirement_for(RiskClass::R3);
    assert!(matches!(req, ApprovalRequirement::HumanApproval { .. }));
    let req5 = g.requirement_for(RiskClass::R5);
    assert!(matches!(req5, ApprovalRequirement::HumanApproval { .. }));
}

#[test]
fn initial_privileges_have_explicit_ttl() {
    let _g = AutonomyGovernor::default();
    let profile = ActorProfile::new("actor_test", RiskClass::R3);
    assert!(profile.privileges_expire_at.is_some(), "ttl must be set");
}

#[test]
fn privileges_decay_after_ttl() {
    let g = AutonomyGovernor::default();
    let mut profile = ActorProfile::new("actor_test", RiskClass::R2);
    // Force expiry: pretend ttl is in the past.
    profile.privileges_expire_at = Some("2026-01-01T00:00:00+00:00".to_string());
    let now = "2030-01-01T00:00:00+00:00".to_string();
    assert!(!g.is_privilege_active(&profile, &now));
}

#[test]
fn deny_reduces_autonomy_level_for_actor() {
    let g = AutonomyGovernor::default();
    let mut profile = ActorProfile::new("actor_leg", RiskClass::R2);
    g.record_decision(&mut profile, false);
    assert!(profile.autonomy_level <= 1);
}

#[test]
fn successful_acknowledgement_lets_actor_keep_their_level() {
    let g = AutonomyGovernor::default();
    let mut profile = ActorProfile::new("actor_ok", RiskClass::R2);
    g.record_decision(&mut profile, true);
    assert!(profile.autonomy_level >= 1);
}

#[test]
fn production_environment_forces_human_approval_for_deployment() {
    let g = AutonomyGovernor::default();
    let req = g.requirement_for_environment(RiskClass::R2, "production");
    assert!(matches!(req, ApprovalRequirement::HumanApproval { .. }));
}
