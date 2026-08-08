//! Hookify-style mission rules → Inference-only evidence (never authorizes).

use std::fs;
use std::path::Path;

use aevum_evidence_graph::{relate_fact, seed_entity, Episode, EpisodeSource, EpistemicKind};
use aevum_memory_fabric::{MemoryBackend, SqliteBackend};
use serde::Serialize;

use crate::graph_cmd::{load_graph, save_graph};
use crate::{chrono_now_iso, load_metadata, require_value, sha256_hex, CliError};

struct RuleFrontmatter {
    name: String,
    pattern: String,
    severity: String,
}

#[derive(Debug, Serialize)]
struct RuleHit {
    rule: String,
    severity: String,
    path: String,
    line: usize,
    message: String,
}

/// `unify rules scan --mission <dir> [--repo <path>]`
pub fn cmd_rules_scan(args: &[String]) -> Result<(), CliError> {
    let mission = require_value(args, "--mission")?;
    let repo = optional(args, "--repo").unwrap_or_else(|| ".".into());
    let rules_dir = Path::new(&mission).join("rules");
    if !rules_dir.is_dir() {
        fs::create_dir_all(&rules_dir).map_err(|e| CliError::Io(e.to_string()))?;
        let default = r#"---
name: deny-sh-c
pattern: sh -c
severity: block
---
Refuse shell-string execution (D14). Prefer process.exec.argv.
"#;
        fs::write(rules_dir.join("deny-sh-c.md"), default)
            .map_err(|e| CliError::Io(e.to_string()))?;
    }

    let rules = load_rules(&rules_dir)?;
    let mut hits: Vec<RuleHit> = Vec::new();
    scan_tree(Path::new(&repo), &rules, &mut hits)?;

    let meta = load_metadata(&mission)?;
    let mut g = load_graph(&mission)?;
    let now = chrono_now_iso();
    let group = format!("mission:{}", meta.mission.mission_id);
    let digest = sha256_hex(&format!("{now}|{}|{:?}", hits.len(), hits.len()));
    let ep_id = format!("ep:rules:{}", &digest[7..19]);
    let raw = serde_json::to_string(&hits).unwrap_or_default();
    g.add_episode(Episode {
        id: ep_id.clone(),
        mission_id: meta.mission.mission_id.clone(),
        group_id: group.clone(),
        source: EpisodeSource::Json,
        content: raw.clone(),
        content_digest: Some(digest),
        valid_at: now.clone(),
        created_at: now.clone(),
        actor_id: Some("rules-scanner".into()),
    })
    .map_err(|e| CliError::Verify(e.to_string()))?;

    g.upsert_node(seed_entity(
        "ent:rules",
        "rules-scanner",
        &meta.mission.mission_id,
        &group,
        &now,
    ));
    g.upsert_node(seed_entity(
        "ent:codebase",
        "codebase",
        &meta.mission.mission_id,
        &group,
        &now,
    ));

    for (i, hit) in hits.iter().take(50).enumerate() {
        let name = if hit.severity == "block" {
            "RULE_BLOCK"
        } else {
            "RULE_WARN"
        };
        let fact_text = format!("[{}] {}:{} — {}", hit.rule, hit.path, hit.line, hit.message);
        let fact = relate_fact(
            &format!("fact:rule:{ep_id}:{i}"),
            "ent:rules",
            "ent:codebase",
            name,
            &fact_text,
            &ep_id,
            &now,
            &now,
            &meta.mission.mission_id,
            &group,
            EpistemicKind::Inference,
        );
        g.assert_fact(fact)
            .map_err(|e| CliError::Verify(e.to_string()))?;
    }
    if hits.is_empty() {
        let fact = relate_fact(
            &format!("fact:rule:{ep_id}:clean"),
            "ent:rules",
            "ent:codebase",
            "RULE_CLEAN",
            "rules scan: 0 hits",
            &ep_id,
            &now,
            &now,
            &meta.mission.mission_id,
            &group,
            EpistemicKind::Inference,
        );
        g.assert_fact(fact)
            .map_err(|e| CliError::Verify(e.to_string()))?;
    }

    save_graph(&mission, &g)?;
    if let Ok(mut sb) = SqliteBackend::open(&mission) {
        *sb.graph_mut() = g;
        let _ = sb.save();
    }

    let report = serde_json::json!({
        "hits": hits.iter().take(100).collect::<Vec<_>>(),
        "hit_count": hits.len(),
        "blocking": hits.iter().filter(|h| h.severity == "block").count(),
        "episode_id": ep_id,
        "note": "rule hits are Inference — never authorize"
    });
    let out = Path::new(&mission).join("rules-report.json");
    fs::write(&out, serde_json::to_string_pretty(&report).unwrap())
        .map_err(|e| CliError::Io(e.to_string()))?;
    println!(
        "✓ rules scan — {} hit(s), episode={} → {}",
        hits.len(),
        ep_id,
        out.display()
    );
    Ok(())
}

