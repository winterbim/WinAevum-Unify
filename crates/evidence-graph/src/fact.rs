//! Bi-temporal facts for the Decision & Evidence Graph.
//!
//! Bi-temporal fields:
//! - `valid_at` / `invalid_at` — event-time validity window
//! - `created_at` / `expired_at` — transaction-time bookkeeping
//!
//! Aevum additions:
//! - `episode_ids` provenance (must be non-empty)
//! - `epistemic` kind with authorization firewall
//! - content-addressed `fact_digest` for ledger binding

use serde::{Deserialize, Serialize};

use crate::ontology::{EdgeKind, EpistemicKind};
use crate::time::is_valid_at;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: crate::ontology::NodeKind,
    pub name: String,
    pub summary: String,
    pub mission_id: String,
    pub group_id: String,
    pub created_at: String,
    /// Optional embedding placeholder for future hybrid semantic search (port).
    pub embedding: Option<Vec<f32>>,
}

/// A typed, bi-temporal edge between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub id: String,
    pub kind: EdgeKind,
    pub source_node_id: String,
    pub target_node_id: String,
    pub name: String,
    pub fact: String,
    pub epistemic: EpistemicKind,
    /// Episodes that produced this fact (provenance).
    pub episode_ids: Vec<String>,
    /// Event time: when the fact became true.
    pub valid_at: String,
    /// Event time: when the fact stopped being true (None = still valid).
    pub invalid_at: Option<String>,
    /// Transaction time: graph write.
    pub created_at: String,
    /// Transaction time: soft-delete / supersession bookkeeping.
    pub expired_at: Option<String>,
    /// Content-addressed digest for Trust Ledger binding.
    pub fact_digest: Option<String>,
    pub group_id: String,
    pub mission_id: String,
}

impl Fact {
    /// Event-time validity: `point ∈ [valid_at, invalid_at)`.
    /// Ignores transaction-time `expired_at` so historical as_of queries work
    /// after invalidate-don't-delete (bi-temporal model).
    pub fn is_active_at(&self, point: &str) -> bool {
        is_valid_at(&self.valid_at, self.invalid_at.as_deref(), point)
    }

    /// Currently believed true (not soft-deleted, not invalidated).
    pub fn is_current(&self) -> bool {
        self.expired_at.is_none() && self.invalid_at.is_none()
    }

    /// Invalidate (do not delete) — contradiction handling.
    pub fn invalidate(&mut self, at: &str, expired_at: &str) {
        self.invalid_at = Some(at.to_string());
        self.expired_at = Some(expired_at.to_string());
    }
}
