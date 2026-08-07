//! Episodes — raw provenance stream.
//!
//! Every derived fact/claim MUST trace back to at least one episode.
//! An episode in Aevum is content-addressed and may carry
//! an integrity digest so LLM text alone cannot become primary evidence.

use serde::{Deserialize, Serialize};

use crate::ontology::NodeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeSource {
    /// Human or agent message ("actor: content").
    Message,
    /// Structured JSON observation (tool result, policy decision, etc.).
    Json,
    /// Plain text document / log excerpt.
    Text,
    /// Explicit fact triple supplied by a trusted producer.
    FactTriple,
    /// Cryptographically attested artefact (digest required).
    Attested,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub mission_id: String,
    pub group_id: String,
    pub source: EpisodeSource,
    /// Raw payload (text or JSON string). Never treated as primary Evidence alone
    /// unless `content_digest` is set and `source == Attested`.
    pub content: String,
    /// Optional SHA-256 of content bytes (`sha256:…`).
    pub content_digest: Option<String>,
    /// Event time — when the observation occurred in the world.
    pub valid_at: String,
    /// Transaction time — when the episode entered the graph.
    pub created_at: String,
    pub actor_id: Option<String>,
}

impl Episode {
    pub fn node_kind(&self) -> NodeKind {
        NodeKind::Episode
    }

    /// Primary evidence path requires attested content with digest (D01 / §11.5).
    pub fn is_primary_evidence_eligible(&self) -> bool {
        matches!(self.source, EpisodeSource::Attested) && self.content_digest.is_some()
    }
}
