#![allow(missing_docs)]
//! Aevum Unify — autonomy governor (M7).
//!
//! Maps a `RiskClass` (and an environment hint) to the approval requirement
//! implied by the autonomy policy. Manages per-actor profiles with explicit
//! privileges that decay after their TTL.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum RiskClass {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
}

impl RiskClass {
    pub fn from_label(s: &str) -> Option<Self> {
        match s.trim().to_uppercase().as_str() {
            "R0" => Some(Self::R0),
            "R1" => Some(Self::R1),
            "R2" => Some(Self::R2),
            "R3" => Some(Self::R3),
            "R4" => Some(Self::R4),
            "R5" => Some(Self::R5),
            _ => None,
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            Self::R0 => 0,
            Self::R1 => 1,
            Self::R2 => 2,
            Self::R3 => 3,
            Self::R4 => 4,
            Self::R5 => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalRequirement {
    /// Action executes immediately.
    None,
    /// Action executes and emits a journal acknowledgement.
    Acknowledgement { note: String },
    /// Wait for an explicit human `approval_id`.
    HumanApproval {
        reason: String,
        approver_roles: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorProfile {
    pub actor_id: String,
    pub max_risk: RiskClass,
    pub autonomy_level: u8, // 0..=3
    pub privileges_expire_at: Option<String>,
}

impl ActorProfile {
    pub fn new(actor_id: &str, max_risk: RiskClass) -> Self {
        Self {
            actor_id: actor_id.to_string(),
            max_risk,
            // Initial TTL is 4 hours; production-grade deployments should refresh.
            privileges_expire_at: Some("2026-08-02T16:00:00+00:00".to_string()),
            autonomy_level: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutonomyGovernor {
    /// Default policy table — overridable per tenant.
    pub policy: ApprovalPolicy,
}

impl Default for AutonomyGovernor {
    fn default() -> Self {
        Self {
            policy: ApprovalPolicy::default_table(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApprovalPolicy {
    pub production_forces_human_above: RiskClass,
    pub staging_forces_human_above: RiskClass,
    pub require_human_above: RiskClass,
    pub acknowledge_above: RiskClass,
}

impl ApprovalPolicy {
    pub fn default_table() -> Self {
        Self {
            production_forces_human_above: RiskClass::R1,
            staging_forces_human_above: RiskClass::R3,
            require_human_above: RiskClass::R3,
            acknowledge_above: RiskClass::R2,
        }
    }
}

impl AutonomyGovernor {
    pub fn requirement_for(&self, risk: RiskClass) -> ApprovalRequirement {
        if risk.rank() >= self.policy.require_human_above.rank() {
            ApprovalRequirement::HumanApproval {
                reason: format!("risk class {risk:?} requires explicit human approval"),
                approver_roles: vec!["admin".into(), "operator".into()],
            }
        } else if risk.rank() >= self.policy.acknowledge_above.rank() {
            ApprovalRequirement::Acknowledgement {
                note: format!("risk class {risk:?} requires a recorded acknowledgement"),
            }
        } else {
            ApprovalRequirement::None
        }
    }

    pub fn requirement_for_environment(&self, risk: RiskClass, env: &str) -> ApprovalRequirement {
        let env_low = env.to_lowercase();
        let effective_threshold = match env_low.as_str() {
            "production" => self.policy.production_forces_human_above,
            "staging" => self.policy.staging_forces_human_above,
            _ => RiskClass::R5,
        };
        if risk.rank() >= effective_threshold.rank() {
            ApprovalRequirement::HumanApproval {
                reason: format!("environment={env_low} elevates threshold for {risk:?}"),
                approver_roles: vec!["admin".into()],
            }
        } else {
            self.requirement_for(risk)
        }
    }

    pub fn record_decision(&self, profile: &mut ActorProfile, approved: bool) {
        if approved {
            if profile.autonomy_level < 3 {
                profile.autonomy_level += 1;
            }
        } else {
            // Decay: refused actions knock autonomy down by one step.
            if profile.autonomy_level > 0 {
                profile.autonomy_level -= 1;
            }
        }
    }

    pub fn is_privilege_active(&self, profile: &ActorProfile, now: &str) -> bool {
        match &profile.privileges_expire_at {
            Some(exp) => match parse_iso_to_seconds(exp) {
                Some(e) => match parse_iso_to_seconds(now) {
                    Some(n) => n < e,
                    None => true,
                },
                None => false,
            },
            None => false,
        }
    }
}

fn parse_iso_to_seconds(s: &str) -> Option<u64> {
    let body = s.split('+').next().or_else(|| s.split('Z').next())?;
    let parts: Vec<&str> = body.split('T').collect();
    if parts.len() != 2 {
        return None;
    }
    let date: Vec<u32> = parts[0].split('-').filter_map(|x| x.parse().ok()).collect();
    let time: Vec<u32> = parts[1]
        .split(':')
        .filter_map(|x| x.split_terminator('.').next()?.parse().ok())
        .collect();
    if date.len() != 3 || time.len() < 3 {
        return None;
    }
    let days_from_epoch = days_since_1970(date[0], date[1], date[2])?;
    Some(
        days_from_epoch * 86_400
            + (time[0] as u64) * 3600
            + (time[1] as u64) * 60
            + (time[2] as u64),
    )
}

fn days_since_1970(y: u32, m: u32, d: u32) -> Option<u64> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let mut days: u64 = 0;
    for yy in 1970..y {
        days += if is_leap(yy) { 366 } else { 365 };
    }
    let months = [
        31u64,
        if is_leap(y) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    for mm in 1..m {
        days += months[(mm - 1) as usize];
    }
    days += (d - 1) as u64;
    Some(days)
}

fn is_leap(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}
