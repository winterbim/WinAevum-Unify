//! Hybrid retrieval — native BM25 + embedding cosine + graph distance + trust.
//!
//! Avoids recall errors by scoring only bi-temporally active facts
//! in-process (no FalkorDB as-of leaks). Embeddings are optional; gates never
//! require them. Trust weight down-ranks Hypothesis / Inference vs Fact.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::fact::{Fact, GraphNode};
use crate::ontology::EpistemicKind;
use crate::temporal::TemporalGraph;

const BM25_K1: f64 = 1.2;
const BM25_B: f64 = 0.75;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub fact_id: String,
    pub score: f64,
    pub fact: String,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct SearchRecipe {
    pub query: String,
    pub as_of: Option<String>,
    pub center_node_id: Option<String>,
    pub limit: usize,
    pub query_embedding: Option<Vec<f32>>,
    /// When set, only facts with this mission_id participate (tenant isolation).
    pub mission_id: Option<String>,
    /// When set, only facts with this group_id participate.
    pub group_id: Option<String>,
}

impl SearchRecipe {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            as_of: None,
            center_node_id: None,
            limit: 10,
            query_embedding: None,
            mission_id: None,
            group_id: None,
        }
    }

    pub fn as_of(mut self, t: impl Into<String>) -> Self {
        self.as_of = Some(t.into());
        self
    }

    pub fn center(mut self, node_id: impl Into<String>) -> Self {
        self.center_node_id = Some(node_id.into());
        self
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.limit = n;
        self
    }

    pub fn with_embedding(mut self, v: Vec<f32>) -> Self {
        self.query_embedding = Some(v);
        self
    }

    pub fn mission(mut self, mission_id: impl Into<String>) -> Self {
        self.mission_id = Some(mission_id.into());
        self
    }

    pub fn group(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = Some(group_id.into());
        self
    }

    pub fn scoped(mut self, mission_id: impl Into<String>, group_id: impl Into<String>) -> Self {
        self.mission_id = Some(mission_id.into());
        self.group_id = Some(group_id.into());
        self
    }
}

/// Public tokenize helper (FTS / extractors share the same rules).
pub fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 1)
        .map(str::to_string)
        .collect()
}

struct DocStats {
    tokens: Vec<String>,
    len: usize,
}

fn build_corpus(facts: &[&Fact]) -> (Vec<DocStats>, HashMap<String, usize>, f64) {
    let mut docs = Vec::with_capacity(facts.len());
    let mut df: HashMap<String, usize> = HashMap::new();
    let mut total_len = 0usize;
    for f in facts {
        let text = format!("{} {}", f.name, f.fact);
        let toks = tokenize(&text);
        let mut seen = std::collections::HashSet::new();
        for t in &toks {
            if seen.insert(t.clone()) {
                *df.entry(t.clone()).or_insert(0) += 1;
            }
        }
        total_len += toks.len();
        docs.push(DocStats {
            len: toks.len().max(1),
            tokens: toks,
        });
    }
    let avgdl = if docs.is_empty() {
        1.0
    } else {
        total_len as f64 / docs.len() as f64
    };
    (docs, df, avgdl.max(1.0))
}

fn idf(n_docs: usize, df: usize) -> f64 {
    let n = n_docs as f64;
    let d = df as f64;
    ((n - d + 0.5) / (d + 0.5) + 1.0).ln().max(0.0)
}

/// Okapi BM25 over an active as-of corpus.
pub fn bm25_score(
    query_tokens: &[String],
    doc_tokens: &[String],
    doc_len: usize,
    avgdl: f64,
    n_docs: usize,
    df: &HashMap<String, usize>,
) -> f64 {
    if query_tokens.is_empty() || doc_tokens.is_empty() {
        return 0.0;
    }
    let mut tf_map: HashMap<&str, usize> = HashMap::new();
    for t in doc_tokens {
        *tf_map.entry(t.as_str()).or_insert(0) += 1;
    }
    let mut score = 0.0;
    let mut seen_q = std::collections::HashSet::new();
    for q in query_tokens {
        if !seen_q.insert(q.as_str()) {
            continue;
        }
        let tf = *tf_map.get(q.as_str()).unwrap_or(&0) as f64;
        if tf == 0.0 {
            continue;
        }
        let dfi = *df.get(q).unwrap_or(&0);
        let idf_q = idf(n_docs, dfi);
        let denom = tf + BM25_K1 * (1.0 - BM25_B + BM25_B * (doc_len as f64 / avgdl));
        score += idf_q * (tf * (BM25_K1 + 1.0)) / denom;
    }
    score
}