fn load_rules(dir: &Path) -> Result<Vec<RuleFrontmatter>, CliError> {
    let mut out = Vec::new();
    for ent in fs::read_dir(dir).map_err(|e| CliError::Io(e.to_string()))? {
        let ent = ent.map_err(|e| CliError::Io(e.to_string()))?;
        let p = ent.path();
        if p.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let raw = fs::read_to_string(&p).map_err(|e| CliError::Io(e.to_string()))?;
        if let Some(fm) = parse_frontmatter(&raw) {
            out.push(fm);
        }
    }
    Ok(out)
}

fn parse_frontmatter(raw: &str) -> Option<RuleFrontmatter> {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let rest = &trimmed[3..];
    let end = rest.find("---")?;
    let yaml = &rest[..end];
    let mut name = String::new();
    let mut pattern = String::new();
    let mut severity = "warn".to_string();
    for line in yaml.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim().trim_matches('"').trim_matches('\'');
        match k {
            "name" => name = v.to_string(),
            "pattern" => {
                pattern = v.replace("\\s+", " ").replace("\\s", " ");
            }
            "severity" => severity = v.to_string(),
            _ => {}
        }
    }
    if pattern.is_empty() {
        return None;
    }
    if name.is_empty() {
        name = pattern.clone();
    }
    Some(RuleFrontmatter {
        name,
        pattern,
        severity,
    })
}

fn scan_tree(
    root: &Path,
    rules: &[RuleFrontmatter],
    hits: &mut Vec<RuleHit>,
) -> Result<(), CliError> {
    let skip = ["target", "node_modules", ".git", "dist", "__pycache__"];
    walk(root, rules, hits, &skip, root)
}

fn walk(
    dir: &Path,
    rules: &[RuleFrontmatter],
    hits: &mut Vec<RuleHit>,
    skip: &[&str],
    root: &Path,
) -> Result<(), CliError> {
    let rd = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };
    for ent in rd.flatten() {
        let p = ent.path();
        let name = ent.file_name().to_string_lossy().to_string();
        if skip.iter().any(|s| *s == name) {
            continue;
        }
        if p.is_dir() {
            walk(&p, rules, hits, skip, root)?;
            continue;
        }
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !matches!(
            ext.as_str(),
            "rs" | "ts" | "tsx" | "js" | "py" | "sh" | "md" | "toml" | "json" | "yml" | "yaml"
        ) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&p) else {
            continue;
        };
        let rel = p.strip_prefix(root).unwrap_or(&p);
        for (li, line) in text.lines().enumerate() {
            for rule in rules {
                if line.contains(&rule.pattern) {
                    hits.push(RuleHit {
                        rule: rule.name.clone(),
                        severity: rule.severity.clone(),
                        path: rel.display().to_string(),
                        line: li + 1,
                        message: format!("matched '{}'", rule.pattern),
                    });
                }
            }
        }
    }
    Ok(())
}

fn optional(args: &[String], key: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == key).map(|w| w[1].clone())
}
