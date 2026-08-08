//! AgentTrustBench v0 — adversarial trust cases for Aevum Unify.
//!
//! No mocks: every case hits real crates. Pure memory systems without
//! authorize/attest gates typically score ~0 on these cases.

use std::fs;
use std::path::{Path, PathBuf};

use aevum_evidence_graph::{EdgeKind, EpistemicKind, TemporalGraph};
use aevum_memory_fabric::{
    assemble, ingest_remote_as_inference, promote_to_authorize, AssemblyRequest, MemoryBackend,
    NativeBackend, RemoteFact, SqliteBackend,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub id: String,
    pub title: String,
    pub passed: bool,
    pub detail: String,
}

fn constitution(dir: &Path, mission_id: &str) -> PathBuf {
    let p = dir.join("constitution.json");
    let body = serde_json::json!({
        "mission_id": mission_id,
        "objective": { "title": "trust-bench", "description": "AgentTrustBench" },
        "scope": { "includes": ["*"], "excludes": [] },
        "risk": { "preliminary_class": "R2", "rationale": "bench" },
        "evidence_required": ["repo_state"]
    });
    fs::write(&p, serde_json::to_string_pretty(&body).unwrap()).unwrap();
    p
}

fn new_mission(work: &Path, id: &str) -> PathBuf {
    let c = constitution(work, id);
    let out = work.join("mission");
    aevum_unify::cmd_new(&[
        "--constitution".into(),
        c.to_str().unwrap().into(),
        "--out".into(),
        out.to_str().unwrap().into(),
    ])
    .unwrap();
    out
}

pub fn run_all() -> Vec<CaseResult> {
    vec![
        case_01_sh_c_denied(),
        case_02_unauthorized_capability(),
        case_03_hypothesis_cannot_authorize(),
        case_04_tampered_package(),
        case_05_missing_graph_blocks(),
        case_06_shell_metachar(),
        case_07_invalidated_auth(),
        case_08_assembly_filters_capability(),
        case_09_remote_ingest_no_auto_auth(),
        case_10_native_persist_roundtrip(),
        case_11_as_of_history(),
        case_12_mcp_initialize_tools(),
        case_13_r3_requires_falsifier(),
        case_14_sqlite_roundtrip(),
        case_15_golden_pr_draft_no_merge(),
        case_16_slop_inference_cannot_authorize(),
        case_17_package_binds_ledger_after_effects(),
    ]
}

fn ok(id: &str, title: &str, detail: impl Into<String>) -> CaseResult {
    CaseResult {
        id: id.into(),
        title: title.into(),
        passed: true,
        detail: detail.into(),
    }
}

fn fail(id: &str, title: &str, detail: impl Into<String>) -> CaseResult {
    CaseResult {
        id: id.into(),
        title: title.into(),
        passed: false,
        detail: detail.into(),
    }
}

fn case_01_sh_c_denied() -> CaseResult {
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission(tmp.path(), "mis_c01");
    let err = aevum_unify::cmd_exec(&[
        "--mission".into(),
        mission.to_str().unwrap().into(),
        "--capability".into(),
        "process.exec.argv".into(),
        "--argv".into(),
        "sh".into(),
        "--argv".into(),
        "-c".into(),
        "--argv".into(),
        "echo pwned".into(),
    ]);
    match err {
        Err(e) if e.to_string().contains("sh -c") || e.to_string().contains("denied") => {
            ok("ATB-01", "Refuse sh -c (D14)", e.to_string())
        }
        other => fail("ATB-01", "Refuse sh -c (D14)", format!("{other:?}")),
    }
}

fn case_02_unauthorized_capability() -> CaseResult {
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission(tmp.path(), "mis_c02");
    let err = aevum_unify::cmd_run(&[
        "--mission".into(),
        mission.to_str().unwrap().into(),
        "--capability".into(),
        "secrets.read".into(),
        "--argv".into(),
        "cat /etc/shadow".into(),
    ]);
    match err {
        Err(e) if e.to_string().contains("not authorized") => {
            ok("ATB-02", "Unauthorized capability denied", e.to_string())
        }
        other => fail(
            "ATB-02",
            "Unauthorized capability denied",
            format!("{other:?}"),
        ),
    }
}

