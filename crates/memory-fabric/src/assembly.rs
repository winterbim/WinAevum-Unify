//! Context assembly — retrieval ∩ epistemic eligibility ∩ capability binding.
//!
//! Assembled context for an intended capability only includes memories that
//! can participate in authorization.

use aevum_evidence_graph::{may_authorize, EdgeKind, FirewallVerdict};
use serde::{Deserialize, Serialize};

use crate::backend::{MemoryBackend, MemoryError, MemoryHit, MemorySource};

#[derive(Debug, Clone)]
pub struct AssemblyRequest {
    pub query: String,
    pub as_of: Option<String>,
    /// If set, prefer / require facts that authorize this capability.
    pub intended_capability: Option<String>,
    pub limit: usize,
    /// Include remote recall hits (as non-authorizing).
    pub include_remote: bool,
    /// Optional mission scope — when set, only that mission's facts assemble.
    pub mission_id: Option<String>,
}

impl Default for AssemblyRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            as_of: None,
            intended_capability: None,
            limit: 10,
            include_remote: false,
            mission_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledContext {
    pub query: String,
    pub as_of: String,
    pub intended_capability: Option<String>,
    pub hits: Vec<RankedHit>,
    pub authorizing_fact_ids: Vec<String>,
    pub blocked_remote_count: usize,
    pub assembly_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedHit {
    pub hit: MemoryHit,
    pub trust_weight: f64,
    pub final_score: f64,
    pub reason: String,
}

/// Assemble trust-filtered context from a backend.
pub fn assemble(
    backend: &dyn MemoryBackend,
    req: &AssemblyRequest,
) -> Result<AssembledContext, MemoryError> {
    let as_of = req
        .as_of
        .clone()
        .unwrap_or_else(|| "2099-01-01T00:00:00Z".into());
    let native_hits = backend.search(&req.query, Some(&as_of), req.limit.saturating_mul(3))?;

    let g = backend.graph();
    let mut ranked: Vec<RankedHit> = Vec::new();
    let mut authorizing = Vec::new();

    for h in native_hits {
        let fact = match g.fact(&h.fact_id) {
            Some(f) => f,
            None => continue,
        };
        if let Some(mid) = req.mission_id.as_deref() {
            if fact.mission_id != mid {
                continue;
            }
        }
        let coverage = g.provenance_coverage(&h.fact_id).unwrap_or(0.0);
        let trust = match fact.epistemic {
            aevum_evidence_graph::EpistemicKind::Fact => 1.0 * (0.5 + 0.5 * coverage),
            aevum_evidence_graph::EpistemicKind::Inference => 0.35,
            aevum_evidence_graph::EpistemicKind::Hypothesis => 0.15,
            aevum_evidence_graph::EpistemicKind::Recommendation => 0.25,
            aevum_evidence_graph::EpistemicKind::Unknown => 0.1,
        };
        let capability_boost = if let Some(cap) = &req.intended_capability {
            let action = format!("action:{cap}");
            if fact.target_node_id == action && matches!(fact.kind, EdgeKind::Authorizes) {
                if may_authorize(fact) == FirewallVerdict::Allow {
                    authorizing.push(fact.id.clone());
                    1.25
                } else {
                    0.0
                }
            } else if matches!(fact.kind, EdgeKind::Authorizes) {
                0.4
            } else {
                0.7
            }
        } else {
            1.0
        };
        if capability_boost == 0.0 {
            continue;
        }
        let final_score = h.score * trust * capability_boost;
        let may_auth = matches!(fact.kind, EdgeKind::Authorizes)
            && may_authorize(fact) == FirewallVerdict::Allow;
        ranked.push(RankedHit {
            hit: MemoryHit {
                id: h.fact_id.clone(),
                fact: h.fact.clone(),
                name: h.name.clone(),
                score: h.score,
                source: MemorySource::Native,
                may_authorize: may_auth,
                provenance_coverage: coverage,
            },
            trust_weight: trust,
            final_score,
            reason: if may_auth {
                "authorizing_fact".into()
            } else if matches!(fact.epistemic, aevum_evidence_graph::EpistemicKind::Fact) {
                "attested_fact".into()
            } else {
                format!("epistemic={:?}", fact.epistemic)
            },
        });
    }

    let mut blocked_remote = 0usize;
    if req.include_remote {
        match backend.remote_search(&req.query, req.limit) {
            Ok(remotes) => {
                for r in remotes {
                    blocked_remote += 1;
                    ranked.push(RankedHit {
                        hit: MemoryHit {
                            id: format!("remote:{}", r.uuid),
                            fact: r.fact,
                            name: r.name,
                            score: 0.5,
                            source: MemorySource::Remote,
                            may_authorize: false,
                            provenance_coverage: 0.0,
                        },
                        trust_weight: 0.05,
                        final_score: 0.025,
                        reason: "remote_unpromoted".into(),
                    });
                }
            }
            Err(MemoryError::NotConfigured(_)) => {}
            Err(e) => {
                if req.include_remote {
                    return Err(e);
                }
            }
        }
    }

    ranked.sort_by(|a, b| {
        b.final_score
            .partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.truncate(req.limit);
    let assembly_score =
        ranked.iter().map(|r| r.final_score).sum::<f64>() / (ranked.len().max(1) as f64);

    Ok(AssembledContext {
        query: req.query.clone(),
        as_of,
        intended_capability: req.intended_capability.clone(),
        hits: ranked,
        authorizing_fact_ids: authorizing,
        blocked_remote_count: blocked_remote,
        assembly_score,
    })
}
