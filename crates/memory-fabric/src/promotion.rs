//! Epistemic promotion — untrusted remote recall → attested authorize.
//!
//! Remote facts enter as **Inference** episodes. They cannot authorize until
//! an operator (or attested pipeline) promotes them with a content digest.

use aevum_evidence_graph::{
    may_authorize, EdgeKind, Episode, EpisodeSource, EpistemicKind, Fact, FirewallVerdict,
    GraphNode, NodeKind, TemporalGraph,
};
use sha2::{Digest, Sha256};

use crate::backend::{MemoryError, RemoteFact};

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    format!("sha256:{}", hex::encode(h.finalize()))
}

fn now_iso() -> String {
    // Second precision UTC — sufficient for promotion timestamps.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Reuse a compact formatter via chrono-less approach matching unify-cli.
    format_epoch(secs)
}

fn format_epoch(now: u64) -> String {
    let minutes = (now / 60) % 60;
    let hours = (now / 3600) % 24;
    let days = now / 86400;
    let days_per_month = [31u64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut year = 1970u64;
    let mut day_of_year = days;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let yd = if leap { 366 } else { 365 };
        if day_of_year < yd {
            break;
        }
        day_of_year -= yd;
        year += 1;
    }
    let mut month = 0usize;
    let mut dom = day_of_year;
    for (i, dm) in days_per_month.iter().enumerate() {
        let m = if i == 1 && ((year % 4 == 0 && year % 100 != 0) || year % 400 == 0) {
            29
        } else {
            *dm
        };
        if dom < m {
            month = i + 1;
            break;
        }
        dom -= m;
    }
    format!("{year:04}-{month:02}-{dom:02}T{hours:02}:{minutes:02}:00+00:00")
}

/// Ingest remote/untrusted facts as non-authorizing inference nodes + episodes.
pub fn ingest_remote_as_inference(
    g: &mut TemporalGraph,
    mission_id: &str,
    remotes: &[RemoteFact],
) -> Result<Vec<String>, MemoryError> {
    let now = now_iso();
    let group = format!("mission:{mission_id}");
    let mut ids = Vec::new();
    for r in remotes {
        let ep_id = format!("ep_remote_{}", r.uuid);
        if g.episode(&ep_id).is_some() {
            continue;
        }
        let content = serde_json::json!({
            "uuid": r.uuid,
            "fact": r.fact,
            "name": r.name,
            "valid_at": r.valid_at,
            "invalid_at": r.invalid_at,
            "origin": "remote"
        })
        .to_string();
        g.add_episode(Episode {
            id: ep_id.clone(),
            mission_id: mission_id.to_string(),
            group_id: group.clone(),
            source: EpisodeSource::Json, // NOT Attested — cannot be primary evidence
            content,
            content_digest: None,
            valid_at: r.valid_at.clone().unwrap_or_else(|| now.clone()),
            created_at: now.clone(),
            actor_id: Some("remote-ingest".into()),
        })
        .map_err(|e| MemoryError::Promotion(e.to_string()))?;

        let node_id = format!("remote:{}", r.uuid);
        g.upsert_node(GraphNode {
            id: node_id.clone(),
            kind: NodeKind::Hypothesis,
            name: if r.name.is_empty() {
                r.fact.clone()
            } else {
                r.name.clone()
            },
            summary: r.fact.clone(),
            mission_id: mission_id.to_string(),
            group_id: group.clone(),
            created_at: now.clone(),
            embedding: None,
        });

        // RelatesTo as Inference — firewall blocks authorizes.
        let fact_id = format!("fact:remote:{}", r.uuid);
        if g.fact(&fact_id).is_none() {
            let fact = Fact {
                id: fact_id.clone(),
                kind: EdgeKind::RelatesTo,
                source_node_id: node_id,
                target_node_id: "claim:constitution".to_string(),
                name: if r.name.is_empty() {
                    "REMOTE_RECALL".into()
                } else {
                    r.name.clone()
                },
                fact: r.fact.clone(),
                epistemic: EpistemicKind::Inference,
                episode_ids: vec![ep_id],
                valid_at: r.valid_at.clone().unwrap_or_else(|| now.clone()),
                invalid_at: r.invalid_at.clone(),
                created_at: now.clone(),
                expired_at: None,
                fact_digest: None,
                group_id: group.clone(),
                mission_id: mission_id.to_string(),
            };
            if g.node("claim:constitution").is_none() {
                g.upsert_node(GraphNode {
                    id: "claim:constitution".into(),
                    kind: NodeKind::Claim,
                    name: "Mission constitution".into(),
                    summary: "bootstrap".into(),
                    mission_id: mission_id.to_string(),
                    group_id: group.clone(),
                    created_at: now.clone(),
                    embedding: None,
                });
            }
            g.assert_fact(fact)
                .map_err(|e| MemoryError::Promotion(e.to_string()))?;
        }
        ids.push(fact_id);
    }
    Ok(ids)
}