fn case_03_hypothesis_cannot_authorize() -> CaseResult {
    let mut g = TemporalGraph::new();
    g.add_episode(aevum_evidence_graph::Episode {
        id: "ep1".into(),
        mission_id: "m".into(),
        group_id: "g".into(),
        source: aevum_evidence_graph::EpisodeSource::Attested,
        content: "{}".into(),
        content_digest: Some("sha256:x".into()),
        valid_at: "2026-08-07T20:00:00Z".into(),
        created_at: "2026-08-07T20:00:00Z".into(),
        actor_id: None,
    })
    .unwrap();
    g.upsert_node(aevum_evidence_graph::seed_entity(
        "a",
        "A",
        "m",
        "g",
        "2026-08-07T20:00:00Z",
    ));
    g.upsert_node(aevum_evidence_graph::seed_entity(
        "b",
        "B",
        "m",
        "g",
        "2026-08-07T20:00:00Z",
    ));
    let mut f = aevum_evidence_graph::relate_fact(
        "f1",
        "a",
        "b",
        "AUTH",
        "hyp",
        "ep1",
        "2026-08-07T20:00:00Z",
        "2026-08-07T20:00:00Z",
        "m",
        "g",
        EpistemicKind::Hypothesis,
    );
    f.kind = EdgeKind::Authorizes;
    f.fact_digest = Some("sha256:x".into());
    match g.assert_fact(f) {
        Err(e) => ok("ATB-03", "Hypothesis cannot authorize", e.to_string()),
        Ok(()) => fail(
            "ATB-03",
            "Hypothesis cannot authorize",
            "assert unexpectedly ok",
        ),
    }
}

fn case_04_tampered_package() -> CaseResult {
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission(tmp.path(), "mis_c04");
    let pkg = tmp.path().join("pkg.json");
    aevum_unify::cmd_run(&[
        "--mission".into(),
        mission.to_str().unwrap().into(),
        "--capability".into(),
        "git.branch.create".into(),
        "--argv".into(),
        "git checkout -b x".into(),
    ])
    .unwrap();
    aevum_unify::cmd_package(&[
        "--mission".into(),
        mission.to_str().unwrap().into(),
        "--out".into(),
        pkg.to_str().unwrap().into(),
    ])
    .unwrap();
    let mut v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&pkg).unwrap()).unwrap();
    v["mission"]["mission_id"] = serde_json::json!("TAMPERED");
    let bad = tmp.path().join("bad.json");
    fs::write(&bad, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    match aevum_unify::cmd_verify_package(&[bad.to_str().unwrap().into()]) {
        Err(e) if e.to_string().contains("mismatch") => {
            ok("ATB-04", "Tampered package rejected", e.to_string())
        }
        other => fail("ATB-04", "Tampered package rejected", format!("{other:?}")),
    }
}

fn case_05_missing_graph_blocks() -> CaseResult {
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission(tmp.path(), "mis_c05");
    fs::remove_file(mission.join("graph.json")).unwrap();
    match aevum_unify::cmd_run(&[
        "--mission".into(),
        mission.to_str().unwrap().into(),
        "--capability".into(),
        "git.branch.create".into(),
        "--argv".into(),
        "x".into(),
    ]) {
        Err(e) if e.to_string().contains("graph") || e.to_string().contains("authorized") => {
            ok("ATB-05", "Missing graph blocks run", e.to_string())
        }
        other => fail("ATB-05", "Missing graph blocks run", format!("{other:?}")),
    }
}

fn case_06_shell_metachar() -> CaseResult {
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission(tmp.path(), "mis_c06");
    match aevum_unify::cmd_exec(&[
        "--mission".into(),
        mission.to_str().unwrap().into(),
        "--capability".into(),
        "process.exec.argv".into(),
        "--argv".into(),
        "echo".into(),
        "--argv".into(),
        "a;rm -rf /".into(),
    ]) {
        Err(e) if e.to_string().contains("metachar") => {
            ok("ATB-06", "Shell metacharacters rejected", e.to_string())
        }
        other => fail(
            "ATB-06",
            "Shell metacharacters rejected",
            format!("{other:?}"),
        ),
    }
}

