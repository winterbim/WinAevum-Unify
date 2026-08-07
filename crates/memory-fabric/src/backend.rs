//! Memory backend contract — native TemporalGraph persistence + search.
//!
//! Remote/untrusted memories never become authorizing facts until promoted
//! through an attested episode (epistemic promotion protocol).

use aevum_evidence_graph::{GraphSnapshot, SearchHit, TemporalGraph};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("io: {0}")]
    Io(String),
    #[error("backend: {0}")]
    Backend(String),
    #[error("remote http: {0}")]
    RemoteHttp(String),
    #[error("promotion rejected: {0}")]
    Promotion(String),
    #[error("not configured: {0}")]
    NotConfigured(String),
}

/// A retrieved memory candidate before trust filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHit {
    pub id: String,
    pub fact: String,
    pub name: String,
    pub score: f64,
    pub source: MemorySource,
    /// True only after epistemic promotion + active authorizes eligibility.
    pub may_authorize: bool,
    pub provenance_coverage: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    Native,
    Remote,
    Promoted,
}

pub trait MemoryBackend: Send {
    fn name(&self) -> &'static str;
    fn load(&mut self) -> Result<(), MemoryError>;
    fn save(&self) -> Result<(), MemoryError>;
    fn graph(&self) -> &TemporalGraph;
    fn graph_mut(&mut self) -> &mut TemporalGraph;
    fn search(
        &self,
        query: &str,
        as_of: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>, MemoryError>;
    /// Optional remote recall plane. Empty on native/sqlite backends.
    fn remote_search(&self, query: &str, limit: usize) -> Result<Vec<RemoteFact>, MemoryError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFact {
    pub uuid: String,
    pub fact: String,
    pub name: String,
    pub valid_at: Option<String>,
    pub invalid_at: Option<String>,
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInfo {
    pub name: String,
    pub snapshot: GraphSnapshot,
}
