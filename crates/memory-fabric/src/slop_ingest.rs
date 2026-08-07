//! Ingest deterministic AI-slop findings into the TemporalGraph as Inference only.
//!
//! Unprecedented combo: Trusted Autonomy ∩ offline slop firewall.
//! Slop evidence can never authorize side-effects (epistemic firewall).

use aevum_evidence_graph::{
    relate_fact, seed_entity, Episode, EpisodeSource, EpistemicKind, TemporalGraph,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::extract::{ExtractError, IngestReport};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlopFinding {
    pub rule: String,
    pub severity: String, // "block" | "warn"
    pub path: String,
    pub line: u32,
    pub message: String,
    #[serde(default)]
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlopReport {
    pub findings: Vec<SlopFinding>,
    pub blocking: u32,
}

impl SlopReport {
    pub fn from_json(raw: &str) -> Result<Self, ExtractError> {
        serde_json::from_str(raw).map_err(|e| ExtractError::Json(e.to_string()))
    }

    pub fn blockers(&self) -> impl Iterator<Item = &SlopFinding> {
        self.findings.iter().filter(|f| f.severity == "block")
    }
}

/// Stamp slop findings onto the graph as Inference — never Fact, never Authorizes.
pub fn ingest_slop_report(
    g: &mut TemporalGraph,
    mission_id: &str,
    report: &SlopReport,
    reference_time: &str,
) -> Result<IngestReport, ExtractError> {
    let group = format!("mission:{mission_id}");
    let raw = serde_json::to_string(report).map_err(|e| ExtractError::Json(e.to_string()))?;
    let digest = format!("sha256:{}", hex_sha256(raw.as_bytes()));
    let episode_id = format!("ep:slop:{}", &digest[7..19]);

    g.add_episode(Episode {
        id: episode_id.clone(),
        mission_id: mission_id.to_string(),
        group_id: group.clone(),
        source: EpisodeSource::Json,
        content: raw,
        content_digest: Some(digest),
        valid_at: reference_time.to_string(),
        created_at: reference_time.to_string(),
        actor_id: Some("slopcheck".into()),
    })?;

    g.upsert_node(seed_entity(
        "ent:slopcheck",
        "slopcheck",
        mission_id,
        &group,
        reference_time,
    ));
    g.upsert_node(seed_entity(
        "ent:codebase",
        "codebase",
        mission_id,
        &group,
        reference_time,
    ));

    let mut facts_n = 0usize;
    for (i, f) in report.findings.iter().enumerate() {
        let name = if f.severity == "block" {
            "SLOP_BLOCK"
        } else {
            "SLOP_WARN"
        };
        let fact_text = format!(
            "[{}] {}:{} — {} | {}",
            f.rule, f.path, f.line, f.message, f.snippet
        );
        let id = format!("fact:slop:{episode_id}:{i}");
        let fact = relate_fact(
            &id,
            "ent:slopcheck",
            "ent:codebase",
            name,
            &fact_text,
            &episode_id,
            reference_time,
            reference_time,
            mission_id,
            &group,
            EpistemicKind::Inference, // NEVER Fact — cannot authorize
        );
        g.assert_fact(fact)?;
        facts_n += 1;
    }

    // Summary fact when clean
    if report.findings.is_empty() {
        let fact = relate_fact(
            &format!("fact:slop:{episode_id}:clean"),
            "ent:slopcheck",
            "ent:codebase",
            "SLOP_CLEAN",
            "slopcheck: 0 blocking, 0 warnings",
            &episode_id,
            reference_time,
            reference_time,
            mission_id,
            &group,
            EpistemicKind::Inference,
        );
        g.assert_fact(fact)?;
        facts_n += 1;
    }

    Ok(IngestReport {
        episode_id,
        nodes_upserted: 2,
        facts_asserted: facts_n,
        reference_time: reference_time.to_string(),
    })
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}