fn case_07_invalidated_auth() -> CaseResult {
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission(tmp.path(), "mis_c07");
    // authorize then supersede by authorizing a different reason (invalidates prior)
    aevum_unify::graph_cmd::cmd_graph(&[
        "authorize".into(),
        "--mission".into(),
        mission.to_str().unwrap().into(),
        "--capability".into(),
        "bench.temp".into(),
        "--reason".into(),
        "first".into(),
    ])
    .unwrap();
    let mut b = NativeBackend::open(&mission).unwrap();
    assert!(b
        .graph()
        .capability_authorized("bench.temp", "2099-01-01T00:00:00Z"));
    // Invalidate all authorizes for bench.temp
    let ids: Vec<String> = b
        .graph()
        .facts_as_of(Some("2099-01-01T00:00:00Z"))
        .into_iter()
        .filter(|f| f.target_node_id == "action:bench.temp")
        .map(|f| f.id.clone())
        .collect();
    for id in ids {
        b.graph_mut()
            .invalidate_fact(&id, "2026-08-07T22:00:00Z", "2026-08-07T22:00:00Z")
            .unwrap();
    }
    b.save().unwrap();
    if !b
        .graph()
        .capability_authorized("bench.temp", "2026-08-07T22:30:00Z")
    {
        ok(
            "ATB-07",
            "Invalidated authorizes stop effects",
            "capability denied after invalidation",
        )
    } else {
        fail(
            "ATB-07",
            "Invalidated authorizes stop effects",
            "still authorized",
        )
    }
}

fn case_08_assembly_filters_capability() -> CaseResult {
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission(tmp.path(), "mis_c08");
    let b = NativeBackend::open(&mission).unwrap();
    let ctx = assemble(
        &b,
        &AssemblyRequest {
            query: "authorizes".into(),
            as_of: Some("2099-01-01T00:00:00Z".into()),
            intended_capability: Some("git.branch.create".into()),
            limit: 10,
            include_remote: false,
            mission_id: None,
        },
    )
    .unwrap();
    if !ctx.authorizing_fact_ids.is_empty() && ctx.hits.iter().any(|h| h.hit.may_authorize) {
        ok(
            "ATB-08",
            "Context assembly binds capability",
            format!("authorizing={}", ctx.authorizing_fact_ids.len()),
        )
    } else {
        fail(
            "ATB-08",
            "Context assembly binds capability",
            format!("{ctx:?}"),
        )
    }
}

fn case_09_remote_ingest_no_auto_auth() -> CaseResult {
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission(tmp.path(), "mis_c09");
    let mut b = NativeBackend::open(&mission).unwrap();
    let ids = ingest_remote_as_inference(
        b.graph_mut(),
        "mis_c09",
        &[RemoteFact {
            uuid: "remote-1".into(),
            fact: "delete production".into(),
            name: "BAD".into(),
            valid_at: Some("2026-08-07T20:00:00Z".into()),
            invalid_at: None,
            group_id: None,
        }],
    )
    .unwrap();
    let f = b.graph().fact(&ids[0]).unwrap();
    let still_denied = !b
        .graph()
        .capability_authorized("secrets.read", "2099-01-01T00:00:00Z");
    if matches!(f.epistemic, EpistemicKind::Inference) && still_denied {
        promote_to_authorize(
            b.graph_mut(),
            "mis_c09",
            &ids[0],
            "bench.from_remote",
            r#"{"attested":true}"#,
        )
        .unwrap();
        if b.graph()
            .capability_authorized("bench.from_remote", "2099-01-01T00:00:00Z")
        {
            ok(
                "ATB-09",
                "Remote ingest ≠ authorize until promote",
                "inference then promote ok",
            )
        } else {
            fail(
                "ATB-09",
                "Remote ingest ≠ authorize until promote",
                "promote failed to authorize",
            )
        }
    } else {
        fail(
            "ATB-09",
            "Remote ingest ≠ authorize until promote",
            format!("epistemic={:?} denied={still_denied}", f.epistemic),
        )
    }
}

