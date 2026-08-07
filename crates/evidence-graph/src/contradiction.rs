//! Contradiction engine — detect and resolve conflicting active facts (P2).
//!
//! Fully deterministic conflict handling (no LLM mediation):
//! same (source, target, kind) with incompatible names, or explicit Refutes /
//! ConflictsWith edges. No network, no LLM.

use serde::{Deserialize, Serialize};

use crate::ontology::EdgeKind;
use crate::temporal::{TemporalError, TemporalGraph};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub left_fact_id: String,
    pub right_fact_id: String,
    pub reason: String,
    pub as_of: String,
}

/// Find pairs of active facts that contradict each other at `as_of`.
pub fn detect_contradictions(g: &TemporalGraph, as_of: &str) -> Vec<Contradiction> {
    let facts = g.facts_as_of(Some(as_of));
    let mut out = Vec::new();

    // Explicit refutes / conflicts_with
    for f in &facts {
        if matches!(f.kind, EdgeKind::Refutes | EdgeKind::ConflictsWith) {
            out.push(Contradiction {
                left_fact_id: f.id.clone(),
                right_fact_id: f.target_node_id.clone(),
                reason: format!("{:?} edge active", f.kind),
                as_of: as_of.to_string(),
            });
        }
    }

    // Same endpoints + kind, different fact text / name → conflict
    for i in 0..facts.len() {
        for j in (i + 1)..facts.len() {
            let a = facts[i];
            let b = facts[j];
            if a.source_node_id == b.source_node_id
                && a.target_node_id == b.target_node_id
                && a.kind == b.kind
                && a.name != b.name
                && !matches!(a.kind, EdgeKind::Mentions)
            {
                out.push(Contradiction {
                    left_fact_id: a.id.clone(),
                    right_fact_id: b.id.clone(),
                    reason: format!(
                        "parallel {:?} edges with distinct names: {} vs {}",
                        a.kind, a.name, b.name
                    ),
                    as_of: as_of.to_string(),
                });
            }
        }
    }
    out
}

/// Apply a Refutes edge: invalidate the target fact at `at`, keep history.
pub fn apply_refutation(
    g: &mut TemporalGraph,
    challenged_fact_id: &str,
    at: &str,
) -> Result<(), TemporalError> {
    g.invalidate_fact(challenged_fact_id, at, at)
}

/// Resolve detected triple conflicts by invalidating older facts (by created_at).
pub fn resolve_parallel_conflicts(
    g: &mut TemporalGraph,
    as_of: &str,
) -> Result<usize, TemporalError> {
    let conflicts = detect_contradictions(g, as_of);
    let mut resolved = 0usize;
    for c in conflicts {
        if c.reason.starts_with("parallel") {
            let left_created = g
                .fact(&c.left_fact_id)
                .map(|f| f.created_at.clone())
                .unwrap_or_default();
            let right_created = g
                .fact(&c.right_fact_id)
                .map(|f| f.created_at.clone())
                .unwrap_or_default();
            let older = if left_created <= right_created {
                c.left_fact_id
            } else {
                c.right_fact_id
            };
            if g.fact(&older)
                .map(|f| f.invalid_at.is_none())
                .unwrap_or(false)
            {
                g.invalidate_fact(&older, as_of, as_of)?;
                resolved += 1;
            }
        }
    }
    Ok(resolved)
}
