//! Epistemic firewall — blueprint §11.5 + D01.
//!
//! Naive memory stores treat all extracted facts similarly for retrieval.
//! Aevum separates *what agents remember* from *what may authorize an effect*.

use crate::fact::Fact;
use crate::ontology::{EdgeKind, EpistemicKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirewallVerdict {
    Allow,
    Deny(&'static str),
}

/// Can this fact participate in an `authorizes` edge toward an ActionIntent?
pub fn may_authorize(fact: &Fact) -> FirewallVerdict {
    if !fact.epistemic.may_authorize_action() {
        return FirewallVerdict::Deny(
            "hypothesis/inference/recommendation cannot authorize an action (D01/§11.5)",
        );
    }
    if fact.episode_ids.is_empty() {
        return FirewallVerdict::Deny("fact without episode provenance cannot authorize");
    }
    if matches!(fact.kind, EdgeKind::Authorizes) && fact.fact_digest.is_none() {
        return FirewallVerdict::Deny("authorizes edge requires fact_digest for ledger binding");
    }
    FirewallVerdict::Allow
}

/// Reject LLM-only episodes as primary evidence for authorization chains.
pub fn llm_output_is_primary_evidence() -> FirewallVerdict {
    FirewallVerdict::Deny("LLM output is not primary evidence (§11.5)")
}

/// Critical claim without evidence → require_more_evidence.
pub fn require_evidence_for_critical(has_evidence: bool) -> FirewallVerdict {
    if has_evidence {
        FirewallVerdict::Allow
    } else {
        FirewallVerdict::Deny("critical claim without evidence: require_more_evidence")
    }
}

/// Helper: only Fact epistemic kinds may sit on Authorizes edges.
pub fn assert_authorizes_edge(epistemic: EpistemicKind) -> FirewallVerdict {
    if epistemic.may_authorize_action() {
        FirewallVerdict::Allow
    } else {
        FirewallVerdict::Deny("non-fact epistemic kind blocked from authorizes")
    }
}