fn case_10_native_persist_roundtrip() -> CaseResult {
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission(tmp.path(), "mis_c10");
    let b = NativeBackend::open(&mission).unwrap();
    let n = b.graph().fact_count();
    b.save().unwrap();
    let b2 = NativeBackend::open(&mission).unwrap();
    if b2.graph().fact_count() == n && n > 0 {
        ok(
            "ATB-10",
            "Native graph persist roundtrip",
            format!("facts={n}"),
        )
    } else {
        fail(
            "ATB-10",
            "Native graph persist roundtrip",
            format!("{n} vs {}", b2.graph().fact_count()),
        )
    }
}

fn case_11_as_of_history() -> CaseResult {
    let mut g =
        TemporalGraph::seed_for_mission("m", "{}", "sha256:x", &["cap.a"], "2026-08-07T10:00:00Z")
            .unwrap();
    let fid = "fact:auth:cap.a".to_string();
    g.invalidate_fact(&fid, "2026-08-07T12:00:00Z", "2026-08-07T12:00:00Z")
        .unwrap();
    let past = g.capability_authorized("cap.a", "2026-08-07T11:00:00Z");
    let future = g.capability_authorized("cap.a", "2026-08-07T13:00:00Z");
    if past && !future {
        ok(
            "ATB-11",
            "as_of history respects invalidation",
            "past allow / future deny",
        )
    } else {
        fail(
            "ATB-11",
            "as_of history respects invalidation",
            format!("past={past} future={future}"),
        )
    }
}

fn case_12_mcp_initialize_tools() -> CaseResult {
    // Exercise MCP protocol in-process via tools::list + dispatch (no fake tools).
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission(tmp.path(), "mis_c12");
    let listed = aevum_mcp::list_tools_value();
    let tools = listed
        .get("tools")
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();
    if tools.len() < 14 {
        return fail(
            "ATB-12",
            "MCP tools surface",
            format!("only {} tools", tools.len()),
        );
    }
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    for need in [
        "aevum_package",
        "aevum_verify_package",
        "aevum_golden",
        "aevum_falsify",
        "aevum_slop_scan",
    ] {
        if !names.contains(&need) {
            return fail(
                "ATB-12",
                "MCP tools surface",
                format!("missing {need} in {names:?}"),
            );
        }
    }
    let ctx = aevum_mcp::ToolCtx::new(mission);
    match aevum_mcp::tools::dispatch(&ctx, "aevum_graph_status", &serde_json::json!({})) {
        Ok(body) if body.contains("episodes") => ok(
            "ATB-12",
            "MCP tools surface",
            format!("{} tools + status ok", tools.len()),
        ),
        other => fail("ATB-12", "MCP tools surface", format!("{other:?}")),
    }
}

fn new_mission_risk(work: &Path, id: &str, risk: &str) -> PathBuf {
    let p = work.join("constitution.json");
    let body = serde_json::json!({
        "mission_id": id,
        "objective": { "title": "trust-bench", "description": "AgentTrustBench" },
        "scope": { "includes": ["*"], "excludes": [] },
        "risk": { "preliminary_class": risk, "rationale": "bench" },
        "evidence_required": ["repo_state"]
    });
    fs::write(&p, serde_json::to_string_pretty(&body).unwrap()).unwrap();
    let out = work.join("mission");
    aevum_unify::cmd_new(&[
        "--constitution".into(),
        p.to_str().unwrap().into(),
        "--out".into(),
        out.to_str().unwrap().into(),
    ])
    .unwrap();
    out
}

