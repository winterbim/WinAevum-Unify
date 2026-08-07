//! Embedding port — semantic hybrid search without making gates depend on LLM vendors.
//!
//! - [`HashingEmbedder`]: deterministic local vectors (always available, offline).
//! - [`OpenAiCompatibleEmbedder`]: real HTTP to OpenAI-compatible `/v1/embeddings`
//!   when `EMBEDDING_URL` + `EMBEDDING_API_KEY` (or `OPENAI_API_KEY`) are set.
//!
//! Gates never require embeddings. They improve recall only.

use aevum_evidence_graph::{hybrid_search, SearchHit, SearchRecipe, TemporalGraph};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::backend::MemoryError;

#[derive(Debug, Error)]
pub enum EmbedError {
    #[error("http: {0}")]
    Http(String),
    #[error("not configured: {0}")]
    NotConfigured(String),
}

impl From<EmbedError> for MemoryError {
    fn from(e: EmbedError) -> Self {
        MemoryError::Backend(e.to_string())
    }
}

pub trait Embedder: Send + Sync {
    fn name(&self) -> &'static str;
    fn dimensions(&self) -> usize;
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;
}

/// Feature-hashing embedder — real math, no network, stable across runs.
pub struct HashingEmbedder {
    dims: usize,
}

impl Default for HashingEmbedder {
    fn default() -> Self {
        Self { dims: 256 }
    }
}

impl HashingEmbedder {
    pub fn new(dims: usize) -> Self {
        Self { dims: dims.max(32) }
    }

    fn hash_token(&self, token: &str) -> (usize, f32) {
        let mut h = Sha256::new();
        h.update(token.as_bytes());
        let bytes = h.finalize();
        let idx = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize % self.dims;
        let sign = if bytes[4] & 1 == 0 { 1.0 } else { -1.0 };
        (idx, sign)
    }
}

impl Embedder for HashingEmbedder {
    fn name(&self) -> &'static str {
        "hashing"
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        let mut out = Vec::with_capacity(texts.len());
        for text in texts {
            let mut v = vec![0.0f32; self.dims];
            for tok in text.to_lowercase().split(|c: char| !c.is_alphanumeric()) {
                if tok.len() < 2 {
                    continue;
                }
                let (i, s) = self.hash_token(tok);
                v[i] += s;
            }
            // Character bigrams — improves recall without network embeddings.
            let chars: Vec<char> = text
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect();
            for w in chars.windows(2) {
                let bigram: String = w.iter().collect();
                let (i, s) = self.hash_token(&bigram);
                v[i] += s * 0.5;
            }
            let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in &mut v {
                    *x /= norm;
                }
            }
            out.push(v);
        }
        Ok(out)
    }
}

/// OpenAI-compatible embeddings HTTP client (real requests — no stubs).
pub struct OpenAiCompatibleEmbedder {
    url: String,
    api_key: String,
    model: String,
    dims: usize,
    agent: ureq::Agent,
}

impl OpenAiCompatibleEmbedder {
    pub fn from_env() -> Result<Self, EmbedError> {
        let url = std::env::var("EMBEDDING_URL")
            .or_else(|_| {
                std::env::var("OPENAI_BASE_URL")
                    .map(|b| format!("{}/v1/embeddings", b.trim_end_matches('/')))
            })
            .unwrap_or_else(|_| "https://api.openai.com/v1/embeddings".into());
        let api_key = std::env::var("EMBEDDING_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .map_err(|_| {
                EmbedError::NotConfigured(
                    "EMBEDDING_API_KEY or OPENAI_API_KEY required for OpenAI-compatible embedder"
                        .into(),
                )
            })?;
        let model =
            std::env::var("EMBEDDING_MODEL").unwrap_or_else(|_| "text-embedding-3-small".into());
        Ok(Self {
            url,
            api_key,
            model,
            dims: 1536,
            agent: ureq::AgentBuilder::new()
                .timeout(std::time::Duration::from_secs(30))
                .build(),
        })
    }
}

impl Embedder for OpenAiCompatibleEmbedder {
    fn name(&self) -> &'static str {
        "openai-compatible"
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        #[derive(Deserialize)]
        struct Resp {
            data: Vec<Item>,
        }
        #[derive(Deserialize)]
        struct Item {
            embedding: Vec<f32>,
            index: usize,
        }
        let body = serde_json::json!({
            "model": self.model,
            "input": texts,
        });
        let resp = self
            .agent
            .post(&self.url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(body)
            .map_err(|e| EmbedError::Http(e.to_string()))?;
        let parsed: Resp = resp
            .into_json()
            .map_err(|e| EmbedError::Http(format!("json: {e}")))?;
        let mut ordered = vec![Vec::new(); texts.len()];
        for item in parsed.data {
            if item.index < ordered.len() {
                ordered[item.index] = item.embedding;
            }
        }
        Ok(ordered)
    }
}

/// Prefer OpenAI-compatible when configured; else hashing (always works offline).
pub fn default_embedder() -> Box<dyn Embedder> {
    match OpenAiCompatibleEmbedder::from_env() {
        Ok(e) => Box::new(e),
        Err(_) => Box::new(HashingEmbedder::default()),
    }
}

/// Embed query + attach vectors onto graph nodes missing embeddings (in-place).
pub fn ensure_node_embeddings(
    g: &mut TemporalGraph,
    embedder: &dyn Embedder,
) -> Result<usize, EmbedError> {
    let mut to_embed: Vec<(String, String)> = Vec::new();
    let snap = g.to_snapshot();
    for n in &snap.nodes {
        if n.embedding.is_none() {
            to_embed.push((n.id.clone(), format!("{} {}", n.name, n.summary)));
        }
    }
    if to_embed.is_empty() {
        return Ok(0);
    }
    let texts: Vec<String> = to_embed.iter().map(|(_, t)| t.clone()).collect();
    let vectors = embedder.embed(&texts)?;
    let mut updated = 0usize;
    for ((id, _), vec) in to_embed.into_iter().zip(vectors.into_iter()) {
        if let Some(mut node) = g.node(&id).cloned() {
            node.embedding = Some(vec);
            g.upsert_node(node);
            updated += 1;
        }
    }
    Ok(updated)
}

/// Hybrid search with query embedding (node vectors used when already present).
pub fn semantic_hybrid_search(
    g: &TemporalGraph,
    query: &str,
    as_of: Option<&str>,
    limit: usize,
    embedder: &dyn Embedder,
) -> Result<Vec<SearchHit>, EmbedError> {
    semantic_hybrid_search_scoped(g, query, as_of, limit, embedder, None)
}

pub fn semantic_hybrid_search_scoped(
    g: &TemporalGraph,
    query: &str,
    as_of: Option<&str>,
    limit: usize,
    embedder: &dyn Embedder,
    scope: Option<&crate::scope::TenantScope>,
) -> Result<Vec<SearchHit>, EmbedError> {
    let q_vec = embedder
        .embed(&[query.to_string()])?
        .into_iter()
        .next()
        .unwrap_or_default();
    let mut recipe = SearchRecipe::new(query).limit(limit).with_embedding(q_vec);
    if let Some(t) = as_of {
        recipe = recipe.as_of(t);
    }
    if let Some(s) = scope {
        recipe = recipe.scoped(&s.mission_id, s.group_id());
    }
    Ok(hybrid_search(g, &recipe))
}
