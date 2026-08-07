//! Temporal Decision & Evidence Graph — Aevum doctrine.
//!
//! Core ideas:
//! - Episodes as provenance stream
//! - Bi-temporal facts (`valid_at` / `invalid_at` + transaction time)
//! - Invalidate-don't-delete on contradiction
//! - Incremental ingest without full recompute
//! - Hybrid retrieval (see [`crate::query`])
//!
//! Aevum-native extensions:
//! - Blueprint §11 ontology
//! - Epistemic firewall (hypothesis cannot authorize)
//! - Content-addressed digests for Trust Ledger
//! - Local-first in-memory + event journal (no Neo4j required)

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::episode::Episode;
use crate::epistemic::{may_authorize, FirewallVerdict};
use crate::fact::{Fact, GraphNode};
use crate::ontology::{EdgeKind, EpistemicKind, NodeKind};
use crate::time::is_valid_at;

#[derive(Debug, Error, PartialEq)]
pub enum TemporalError {
    #[error("unknown node: {0}")]
    UnknownNode(String),
    #[error("unknown episode: {0}")]
    UnknownEpisode(String),
    #[error("unknown fact: {0}")]
    UnknownFact(String),
    #[error("rejected: {0}")]
    Rejected(String),
    #[error("duplicate id: {0}")]
    Duplicate(String),
}

/// Append-only event for reconstructibility (M6 exit criterion).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GraphEvent {
    EpisodeAdded { id: String },
    NodeUpserted { id: String },
    FactAsserted { id: String },
    FactInvalidated { id: String, at: String },
}

/// Durable snapshot — local-first persistence (no Neo4j).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    pub version: String,
    pub episodes: Vec<Episode>,
    pub nodes: Vec<GraphNode>,
    pub facts: Vec<Fact>,
    pub events: Vec<GraphEvent>,
}

impl Default for GraphSnapshot {
    fn default() -> Self {
        Self {
            version: "aevum.temporal-graph/v1".into(),
            episodes: Vec::new(),
            nodes: Vec::new(),
            facts: Vec::new(),
            events: Vec::new(),
        }
    }
}

#[derive(Default)]
pub struct TemporalGraph {
    episodes: HashMap<String, Episode>,
    nodes: HashMap<String, GraphNode>,
    facts: HashMap<String, Fact>,
    /// Adjacency: node_id → set of fact_ids
    adjacency: HashMap<String, HashSet<String>>,
    events: Vec<GraphEvent>,
}