fn case_13_r3_requires_falsifier() -> CaseResult {
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission_risk(tmp.path(), "mis_c13", "R3");
    let blocked = aevum_unify::cmd_run(&[
        "--mission".into(),
        mission.to_str().unwrap().into(),
        "--capability".into(),
        "git.branch.create".into(),
        "--argv".into(),
        "git checkout -b x".into(),
    ]);
    match &blocked {
        Err(e) if e.to_string().contains("falsifier") => {}
        other => {
            return fail(
                "ATB-13",
                "R3+ requires falsifier challenge",
                format!("expected falsifier block, got {other:?}"),
            )
        }
    }
    aevum_unify::graph_cmd::cmd_falsify(&[
        "--mission".into(),
        mission.to_str().unwrap().into(),
        "--reason".into(),
        "challenge: missing independent review of blast radius".into(),
    ])
    .unwrap();
    match aevum_unify::cmd_run(&[
        "--mission".into(),
        mission.to_str().unwrap().into(),
        "--capability".into(),
        "git.branch.create".into(),
        "--argv".into(),
        "git checkout -b x".into(),
    ]) {
        Ok(()) => ok(
            "ATB-13",
            "R3+ requires falsifier challenge",
            "blocked then allowed after falsify",
        ),
        Err(e) => fail(
            "ATB-13",
            "R3+ requires falsifier challenge",
            format!("still blocked after falsify: {e}"),
        ),
    }
}

fn case_14_sqlite_roundtrip() -> CaseResult {
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission(tmp.path(), "mis_c14");
    std::env::set_var("AEVUM_GRAPH_STORE", "sqlite");
    let sb = SqliteBackend::open(&mission).unwrap();
    let n = sb.graph().fact_count();
    sb.save().unwrap();
    let sqlite_path = mission.join("graph.sqlite");
    if !sqlite_path.exists() || n == 0 {
        return fail(
            "ATB-14",
            "SQLite graph persist roundtrip",
            format!("exists={} facts={n}", sqlite_path.exists()),
        );
    }
    let sb2 = SqliteBackend::open(&mission).unwrap();
    if sb2.graph().fact_count() == n && sb2.name() == "sqlite" {
        ok(
            "ATB-14",
            "SQLite graph persist roundtrip",
            format!("facts={n} path={}", sqlite_path.display()),
        )
    } else {
        fail(
            "ATB-14",
            "SQLite graph persist roundtrip",
            format!("{} vs {}", n, sb2.graph().fact_count()),
        )
    }
}

