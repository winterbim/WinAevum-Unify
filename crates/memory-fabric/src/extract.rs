//! Deterministic episode extraction — 100% autonomous (no LLM, no remote memory service).
//!
//! `valid_at` for every derived fact is ALWAYS the episode's `valid_at`
//! (REFERENCE_TIME). Never wall-clock "today" — that failure class
//! mode we refuse to copy.

use aevum_evidence_graph::{
    relate_fact, seed_entity, Episode, EpisodeSource, EpistemicKind, TemporalError, TemporalGraph,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("json: {0}")]
    Json(String),
    #[error("graph: {0}")]
    Graph(#[from] TemporalError),
    #[error("rejected: {0}")]
    Rejected(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredFact {
    pub source: String,
    pub target: String,
    pub name: String,
    pub fact: String,
    #[serde(default)]
    pub source_label: Option<String>,
    #[serde(default)]
    pub target_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredEpisodeDoc {
    #[serde(default)]
    pub facts: Vec<StructuredFact>,
    #[serde(default)]
    pub entities: Vec<StructuredEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredEntity {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct IngestReport {
    pub episode_id: String,
    pub nodes_upserted: usize,
    pub facts_asserted: usize,
    pub reference_time: String,
}

/// Ingest structured JSON. All fact `valid_at` = `reference_time` (episode event time).
pub fn ingest_structured_json(
    g: &mut TemporalGraph,
    mission_id: &str,
    reference_time: &str,
    json_body: &str,
    attested: bool,
) -> Result<IngestReport, ExtractError> {
    let doc: StructuredEpisodeDoc =
        serde_json::from_str(json_body).map_err(|e| ExtractError::Json(e.to_string()))?;
    ingest_structured(g, mission_id, reference_time, json_body, &doc, attested)
}

fn ingest_structured(
    g: &mut TemporalGraph,
    mission_id: &str,
    reference_time: &str,
    raw: &str,
    doc: &StructuredEpisodeDoc,
    attested: bool,
) -> Result<IngestReport, ExtractError> {
    let digest = format!("sha256:{}", hex_sha256(raw.as_bytes()));
    let episode_id = format!("ep:ingest:{}", &digest[7..19]);
    let ep = Episode {
        id: episode_id.clone(),
        mission_id: mission_id.to_string(),
        group_id: format!("mission:{mission_id}"),
        source: if attested {
            EpisodeSource::Attested
        } else {
            EpisodeSource::Json
        },
        content: raw.to_string(),
        content_digest: Some(digest),
        valid_at: reference_time.to_string(),
        created_at: reference_time.to_string(),
        actor_id: Some("extractor:deterministic".into()),
    };
    g.add_episode(ep)?;

    let mut nodes = 0usize;
    for ent in &doc.entities {
        g.upsert_node(seed_entity(
            &ent.id,
            &ent.name,
            mission_id,
            &format!("mission:{mission_id}"),
            reference_time,
        ));
        nodes += 1;
    }

    let epistemic = if attested {
        EpistemicKind::Fact
    } else {
        EpistemicKind::Inference
    };

    let mut facts_n = 0usize;
    for (i, f) in doc.facts.iter().enumerate() {
        if g.node(&f.source).is_none() {
            g.upsert_node(seed_entity(
                &f.source,
                f.source_label.as_deref().unwrap_or(&f.source),
                mission_id,
                &format!("mission:{mission_id}"),
                reference_time,
            ));
            nodes += 1;
        }
        if g.node(&f.target).is_none() {
            g.upsert_node(seed_entity(
                &f.target,
                f.target_label.as_deref().unwrap_or(&f.target),
                mission_id,
                &format!("mission:{mission_id}"),
                reference_time,
            ));
            nodes += 1;
        }
        let fact = relate_fact(
            &format!("fact:ingest:{episode_id}:{i}"),
            &f.source,
            &f.target,
            &f.name,
            &f.fact,
            &episode_id,
            reference_time, // NEVER wall-clock — refuse wall-clock timestamp bugs
            reference_time,
            mission_id,
            &format!("mission:{mission_id}"),
            epistemic,
        );
        g.assert_fact(fact)?;
        facts_n += 1;
    }

    Ok(IngestReport {
        episode_id,
        nodes_upserted: nodes,
        facts_asserted: facts_n,
        reference_time: reference_time.to_string(),
    })
}

/// Heuristic text ingest: lines `SUBJECT -REL-> OBJECT: detail` → Inference facts.
/// `valid_at` always = `reference_time`.
pub fn ingest_text_triples(
    g: &mut TemporalGraph,
    mission_id: &str,
    reference_time: &str,
    text: &str,
) -> Result<IngestReport, ExtractError> {
    let mut facts = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Pattern: A -NAME-> B: optional detail
        if let Some((left, rest)) = line.split_once("->") {
            let left = left.trim().trim_end_matches('-').trim();
            let (rel_src, rel_name) = if let Some((s, n)) = left.rsplit_once('-') {
                (s.trim(), n.trim())
            } else {
                (left, "RELATES_TO")
            };
            let (tgt, detail) = if let Some((t, d)) = rest.split_once(':') {
                (t.trim(), d.trim())
            } else {
                (rest.trim(), rest.trim())
            };
            if rel_src.is_empty() || tgt.is_empty() {
                continue;
            }
            let src_id = slug_id(rel_src);
            let tgt_id = slug_id(tgt);
            facts.push(StructuredFact {
                source: src_id,
                target: tgt_id,
                name: rel_name.to_uppercase().replace(' ', "_"),
                fact: if detail.is_empty() {
                    format!("{rel_src} {rel_name} {tgt}")
                } else {
                    detail.to_string()
                },
                source_label: Some(rel_src.to_string()),
                target_label: Some(tgt.to_string()),
            });
        }
    }
    if facts.is_empty() {
        return Err(ExtractError::Rejected(
            "no triples found — use 'Subject -REL-> Object: detail' lines or JSON".into(),
        ));
    }
    let doc = StructuredEpisodeDoc {
        facts,
        entities: vec![],
    };
    let raw = serde_json::to_string(&serde_json::json!({ "facts": doc.facts })).unwrap();
    ingest_structured(g, mission_id, reference_time, &raw, &doc, false)
}

fn slug_id(s: &str) -> String {
    let mut out = String::from("ent:");
    for c in s.chars().flat_map(|c| c.to_lowercase()) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_end_matches('_').to_string()
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}