impl TemporalGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn event_log(&self) -> &[GraphEvent] {
        &self.events
    }

    pub fn episode_count(&self) -> usize {
        self.episodes.len()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn fact_count(&self) -> usize {
        self.facts.len()
    }

    pub fn to_snapshot(&self) -> GraphSnapshot {
        GraphSnapshot {
            version: "aevum.temporal-graph/v1".into(),
            episodes: self.episodes.values().cloned().collect(),
            nodes: self.nodes.values().cloned().collect(),
            facts: self.facts.values().cloned().collect(),
            events: self.events.clone(),
        }
    }

    pub fn from_snapshot(snap: GraphSnapshot) -> Result<Self, TemporalError> {
        let mut g = Self::new();
        // Load without re-emitting side effects: insert directly then rebuild events/adjacency.
        for ep in snap.episodes {
            g.episodes.insert(ep.id.clone(), ep);
        }
        for n in snap.nodes {
            g.nodes.insert(n.id.clone(), n);
        }
        for f in snap.facts {
            g.link_adjacency(&f);
            g.facts.insert(f.id.clone(), f);
        }
        g.events = snap.events;
        Ok(g)
    }

    /// True if an active `authorizes` fact targets `action:{capability}` and passes firewall.
    pub fn capability_authorized(&self, capability: &str, as_of: &str) -> bool {
        let action_id = format!("action:{capability}");
        for f in self.facts_as_of(Some(as_of)) {
            if f.target_node_id == action_id
                && matches!(f.kind, EdgeKind::Authorizes)
                && may_authorize(f) == FirewallVerdict::Allow
            {
                return true;
            }
        }
        false
    }

    /// Bootstrap a mission graph: attested constitution episode + authorizes for capabilities.
    pub fn seed_for_mission(
        mission_id: &str,
        constitution_src: &str,
        constitution_digest: &str,
        capabilities: &[&str],
        now: &str,
    ) -> Result<Self, TemporalError> {
        let mut g = Self::new();
        let group = format!("mission:{mission_id}");
        let ep = Episode {
            id: "ep_constitution".into(),
            mission_id: mission_id.to_string(),
            group_id: group.clone(),
            source: crate::episode::EpisodeSource::Attested,
            content: constitution_src.to_string(),
            content_digest: Some(constitution_digest.to_string()),
            valid_at: now.to_string(),
            created_at: now.to_string(),
            actor_id: Some("spiffe://local.aevum/ledger-authority".into()),
        };
        g.add_episode(ep)?;
        g.upsert_node(GraphNode {
            id: "claim:constitution".into(),
            kind: NodeKind::Claim,
            name: "Mission constitution is binding".into(),
            summary: format!("digest {constitution_digest}"),
            mission_id: mission_id.to_string(),
            group_id: group.clone(),
            created_at: now.to_string(),
            embedding: None,
        });
        for cap in capabilities {
            let action_id = format!("action:{cap}");
            g.upsert_node(GraphNode {
                id: action_id.clone(),
                kind: NodeKind::ActionIntent,
                name: (*cap).to_string(),
                summary: format!("capability {cap}"),
                mission_id: mission_id.to_string(),
                group_id: group.clone(),
                created_at: now.to_string(),
                embedding: None,
            });
            let digest = format!("sha256:auth:{}:{}:{}", mission_id, cap, constitution_digest);
            let fact = Fact {
                id: format!("fact:auth:{cap}"),
                kind: EdgeKind::Authorizes,
                source_node_id: "claim:constitution".into(),
                target_node_id: action_id,
                name: "AUTHORIZES".into(),
                fact: format!("constitution authorizes {cap}"),
                epistemic: EpistemicKind::Fact,
                episode_ids: vec!["ep_constitution".into()],
                valid_at: now.to_string(),
                invalid_at: None,
                created_at: now.to_string(),
                expired_at: None,
                fact_digest: Some(digest),
                group_id: group.clone(),
                mission_id: mission_id.to_string(),
            };
            g.assert_fact(fact)?;
        }
        Ok(g)
    }

    // ── Episodes ──────────────────────────────────────────────────────────

    pub fn add_episode(&mut self, ep: Episode) -> Result<(), TemporalError> {
        if self.episodes.contains_key(&ep.id) {
            return Err(TemporalError::Duplicate(ep.id));
        }
        let id = ep.id.clone();
        self.episodes.insert(id.clone(), ep);
        self.events.push(GraphEvent::EpisodeAdded { id });
        Ok(())
    }

    pub fn episode(&self, id: &str) -> Option<&Episode> {
        self.episodes.get(id)
    }

    pub fn episodes(&self) -> impl Iterator<Item = &Episode> {
        self.episodes.values()
    }

    // ── Nodes ─────────────────────────────────────────────────────────────

    pub fn upsert_node(&mut self, node: GraphNode) {
        let id = node.id.clone();
        self.nodes.insert(id.clone(), node);
        self.events.push(GraphEvent::NodeUpserted { id });
    }

    pub fn node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    pub fn neighbor_ids(&self, node_id: &str) -> HashSet<String> {
        let mut out = HashSet::new();
        if let Some(fids) = self.adjacency.get(node_id) {
            for fid in fids {
                if let Some(f) = self.facts.get(fid) {
                    out.insert(f.source_node_id.clone());
                    out.insert(f.target_node_id.clone());
                }
            }
        }
        out.remove(node_id);
        out
    }

    // ── Facts ─────────────────────────────────────────────────────────────

    /// Assert a bi-temporal fact. Requires non-empty episode provenance.
    /// Authorizes edges must pass the epistemic firewall.
    pub fn assert_fact(&mut self, fact: Fact) -> Result<(), TemporalError> {
        if self.facts.contains_key(&fact.id) {
            return Err(TemporalError::Duplicate(fact.id));
        }
        if fact.episode_ids.is_empty() {
            return Err(TemporalError::Rejected(
                "fact must cite at least one episode (provenance)".into(),
            ));
        }
        for eid in &fact.episode_ids {
            if !self.episodes.contains_key(eid) {
                return Err(TemporalError::UnknownEpisode(eid.clone()));
            }
        }
        if !self.nodes.contains_key(&fact.source_node_id) {
            return Err(TemporalError::UnknownNode(fact.source_node_id.clone()));
        }
        if !self.nodes.contains_key(&fact.target_node_id) {
            return Err(TemporalError::UnknownNode(fact.target_node_id.clone()));
        }
        if matches!(fact.kind, EdgeKind::Authorizes) {
            match may_authorize(&fact) {
                FirewallVerdict::Allow => {}
                FirewallVerdict::Deny(reason) => {
                    return Err(TemporalError::Rejected(reason.to_string()));
                }
            }
        }
        // Auto-invalidate conflicting active facts with same (src, name, tgt) if new fact
        // is RelatesTo / Supports and explicitly supersedes (same name).
        if matches!(fact.kind, EdgeKind::RelatesTo | EdgeKind::Supports) {
            self.invalidate_conflicts(&fact)?;
        }

        let id = fact.id.clone();
        self.link_adjacency(&fact);
        self.facts.insert(id.clone(), fact);
        self.events.push(GraphEvent::FactAsserted { id });
        Ok(())
    }

    fn link_adjacency(&mut self, fact: &Fact) {
        self.adjacency
            .entry(fact.source_node_id.clone())
            .or_default()
            .insert(fact.id.clone());
        self.adjacency
            .entry(fact.target_node_id.clone())
            .or_default()
            .insert(fact.id.clone());
    }

    fn invalidate_conflicts(&mut self, incoming: &Fact) -> Result<(), TemporalError> {
        let now = incoming.created_at.clone();
        let at = incoming.valid_at.clone();
        let mut to_invalidate: Vec<String> = Vec::new();
        for f in self.facts.values() {
            if f.id == incoming.id {
                continue;
            }
            if f.expired_at.is_some() {
                continue;
            }
            let same_triple = f.source_node_id == incoming.source_node_id
                && f.target_node_id == incoming.target_node_id
                && f.name == incoming.name;
            if same_triple && f.is_active_at(&incoming.valid_at) {
                // Contradiction: old fact still active when new one starts
                to_invalidate.push(f.id.clone());
            }
        }
        for id in to_invalidate {
            self.invalidate_fact(&id, &at, &now)?;
        }
        Ok(())
    }

    /// Invalidate a fact (preserve history) — bi-temporal contradiction model.
    pub fn invalidate_fact(
        &mut self,
        id: &str,
        invalid_at: &str,
        expired_at: &str,
    ) -> Result<(), TemporalError> {
        let fact = self
            .facts
            .get_mut(id)
            .ok_or_else(|| TemporalError::UnknownFact(id.to_string()))?;
        fact.invalidate(invalid_at, expired_at);
        self.events.push(GraphEvent::FactInvalidated {
            id: id.to_string(),
            at: invalid_at.to_string(),
        });
        Ok(())
    }

    pub fn fact(&self, id: &str) -> Option<&Fact> {
        self.facts.get(id)
    }

    /// Facts valid at `as_of` (event time). `None` → currently believed facts.
    pub fn facts_as_of(&self, as_of: Option<&str>) -> Vec<&Fact> {
        self.facts
            .values()
            .filter(|f| match as_of {
                Some(t) => f.is_active_at(t),
                None => f.is_current(),
            })
            .collect()
    }

    /// Point-in-time query: is edge active?
    pub fn is_fact_valid_at(&self, id: &str, point: &str) -> Result<bool, TemporalError> {
        let f = self
            .facts
            .get(id)
            .ok_or_else(|| TemporalError::UnknownFact(id.to_string()))?;
        Ok(f.is_active_at(point))
    }

    /// Can any active `authorizes` fact from `from` to `to` pass the firewall at `as_of`?
    pub fn authorization_allowed(
        &self,
        from: &str,
        to: &str,
        as_of: &str,
    ) -> Result<FirewallVerdict, TemporalError> {
        for f in self.facts_as_of(Some(as_of)) {
            if f.source_node_id == from
                && f.target_node_id == to
                && matches!(f.kind, EdgeKind::Authorizes)
            {
                return Ok(may_authorize(f));
            }
        }
        Ok(FirewallVerdict::Deny("no active authorizes edge"))
    }

    /// Coverage: fraction of required episode digests present for a claim node.
    pub fn provenance_coverage(&self, fact_id: &str) -> Result<f64, TemporalError> {
        let f = self
            .facts
            .get(fact_id)
            .ok_or_else(|| TemporalError::UnknownFact(fact_id.to_string()))?;
        if f.episode_ids.is_empty() {
            return Ok(0.0);
        }
        let attested = f
            .episode_ids
            .iter()
            .filter(|id| {
                self.episodes
                    .get(id.as_str())
                    .map(|e| e.is_primary_evidence_eligible())
                    .unwrap_or(false)
            })
            .count();
        Ok(attested as f64 / f.episode_ids.len() as f64)
    }

    /// Rebuild adjacency from facts (after journal replay).
    pub fn reindex(&mut self) {
        self.adjacency.clear();
        let facts: Vec<Fact> = self.facts.values().cloned().collect();
        for f in facts {
            self.link_adjacency(&f);
        }
    }
}