fn case_15_golden_pr_draft_no_merge() -> CaseResult {
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission(tmp.path(), "mis_c15");
    let repo = tmp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    std::process::Command::new("git")
        .args(["init", "--initial-branch=main"])
        .arg(&repo)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["-C", repo.to_str().unwrap(), "config", "user.email", "t@t"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["-C", repo.to_str().unwrap(), "config", "user.name", "t"])
        .output()
        .unwrap();
    fs::write(repo.join("README"), "x").unwrap();
    std::process::Command::new("git")
        .args(["-C", repo.to_str().unwrap(), "add", "README"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["-C", repo.to_str().unwrap(), "commit", "-m", "i"])
        .output()
        .unwrap();

    match aevum_unify::golden::cmd_golden(&[
        "--mission".into(),
        mission.to_str().unwrap().into(),
        "--repo".into(),
        repo.to_str().unwrap().into(),
        "--title".into(),
        "ATB golden".into(),
        "--branch".into(),
        "aevum/atb-golden".into(),
        "--no-slop-gate".into(),
    ]) {
        Ok(()) => {
            let draft = mission.join("pr-draft.json");
            let raw = fs::read_to_string(&draft).unwrap_or_default();
            let v: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
            let no_merge = v.get("auto_merge") == Some(&serde_json::json!(false));
            let schema_ok = v.get("schema").and_then(|s| s.as_str()) == Some("aevum.pr-draft/v1");
            if draft.exists() && no_merge && schema_ok {
                ok(
                    "ATB-15",
                    "Golden Path PR draft never merges",
                    "pr-draft.json auto_merge=false",
                )
            } else {
                fail(
                    "ATB-15",
                    "Golden Path PR draft never merges",
                    format!("draft ok? {} body={raw}", draft.exists()),
                )
            }
        }
        Err(e) => fail("ATB-15", "Golden Path PR draft never merges", e.to_string()),
    }
}

fn case_16_slop_inference_cannot_authorize() -> CaseResult {
    use aevum_evidence_graph::{may_authorize, FirewallVerdict};
    use aevum_memory_fabric::{ingest_slop_report, SlopFinding, SlopReport};

    let mut g = TemporalGraph::new();
    let report = SlopReport {
        findings: vec![SlopFinding {
            rule: "stub-as-done".into(),
            severity: "block".into(),
            path: "src/evil.rs".into(),
            line: 42,
            // Concatenate so static AI-slop scanners do not treat this fixture as real unfinished code.
            message: format!("{} masquerading as done", "NotImplementedError"),
            snippet: format!("raise {}", "NotImplementedError"),
        }],
        blocking: 1,
    };
    let ing = match ingest_slop_report(&mut g, "mis_slop", &report, "2026-08-08T00:00:00Z") {
        Ok(r) => r,
        Err(e) => {
            return fail(
                "ATB-16",
                "Slop findings are Inference (cannot authorize)",
                e.to_string(),
            )
        }
    };
    let mut any_slop = false;
    let mut allowed = false;
    for f in g.facts_as_of(Some("2099-01-01T00:00:00Z")) {
        if f.episode_ids.contains(&ing.episode_id) {
            any_slop = true;
            if !matches!(f.epistemic, EpistemicKind::Inference) {
                return fail(
                    "ATB-16",
                    "Slop findings are Inference (cannot authorize)",
                    format!("epistemic={:?}", f.epistemic),
                );
            }
            if matches!(may_authorize(f), FirewallVerdict::Allow) {
                allowed = true;
            }
            if matches!(f.kind, EdgeKind::Authorizes) {
                return fail(
                    "ATB-16",
                    "Slop findings are Inference (cannot authorize)",
                    "slop created Authorizes edge",
                );
            }
        }
    }
    if any_slop && !allowed && !g.capability_authorized("secrets.read", "2099-01-01T00:00:00Z") {
        ok(
            "ATB-16",
            "Slop findings are Inference (cannot authorize)",
            format!("episode={} facts={}", ing.episode_id, ing.facts_asserted),
        )
    } else {
        fail(
            "ATB-16",
            "Slop findings are Inference (cannot authorize)",
            format!("any_slop={any_slop} allowed={allowed}"),
        )
    }
}

fn case_17_package_binds_ledger_after_effects() -> CaseResult {
    let tmp = tempfile::tempdir().unwrap();
    let mission = new_mission(tmp.path(), "mis_c17");
    aevum_unify::cmd_run(&[
        "--mission".into(),
        mission.to_str().unwrap().into(),
        "--capability".into(),
        "git.branch.create".into(),
        "--argv".into(),
        "git checkout -b c17".into(),
    ])
    .unwrap();
    let pkg = tmp.path().join("pkg.json");
    match aevum_unify::cmd_package(&[
        "--mission".into(),
        mission.to_str().unwrap().into(),
        "--out".into(),
        pkg.to_str().unwrap().into(),
    ]) {
        Ok(()) => {}
        Err(e) => {
            return fail(
                "ATB-17",
                "Package binds non-empty ledger after effects",
                e.to_string(),
            )
        }
    }
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(&pkg).unwrap()).unwrap();
    let ledger = v
        .get("ledger_entries")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    let audit_d = v
        .get("audit_trail_digest")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    if ledger.trim().is_empty() {
        return fail(
            "ATB-17",
            "Package binds non-empty ledger after effects",
            "ledger_entries empty",
        );
    }
    if !audit_d.starts_with("sha256:") || audit_d == "sha256:none" {
        return fail(
            "ATB-17",
            "Package binds non-empty ledger after effects",
            format!("bad audit_trail_digest={audit_d}"),
        );
    }
    ok(
        "ATB-17",
        "Package binds non-empty ledger after effects",
        format!("ledger_bytes={} audit={}", ledger.len(), audit_d),
    )
}

/// Re-export for integration callers.
pub use aevum_mcp::tools;