/// Promote a previously ingested remote fact to authorizing capability.
/// Requires attested content (digest) — the operator attests the observation.
pub fn promote_to_authorize(
    g: &mut TemporalGraph,
    mission_id: &str,
    remote_fact_id: &str,
    capability: &str,
    attested_content: &str,
) -> Result<String, MemoryError> {
    let src = g
        .fact(remote_fact_id)
        .ok_or_else(|| MemoryError::Promotion(format!("unknown fact {remote_fact_id}")))?
        .clone();
    if !matches!(
        src.epistemic,
        EpistemicKind::Inference | EpistemicKind::Hypothesis
    ) {
        return Err(MemoryError::Promotion(
            "only inference/hypothesis remote facts can be promoted".into(),
        ));
    }
    let now = now_iso();
    let group = format!("mission:{mission_id}");
    let digest = sha256_hex(attested_content);
    let ep_id = format!("ep_promote_{}", &digest[7..19]);
    g.add_episode(Episode {
        id: ep_id.clone(),
        mission_id: mission_id.to_string(),
        group_id: group.clone(),
        source: EpisodeSource::Attested,
        content: attested_content.to_string(),
        content_digest: Some(digest.clone()),
        valid_at: now.clone(),
        created_at: now.clone(),
        actor_id: Some("spiffe://local.aevum/agent/promoter".into()),
    })
    .map_err(|e| MemoryError::Promotion(e.to_string()))?;

    let action_id = format!("action:{capability}");
    if g.node(&action_id).is_none() {
        g.upsert_node(GraphNode {
            id: action_id.clone(),
            kind: NodeKind::ActionIntent,
            name: capability.to_string(),
            summary: format!("promoted from {remote_fact_id}"),
            mission_id: mission_id.to_string(),
            group_id: group.clone(),
            created_at: now.clone(),
            embedding: None,
        });
    }
    if g.node("claim:constitution").is_none() {
        return Err(MemoryError::Promotion(
            "claim:constitution missing — seed mission first".into(),
        ));
    }

    let auth = Fact {
        id: format!("fact:promoted:{capability}:{}", &digest[7..15]),
        kind: EdgeKind::Authorizes,
        source_node_id: "claim:constitution".into(),
        target_node_id: action_id,
        name: "AUTHORIZES".into(),
        fact: format!("promoted: {}", src.fact),
        epistemic: EpistemicKind::Fact,
        episode_ids: vec![ep_id],
        valid_at: now.clone(),
        invalid_at: None,
        created_at: now,
        expired_at: None,
        fact_digest: Some(digest),
        group_id: group,
        mission_id: mission_id.to_string(),
    };
    match may_authorize(&auth) {
        FirewallVerdict::Allow => {}
        FirewallVerdict::Deny(r) => return Err(MemoryError::Promotion(r.into())),
    }
    let id = auth.id.clone();
    g.assert_fact(auth)
        .map_err(|e| MemoryError::Promotion(e.to_string()))?;
    Ok(id)
}
