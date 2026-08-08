//! Trust gates on the temporal graph (authorize / deny / falsifier).

use std::fs;
use std::path::Path;

use super::io::{load_graph, save_graph};
use crate::{chrono_now_iso, CliError};

pub fn require_authorized(mission_dir: &str, capability: &str) -> Result<(), CliError> {
    let g = load_graph(mission_dir)?;
    let now = chrono_now_iso();
    if g.capability_authorized(capability, &now) {
        return Ok(());
    }
    let reason = format!(
        "capability `{capability}` is not authorized by the temporal graph at {now}          (need active authorizes edge → action:{capability}; use `unify graph authorize`)"
    );
    if let Err(e) = record_denial_episode(mission_dir, capability, &reason) {
        eprintln!("warning: denial episode not recorded for `{capability}`: {e}");
    }
    Err(CliError::Verify(reason))
}

pub fn record_denial_episode(
    mission_dir: &str,
    capability: &str,
    reason: &str,
) -> Result<(), CliError> {
    use aevum_evidence_graph::{relate_fact, seed_entity, Episode, EpisodeSource, EpistemicKind};
    use aevum_memory_fabric::{MemoryBackend, SqliteBackend};

    let meta = crate::load_metadata(mission_dir)?;
    let mut g = load_graph(mission_dir)?;
    let now = chrono_now_iso();
    let group = format!("mission:{}", meta.mission.mission_id);
    let digest = crate::sha256_hex(&format!("{now}|{capability}|{reason}"));
    let ep_id = format!("ep:deny:{}", &digest[7..19]);
    g.add_episode(Episode {
        id: ep_id.clone(),
        mission_id: meta.mission.mission_id.clone(),
        group_id: group.clone(),
        source: EpisodeSource::Json,
        content: serde_json::json!({
            "kind": "DENIED_CAPABILITY",
            "capability": capability,
            "reason": reason,
        })
        .to_string(),
        content_digest: Some(digest),
        valid_at: now.clone(),
        created_at: now.clone(),
        actor_id: Some("aevum-firewall".into()),
    })
    .map_err(|e| CliError::Verify(e.to_string()))?;
    g.upsert_node(seed_entity(
        "ent:firewall",
        "aevum-firewall",
        &meta.mission.mission_id,
        &group,
        &now,
    ));
    g.upsert_node(seed_entity(
        &format!("action:{capability}"),
        capability,
        &meta.mission.mission_id,
        &group,
        &now,
    ));
    let fact = relate_fact(
        &format!("fact:deny:{ep_id}"),
        "ent:firewall",
        &format!("action:{capability}"),
        "DENIED_CAPABILITY",
        reason,
        &ep_id,
        &now,
        &now,
        &meta.mission.mission_id,
        &group,
        EpistemicKind::Inference,
    );
    g.assert_fact(fact)
        .map_err(|e| CliError::Verify(e.to_string()))?;
    save_graph(mission_dir, &g)?;
    if let Ok(mut sb) = SqliteBackend::open(mission_dir) {
        *sb.graph_mut() = g;
        let _ = sb.save();
    }
    Ok(())
}

pub fn require_falsifier_if_needed(
    mission_dir: &str,
    risk: aevum_autonomy_governor::RiskClass,
) -> Result<(), CliError> {
    use aevum_autonomy_governor::RiskClass;
    if risk.rank() < RiskClass::R3.rank() {
        return Ok(());
    }
    let path = Path::new(mission_dir).join("falsifier.jsonl");
    if !path.exists() {
        return Err(CliError::Verify(
            "R3+ blocked: missing falsifier.jsonl — run `unify falsify --mission … --reason …`"
                .into(),
        ));
    }
    let raw = fs::read_to_string(&path).map_err(|e| CliError::Io(e.to_string()))?;
    let count = raw.lines().filter(|l| !l.trim().is_empty()).count();
    if count == 0 {
        return Err(CliError::Verify(
            "R3+ blocked: falsifier.jsonl is empty".into(),
        ));
    }
    let has_falsifier = raw.lines().filter(|l| !l.trim().is_empty()).any(|l| {
        let v: serde_json::Value = serde_json::from_str(l).unwrap_or_default();
        v.get("role").and_then(|r| r.as_str()) == Some("falsifier")
    });
    if !has_falsifier {
        return Err(CliError::Verify(
            "R3+ blocked: no falsifier-role challenge recorded".into(),
        ));
    }
    Ok(())
}
