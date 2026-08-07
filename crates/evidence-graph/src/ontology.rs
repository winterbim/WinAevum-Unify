//! Typed nodes & relations from Aevum Unify blueprint §11.
//!
//! Prescribed ontology (typed entity/edge kinds)
//! but fixed to the Decision & Evidence Graph contract — not free-form agent memory.

use serde::{Deserialize, Serialize};

/// Blueprint §11.1 node kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Objective,
    Constraint,
    Claim,
    Evidence,
    Hypothesis,
    Option,
    Objection,
    Experiment,
    Decision,
    ActionIntent,
    Outcome,
    Lesson,
    /// Provenance stream entry (Episode).
    Episode,
    /// Named entity / resource referenced by facts.
    Entity,
}

/// Blueprint §11.2 relation kinds (+ temporal invalidation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Supports,
    Refutes,
    DependsOn,
    DerivedFrom,
    ConflictsWith,
    Tests,
    SelectedOver,
    Authorizes,
    Produced,
    VerifiedBy,
    InvalidatedBy,
    /// Entity → Entity factual relation.
    RelatesTo,
    /// Episode mentions a node.
    Mentions,
}

/// Epistemic kind for claims (D01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicKind {
    Fact,
    Inference,
    Hypothesis,
    Recommendation,
    Unknown,
}

impl EpistemicKind {
    /// D01 / §11.5: only facts (and verified inferences under policy) may authorize.
    pub fn may_authorize_action(self) -> bool {
        matches!(self, EpistemicKind::Fact)
    }
}
