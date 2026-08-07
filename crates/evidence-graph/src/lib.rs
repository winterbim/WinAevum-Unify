#![allow(missing_docs)]
//! Aevum Unify — Decision & Evidence Graph (M6+).
//!
//! Two layers:
//! 1. [`store::EvidenceStore`] — original claim↔evidence freshness/challenge API.
//! 2. [`temporal::TemporalGraph`] — bi-temporal Decision & Evidence Graph
//!    with episodes, fact invalidation, epistemic firewall, and hybrid search.
//!
//! Doctrine: blueprint §11, D01, D23. Native bi-temporal model, not a vendored fork.

pub mod contradiction;
pub mod episode;
pub mod epistemic;
pub mod fact;
pub mod ontology;
pub mod query;
pub mod store;
pub mod temporal;
pub mod time;

pub use contradiction::{
    apply_refutation, detect_contradictions, resolve_parallel_conflicts, Contradiction,
};

pub use episode::{Episode, EpisodeSource};
pub use epistemic::{
    assert_authorizes_edge, llm_output_is_primary_evidence, may_authorize,
    require_evidence_for_critical, FirewallVerdict,
};
pub use fact::{Fact, GraphNode};
pub use ontology::{EdgeKind, EpistemicKind, NodeKind};
pub use query::{
    bm25_score, export_neighborhood, hybrid_search, local_cross_encoder_score, rrf_fuse, tokenize,
    NeighborhoodExport, SearchHit, SearchRecipe,
};
pub use store::{
    Challenge, Claim, ClaimStatus, Decision, EvidenceItem, EvidenceKind, EvidenceStatus,
    EvidenceStore, FreshnessPolicy, GraphError,
};
pub use temporal::{
    fact_window_contains, relate_fact, seed_claim_node, seed_entity, GraphEvent, GraphSnapshot,
    TemporalError, TemporalGraph,
};