/// Convenience builder for a minimal mission subgraph.
pub fn seed_entity(id: &str, name: &str, mission_id: &str, group_id: &str, at: &str) -> GraphNode {
    GraphNode {
        id: id.to_string(),
        kind: NodeKind::Entity,
        name: name.to_string(),
        summary: name.to_string(),
        mission_id: mission_id.to_string(),
        group_id: group_id.to_string(),
        created_at: at.to_string(),
        embedding: None,
    }
}

pub fn seed_claim_node(
    id: &str,
    statement: &str,
    mission_id: &str,
    group_id: &str,
    at: &str,
) -> GraphNode {
    GraphNode {
        id: id.to_string(),
        kind: NodeKind::Claim,
        name: statement.to_string(),
        summary: statement.to_string(),
        mission_id: mission_id.to_string(),
        group_id: group_id.to_string(),
        created_at: at.to_string(),
        embedding: None,
    }
}

/// Helper to build a RelatesTo fact with provenance.
#[allow(clippy::too_many_arguments)]
pub fn relate_fact(
    id: &str,
    src: &str,
    tgt: &str,
    name: &str,
    fact_text: &str,
    episode_id: &str,
    valid_at: &str,
    created_at: &str,
    mission_id: &str,
    group_id: &str,
    epistemic: EpistemicKind,
) -> Fact {
    Fact {
        id: id.to_string(),
        kind: EdgeKind::RelatesTo,
        source_node_id: src.to_string(),
        target_node_id: tgt.to_string(),
        name: name.to_string(),
        fact: fact_text.to_string(),
        epistemic,
        episode_ids: vec![episode_id.to_string()],
        valid_at: valid_at.to_string(),
        invalid_at: None,
        created_at: created_at.to_string(),
        expired_at: None,
        fact_digest: None,
        group_id: group_id.to_string(),
        mission_id: mission_id.to_string(),
    }
}

/// Check whether a fact's event-time window contains `point` (re-export helper).
pub fn fact_window_contains(valid_at: &str, invalid_at: Option<&str>, point: &str) -> bool {
    is_valid_at(valid_at, invalid_at, point)
}
