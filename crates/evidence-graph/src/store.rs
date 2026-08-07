//! Legacy M6 claim↔evidence store (kept for backward compatibility).
//! Prefer [`crate::temporal::TemporalGraph`] for new work.

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::time::parse_iso_to_seconds;

#[derive(Debug, Error, PartialEq)]
pub enum GraphError {
    #[error("unknown claim: {0}")]
    UnknownClaim(String),
    #[error("unknown evidence: {0}")]
    UnknownEvidence(String),
    #[error("claim {0} is missing required evidence")]
    MissingRequired(String),
    #[error("operation rejected: {0}")]
    Rejected(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FreshnessPolicy {
    #[default]
    /// Every evidence must have a captured_at within window of `now`.
    Strict,
    /// Only the most recent evidence must satisfy window.
    Lenient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum EvidenceKind {
    RepoState,
    TestsLog,
    LintLog,
    BuildLog,
    DependencyAudit,
    UserSignoff,
    ConstitutionDigest,
    PolicyDecision,
    Objection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub id: String,
    pub kind: EvidenceKind,
    pub title: String,
    pub summary: String,
    pub digest: String,
    pub captured_at: String,
    pub freshness_window_seconds: u64,
    pub backrefs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub by: String,
    pub target_evidence_id: String,
    pub reason: String,
    pub raised_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimStatus {
    Proposed,
    Pending,
    Accepted,
    Rejected,
    Challenged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceStatus {
    Fresh,
    Stale,
    Challenged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub statement: String,
    pub rationale: String,
    pub required_evidence_ids: Vec<String>,
    pub attached: Vec<EvidenceItem>,
    pub required: bool,
    pub created_at: String,
    pub status: ClaimStatus,
}

#[derive(Default)]
pub struct EvidenceStore {
    claims: HashMap<String, Claim>,
    evidence: HashMap<String, EvidenceItem>,
    challenges: HashMap<String, Vec<Challenge>>,
    freshness: FreshnessPolicy,
}

impl EvidenceStore {
    pub fn new(freshness: FreshnessPolicy) -> Self {
        Self {
            freshness,
            ..Default::default()
        }
    }

    pub fn add_claim(&mut self, c: Claim) {
        for e in &c.attached {
            self.evidence.insert(e.id.clone(), e.clone());
        }
        self.claims.insert(c.id.clone(), c);
    }

    pub fn claim(&self, id: &str) -> Option<&Claim> {
        self.claims.get(id)
    }

    pub fn challenge(&mut self, c: Challenge) -> Result<(), GraphError> {
        if !self.evidence.contains_key(&c.target_evidence_id) {
            return Err(GraphError::UnknownEvidence(c.target_evidence_id));
        }
        let bucket = self
            .challenges
            .entry(c.target_evidence_id.clone())
            .or_default();
        let already = bucket.iter().any(|x| x.by == c.by && x.reason == c.reason);
        if !already {
            bucket.push(c.clone());
        }
        for claim in self.claims.values_mut() {
            let touches = claim.attached.iter().any(|e| e.id == c.target_evidence_id);
            if touches {
                if matches!(claim.status, ClaimStatus::Accepted) {
                    return Err(GraphError::Rejected(
                        "cannot challenge an already-Accepted claim".into(),
                    ));
                }
                claim.status = ClaimStatus::Challenged;
            }
        }
        Ok(())
    }

    pub fn check_claim(&self, id: &str, now: &str) -> Result<EvidenceStatus, GraphError> {
        let claim = self
            .claims
            .get(id)
            .ok_or_else(|| GraphError::UnknownClaim(id.to_string()))?;
        for e in &claim.attached {
            if let Some(chals) = self.challenges.get(&e.id) {
                if !chals.is_empty() {
                    return Ok(EvidenceStatus::Challenged);
                }
            }
        }
        if matches!(self.freshness, FreshnessPolicy::Strict) {
            for e in &claim.attached {
                if !within_window(e, now) {
                    return Ok(EvidenceStatus::Stale);
                }
            }
        }
        Ok(EvidenceStatus::Fresh)
    }

    pub fn decide(&mut self, id: &str, decision: Decision) -> Result<Decision, String> {
        let decision_now = "2099-01-01T00:00:00+00:00";
        let provided_set: BTreeSet<String> = self
            .claims
            .get(id)
            .map(|c| c.attached.iter().map(|e| e.id.clone()).collect())
            .unwrap_or_default();
        let required: Vec<String> = self
            .claims
            .get(id)
            .map(|c| c.required_evidence_ids.clone())
            .unwrap_or_default();
        for r in &required {
            if !provided_set.contains(r) {
                if let Some(c) = self.claims.get_mut(id) {
                    c.status = ClaimStatus::Pending;
                }
                return Err(format!("claim {id} is missing required evidence {r}"));
            }
        }
        let status = self
            .check_claim(id, decision_now)
            .map_err(|e| e.to_string())?;
        if decision == Decision::Accepted && !matches!(status, EvidenceStatus::Fresh) {
            let candidate_now = self
                .claims
                .get(id)
                .and_then(|c| {
                    let latest = c.attached.iter().map(|e| &e.captured_at).max()?;
                    Some(latest.clone())
                })
                .unwrap_or_else(|| decision_now.to_string());
            let recheck = self
                .check_claim(id, &candidate_now)
                .map_err(|e| e.to_string())?;
            if !matches!(recheck, EvidenceStatus::Fresh) {
                if let Some(c) = self.claims.get_mut(id) {
                    c.status = ClaimStatus::Pending;
                }
                return Err(format!("claim {id} is not in a Fresh state ({recheck:?})"));
            }
        }
        if let Some(c) = self.claims.get_mut(id) {
            c.status = match decision {
                Decision::Accepted => ClaimStatus::Accepted,
                Decision::Rejected => ClaimStatus::Rejected,
            };
        }
        Ok(decision)
    }
}

fn within_window(e: &EvidenceItem, now: &str) -> bool {
    let Some(captured) = parse_iso_to_seconds(&e.captured_at) else {
        return false;
    };
    let Some(n) = parse_iso_to_seconds(now) else {
        return true;
    };
    n.abs_diff(captured) <= e.freshness_window_seconds
}