fn cosine(a: &[f32], b: &[f32]) -> f64 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for i in 0..a.len() {
        let x = a[i] as f64;
        let y = b[i] as f64;
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

fn graph_boost(g: &TemporalGraph, fact: &Fact, center: Option<&str>) -> f64 {
    let Some(c) = center else {
        return 0.0;
    };
    if fact.source_node_id == c || fact.target_node_id == c {
        return 0.35;
    }
    let neighbors = g.neighbor_ids(c);
    if neighbors.contains(&fact.source_node_id) || neighbors.contains(&fact.target_node_id) {
        return 0.15;
    }
    0.0
}

fn trust_weight(kind: EpistemicKind) -> f64 {
    match kind {
        EpistemicKind::Fact => 1.0,
        EpistemicKind::Inference => 0.45,
        EpistemicKind::Recommendation => 0.35,
        EpistemicKind::Hypothesis => 0.15,
        EpistemicKind::Unknown => 0.1,
    }
}

fn embedding_score(g: &TemporalGraph, fact: &Fact, query_emb: Option<&[f32]>) -> f64 {
    let src = g
        .node(&fact.source_node_id)
        .and_then(|n| n.embedding.as_ref());
    let tgt = g
        .node(&fact.target_node_id)
        .and_then(|n| n.embedding.as_ref());
    if let Some(q) = query_emb {
        let mut best = 0.0f64;
        if let Some(a) = src {
            best = best.max(cosine(q, a));
        }
        if let Some(b) = tgt {
            best = best.max(cosine(q, b));
        }
        return best.max(0.0);
    }
    match (src, tgt) {
        (Some(a), Some(b)) => cosine(a, b).max(0.0) * 0.5,
        _ => 0.0,
    }
}

fn normalize_scores(raw: &[f64]) -> Vec<f64> {
    let max = raw.iter().cloned().fold(0.0_f64, f64::max);
    if max <= 0.0 {
        return raw.iter().map(|_| 0.0).collect();
    }
    raw.iter().map(|x| x / max).collect()
}

/// Reciprocal Rank Fusion across ranked lists (k=60 classic).
pub fn rrf_fuse(rank_lists: &[Vec<usize>], n_docs: usize, k: f64) -> Vec<f64> {
    let mut scores = vec![0.0; n_docs];
    for list in rank_lists {
        for (rank, &doc_i) in list.iter().enumerate() {
            if doc_i < n_docs {
                scores[doc_i] += 1.0 / (k + rank as f64 + 1.0);
            }
        }
    }
    scores
}

fn rank_indices_by(scores: &[f64]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..scores.len()).collect();
    idx.sort_by(|&a, &b| {
        scores[b]
            .partial_cmp(&scores[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // drop zero-score tails from ranking lists (keep all for RRF stability)
    idx
}

/// Local cross-encoder surrogate: Jaccard on query/doc tokens (offline, deterministic).
/// Not a neural CE — intentionally avoids LLM dependency while improving
/// query–document interaction beyond independent BM25/embed scores.
pub fn local_cross_encoder_score(query_tokens: &[String], doc_tokens: &[String]) -> f64 {
    if query_tokens.is_empty() || doc_tokens.is_empty() {
        return 0.0;
    }
    let q: std::collections::HashSet<&str> = query_tokens.iter().map(|s| s.as_str()).collect();
    let d: std::collections::HashSet<&str> = doc_tokens.iter().map(|s| s.as_str()).collect();
    let inter = q.intersection(&d).count() as f64;
    let union = q.union(&d).count() as f64;
    if union == 0.0 {
        return 0.0;
    }
    // Soften with coverage of query terms
    let coverage = inter / q.len() as f64;
    (inter / union) * 0.5 + coverage * 0.5
}

/// Hybrid search: BM25 + embed + graph + local CE, fused via RRF, then trust weight.
pub fn hybrid_search(g: &TemporalGraph, recipe: &SearchRecipe) -> Vec<SearchHit> {
    let q_tokens = tokenize(&recipe.query);
    let as_of = recipe.as_of.as_deref();
    let facts: Vec<&Fact> = g
        .facts_as_of(as_of)
        .into_iter()
        .filter(|f| {
            if let Some(m) = recipe.mission_id.as_deref() {
                if f.mission_id != m {
                    return false;
                }
            }
            if let Some(gid) = recipe.group_id.as_deref() {
                if f.group_id != gid {
                    return false;
                }
            }
            true
        })
        .collect();
    if facts.is_empty() {
        return vec![];
    }
    let (docs, df, avgdl) = build_corpus(&facts);
    let n_docs = facts.len();

    let mut bm25_raw = Vec::with_capacity(n_docs);
    let mut emb_raw = Vec::with_capacity(n_docs);
    let mut gb_raw = Vec::with_capacity(n_docs);
    let mut ce_raw = Vec::with_capacity(n_docs);
    let mut trust_raw = Vec::with_capacity(n_docs);

    for (i, fact) in facts.iter().enumerate() {
        bm25_raw.push(bm25_score(
            &q_tokens,
            &docs[i].tokens,
            docs[i].len,
            avgdl,
            n_docs,
            &df,
        ));
        emb_raw.push(embedding_score(g, fact, recipe.query_embedding.as_deref()));
        gb_raw.push(graph_boost(g, fact, recipe.center_node_id.as_deref()));
        ce_raw.push(local_cross_encoder_score(&q_tokens, &docs[i].tokens));
        trust_raw.push(trust_weight(fact.epistemic));
    }

    let lists = vec![
        rank_indices_by(&bm25_raw),
        rank_indices_by(&emb_raw),
        rank_indices_by(&ce_raw),
        rank_indices_by(&gb_raw),
    ];
    let rrf = rrf_fuse(&lists, n_docs, 60.0);
    let rrf_n = normalize_scores(&rrf);
    let ce_n = normalize_scores(&ce_raw);

    let mut hits: Vec<SearchHit> = Vec::new();
    for (i, fact) in facts.iter().enumerate() {
        // RRF primary + CE interaction boost, then epistemic trust
        let base = rrf_n[i] * 0.75 + ce_n[i] * 0.25;
        let score = base * trust_raw[i];
        if score > 0.0 || bm25_raw[i] > 0.0 || ce_raw[i] > 0.0 {
            hits.push(SearchHit {
                fact_id: fact.id.clone(),
                score: if score > 0.0 {
                    score
                } else {
                    bm25_raw[i].max(ce_raw[i]) * 0.01 * trust_raw[i]
                },
                fact: fact.fact.clone(),
                name: fact.name.clone(),
            });
        }
    }

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(if recipe.limit == 0 { 10 } else { recipe.limit });
    hits
}

/// Reconstruct a human-readable subgraph around a node at `as_of`.
pub fn export_neighborhood(g: &TemporalGraph, node_id: &str, as_of: &str) -> NeighborhoodExport {
    let node = g.node(node_id).cloned();
    let facts: Vec<Fact> = g
        .facts_as_of(Some(as_of))
        .into_iter()
        .filter(|f| f.source_node_id == node_id || f.target_node_id == node_id)
        .cloned()
        .collect();
    let mut related: Vec<GraphNode> = Vec::new();
    for f in &facts {
        for id in [&f.source_node_id, &f.target_node_id] {
            if id != node_id {
                if let Some(n) = g.node(id) {
                    if !related.iter().any(|x| x.id == n.id) {
                        related.push(n.clone());
                    }
                }
            }
        }
    }
    NeighborhoodExport {
        node,
        facts,
        related,
    }
}

#[derive(Debug, Clone)]
pub struct NeighborhoodExport {
    pub node: Option<GraphNode>,
    pub facts: Vec<Fact>,
    pub related: Vec<GraphNode>,
}

#[cfg(test)]
mod bm25_unit {
    use super::*;

    #[test]
    fn local_ce_prefers_overlapping_query() {
        let q = tokenize("rust toolchain");
        let a = tokenize("rust toolchain installed locally");
        let b = tokenize("python docker kubernetes");
        assert!(local_cross_encoder_score(&q, &a) > local_cross_encoder_score(&q, &b));
    }

    #[test]
    fn rrf_boosts_consensus_docs() {
        // doc0 ranks high in both lists → should beat doc1
        let lists = vec![vec![0usize, 1], vec![0usize, 2]];
        let s = rrf_fuse(&lists, 3, 60.0);
        assert!(s[0] > s[1]);
        assert!(s[0] > s[2]);
    }
}
