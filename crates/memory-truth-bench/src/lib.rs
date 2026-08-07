//! MemoryTruthBench — adversarial memory integrity cases (offline, no network/LLM).

use aevum_evidence_graph::{
    detect_contradictions, hybrid_search, relate_fact, resolve_parallel_conflicts, seed_entity,
    EdgeKind, Episode, EpisodeSource, EpistemicKind, SearchRecipe, TemporalGraph,
};
use aevum_memory_fabric::{
    assemble, ingest_structured_json, AssemblyRequest, MemoryBackend, MultiTenantStore,
    NativeBackend, TenantScope,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub id: String,
    pub title: String,
    pub passed: bool,
    pub detail: String,
}

pub fn run_all() -> Vec<CaseResult> {
    vec![
        case_01_reference_time_not_today(),
        case_02_as_of_excludes_future(),
        case_03_contradiction_prefer_current(),
        case_04_hypothesis_not_authorizing_in_assembly(),
        case_05_offline_no_network(),
        case_06_rrf_local_ce_ranks_relevant(),
        case_07_sqlite_is_default_backend(),
        case_08_tenant_isolation_no_leak(),
        case_09_multi_tenant_registry_scale(),
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

fn case_01_reference_time_not_today() -> CaseResult {
    let mut g = TemporalGraph::new();
    let ref_t = "2024-01-15T12:00:00Z";
    let body = r#"{
      "facts": [{
        "source": "ent:brent",
        "target": "ent:plex",
        "name": "INSTALLED",
        "fact": "Brent installed Plex on minimind",
        "source_label": "Brent",
        "target_label": "Plex"
      }]
    }"#;
    let report = match ingest_structured_json(&mut g, "mis_mtb", ref_t, body, false) {
        Ok(r) => r,
        Err(e) => return fail("MTB-01", "valid_at = REFERENCE_TIME", e.to_string()),
    };
    let bad = g
        .facts_as_of(Some(ref_t))
        .into_iter()
        .filter(|f| f.episode_ids.contains(&report.episode_id))
        .any(|f| f.valid_at != ref_t);
    if report.reference_time == ref_t && !bad {
        ok(
            "MTB-01",
            "valid_at = REFERENCE_TIME (not today)",
            format!("episode={} at {ref_t}", report.episode_id),
        )
    } else {
        fail(
            "MTB-01",
            "valid_at = REFERENCE_TIME (not today)",
            "fact timestamp drifted from reference_time",
        )
    }
}

fn case_02_as_of_excludes_future() -> CaseResult {
    let mut g = TemporalGraph::new();
    g.add_episode(Episode {
        id: "ep1".into(),
        mission_id: "m".into(),
        group_id: "g".into(),
        source: EpisodeSource::Text,
        content: "past".into(),
        content_digest: None,
        valid_at: "2024-01-01T00:00:00Z".into(),
        created_at: "2024-01-01T00:00:00Z".into(),
        actor_id: None,
    })
    .unwrap();
    g.add_episode(Episode {
        id: "ep2".into(),
        mission_id: "m".into(),
        group_id: "g".into(),
        source: EpisodeSource::Text,
        content: "future".into(),
        content_digest: None,
        valid_at: "2024-03-01T00:00:00Z".into(),
        created_at: "2024-03-01T00:00:00Z".into(),
        actor_id: None,
    })
    .unwrap();
    g.upsert_node(seed_entity("a", "A", "m", "g", "2024-01-01T00:00:00Z"));
    g.upsert_node(seed_entity("b", "B", "m", "g", "2024-01-01T00:00:00Z"));
    g.assert_fact(relate_fact(
        "f_past",
        "a",
        "b",
        "PAST",
        "past fact",
        "ep1",
        "2024-01-01T00:00:00Z",
        "2024-01-01T00:00:00Z",
        "m",
        "g",
        EpistemicKind::Fact,
    ))
    .unwrap();
    g.assert_fact(relate_fact(
        "f_future",
        "a",
        "b",
        "FUTURE",
        "future fact",
        "ep2",
        "2024-03-01T00:00:00Z",
        "2024-03-01T00:00:00Z",
        "m",
        "g",
        EpistemicKind::Fact,
    ))
    .unwrap();
    let as_of = "2024-02-01T00:00:00Z";
    let hits = hybrid_search(&g, &SearchRecipe::new("fact").as_of(as_of).limit(20));
    let active = g.facts_as_of(Some(as_of));
    let leaked =
        active.iter().any(|f| f.id == "f_future") || hits.iter().any(|h| h.fact_id == "f_future");
    let has_past = active.iter().any(|f| f.id == "f_past");
    if !leaked && has_past {
        ok("MTB-02", "as-of excludes future facts", "no future leak")
    } else {
        fail(
            "MTB-02",
            "as-of excludes future facts",
            format!("leaked={leaked} has_past={has_past}"),
        )
    }
}

fn case_03_contradiction_prefer_current() -> CaseResult {
    let mut g = TemporalGraph::new();
    g.add_episode(Episode {
        id: "ep".into(),
        mission_id: "m".into(),
        group_id: "g".into(),
        source: EpisodeSource::Json,
        content: "{}".into(),
        content_digest: Some("sha256:x".into()),
        valid_at: "2026-01-01T00:00:00Z".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        actor_id: None,
    })
    .unwrap();
    g.upsert_node(seed_entity("s", "S", "m", "g", "2026-01-01T00:00:00Z"));
    g.upsert_node(seed_entity("t", "T", "m", "g", "2026-01-01T00:00:00Z"));
    g.assert_fact(relate_fact(
        "f_old",
        "s",
        "t",
        "STATUS_A",
        "status is A",
        "ep",
        "2026-01-01T00:00:00Z",
        "2026-01-01T00:00:00Z",
        "m",
        "g",
        EpistemicKind::Fact,
    ))
    .unwrap();
    g.assert_fact(relate_fact(
        "f_new",
        "s",
        "t",
        "STATUS_B",
        "status is B",
        "ep",
        "2026-02-01T00:00:00Z",
        "2026-02-01T00:00:00Z",
        "m",
        "g",
        EpistemicKind::Fact,
    ))
    .unwrap();
    let as_of = "2026-02-15T00:00:00Z";
    let before = detect_contradictions(&g, as_of);
    let n = resolve_parallel_conflicts(&mut g, as_of).unwrap();
    let active = g.facts_as_of(Some(as_of));
    let old_gone = !active.iter().any(|f| f.id == "f_old");
    let new_live = active.iter().any(|f| f.id == "f_new");
    if !before.is_empty() && n > 0 && old_gone && new_live {
        ok(
            "MTB-03",
            "contradiction prefers current state",
            format!("resolved={n}"),
        )
    } else {
        fail(
            "MTB-03",
            "contradiction prefers current state",
            format!(
                "before={} n={n} old_gone={old_gone} new_live={new_live}",
                before.len()
            ),
        )
    }
}

fn case_04_hypothesis_not_authorizing_in_assembly() -> CaseResult {
    let g = TemporalGraph::seed_for_mission(
        "mis_mtb4",
        "{}",
        "sha256:x",
        &["git.branch.create"],
        "2026-08-01T00:00:00Z",
    )
    .unwrap();
    let dir = std::env::temp_dir().join(format!("mtb4-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("graph.json"),
        serde_json::to_string_pretty(&g.to_snapshot()).unwrap(),
    )
    .unwrap();
    let backend = NativeBackend::open(&dir).unwrap();
    let ctx = assemble(
        &backend,
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
    let hyp_auth = ctx.hits.iter().any(|h| {
        h.hit.may_authorize
            && backend
                .graph()
                .fact(&h.hit.id)
                .map(|f| matches!(f.epistemic, EpistemicKind::Hypothesis))
                .unwrap_or(false)
    });
    let _ = EdgeKind::Authorizes;
    if !hyp_auth && !ctx.authorizing_fact_ids.is_empty() {
        ok(
            "MTB-04",
            "hypothesis cannot authorize via assemble",
            format!("authorizing={}", ctx.authorizing_fact_ids.len()),
        )
    } else if hyp_auth {
        fail(
            "MTB-04",
            "hypothesis cannot authorize via assemble",
            "hypothesis leaked as may_authorize",
        )
    } else {
        fail(
            "MTB-04",
            "hypothesis cannot authorize via assemble",
            "no authorizing facts from seed",
        )
    }
}

fn case_05_offline_no_network() -> CaseResult {
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("EMBEDDING_API_KEY");
    let mut g = TemporalGraph::new();
    let r = ingest_structured_json(
        &mut g,
        "mis_off",
        "2020-06-01T00:00:00Z",
        r#"{"facts":[{"source":"a","target":"b","name":"X","fact":"offline ok"}]}"#,
        false,
    );
    match r {
        Ok(rep) if rep.facts_asserted == 1 => ok(
            "MTB-05",
            "offline: no LLM/network required",
            "ingest+graph ok with env cleared",
        ),
        other => fail(
            "MTB-05",
            "offline: no LLM/network required",
            format!("{other:?}"),
        ),
    }
}

fn case_06_rrf_local_ce_ranks_relevant() -> CaseResult {
    let mut g = TemporalGraph::new();
    g.add_episode(Episode {
        id: "ep".into(),
        mission_id: "m".into(),
        group_id: "g".into(),
        source: EpisodeSource::Text,
        content: "x".into(),
        content_digest: None,
        valid_at: "2026-01-01T00:00:00Z".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        actor_id: None,
    })
    .unwrap();
    g.upsert_node(seed_entity("n1", "N1", "m", "g", "2026-01-01T00:00:00Z"));
    g.upsert_node(seed_entity("n2", "N2", "m", "g", "2026-01-01T00:00:00Z"));
    g.upsert_node(seed_entity("n3", "N3", "m", "g", "2026-01-01T00:00:00Z"));
    g.assert_fact(relate_fact(
        "f_rel",
        "n1",
        "n2",
        "USES",
        "rust toolchain installed for builds",
        "ep",
        "2026-01-01T00:00:00Z",
        "2026-01-01T00:00:00Z",
        "m",
        "g",
        EpistemicKind::Fact,
    ))
    .unwrap();
    g.assert_fact(relate_fact(
        "f_noise",
        "n1",
        "n3",
        "NOTES",
        "weather is cloudy today",
        "ep",
        "2026-01-01T00:00:00Z",
        "2026-01-01T00:00:00Z",
        "m",
        "g",
        EpistemicKind::Fact,
    ))
    .unwrap();
    let hits = hybrid_search(
        &g,
        &SearchRecipe::new("rust toolchain")
            .as_of("2026-01-02T00:00:00Z")
            .limit(5),
    );
    if hits.first().map(|h| h.fact_id.as_str()) == Some("f_rel") {
        ok(
            "MTB-06",
            "RRF+local CE ranks relevant first",
            format!("top={} score={}", hits[0].fact_id, hits[0].score),
        )
    } else {
        fail(
            "MTB-06",
            "RRF+local CE ranks relevant first",
            format!("hits={hits:?}"),
        )
    }
}

fn case_07_sqlite_is_default_backend() -> CaseResult {
    let dir = std::env::temp_dir().join(format!("mtb7-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let g = TemporalGraph::seed_for_mission(
        "mis_mtb7",
        "{}",
        "sha256:x",
        &["git.branch.create"],
        "2026-08-01T00:00:00Z",
    )
    .unwrap();
    std::fs::write(
        dir.join("graph.json"),
        serde_json::to_string_pretty(&g.to_snapshot()).unwrap(),
    )
    .unwrap();
    std::env::remove_var("AEVUM_GRAPH_STORE");
    match aevum_memory_fabric::open_backend(&dir) {
        Ok(b) if b.name() == "sqlite" => ok(
            "MTB-07",
            "SQLite is default memory backend",
            format!("backend={}", b.name()),
        ),
        Ok(b) => fail(
            "MTB-07",
            "SQLite is default memory backend",
            format!("unexpected backend {}", b.name()),
        ),
        Err(e) => fail("MTB-07", "SQLite is default memory backend", e.to_string()),
    }
}

fn case_08_tenant_isolation_no_leak() -> CaseResult {
    let root = std::env::temp_dir().join(format!("mtb8-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let store = match MultiTenantStore::open(&root) {
        Ok(s) => s,
        Err(e) => {
            return fail(
                "MTB-08",
                "tenant isolation (no cross-mission leak)",
                e.to_string(),
            )
        }
    };
    let a = TenantScope::local("mis_mtb8_a");
    let b = TenantScope::local("mis_mtb8_b");

    let mut ga = TemporalGraph::new();
    ga.add_episode(Episode {
        id: "ep_a".into(),
        mission_id: a.mission_id.clone(),
        group_id: a.group_id(),
        source: EpisodeSource::Text,
        content: "a".into(),
        content_digest: None,
        valid_at: "2026-01-01T00:00:00Z".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        actor_id: None,
    })
    .unwrap();
    ga.upsert_node(seed_entity(
        "n1",
        "N1",
        &a.mission_id,
        &a.group_id(),
        "2026-01-01T00:00:00Z",
    ));
    ga.upsert_node(seed_entity(
        "n2",
        "N2",
        &a.mission_id,
        &a.group_id(),
        "2026-01-01T00:00:00Z",
    ));
    ga.assert_fact(relate_fact(
        "f_a",
        "n1",
        "n2",
        "SECRET",
        "alpha-token-unique-aaa",
        "ep_a",
        "2026-01-01T00:00:00Z",
        "2026-01-01T00:00:00Z",
        &a.mission_id,
        &a.group_id(),
        EpistemicKind::Fact,
    ))
    .unwrap();
    store.put_graph(&a, &ga).unwrap();

    let mut gb = TemporalGraph::new();
    gb.add_episode(Episode {
        id: "ep_b".into(),
        mission_id: b.mission_id.clone(),
        group_id: b.group_id(),
        source: EpisodeSource::Text,
        content: "b".into(),
        content_digest: None,
        valid_at: "2026-01-01T00:00:00Z".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        actor_id: None,
    })
    .unwrap();
    gb.upsert_node(seed_entity(
        "n1",
        "N1",
        &b.mission_id,
        &b.group_id(),
        "2026-01-01T00:00:00Z",
    ));
    gb.upsert_node(seed_entity(
        "n2",
        "N2",
        &b.mission_id,
        &b.group_id(),
        "2026-01-01T00:00:00Z",
    ));
    gb.assert_fact(relate_fact(
        "f_b",
        "n1",
        "n2",
        "SECRET",
        "beta-token-unique-bbb",
        "ep_b",
        "2026-01-01T00:00:00Z",
        "2026-01-01T00:00:00Z",
        &b.mission_id,
        &b.group_id(),
        EpistemicKind::Fact,
    ))
    .unwrap();
    store.put_graph(&b, &gb).unwrap();

    let hits_a = store
        .search(
            &a,
            "alpha-token-unique-aaa",
            Some("2026-02-01T00:00:00Z"),
            10,
        )
        .unwrap();
    let leak = store
        .search(
            &a,
            "beta-token-unique-bbb",
            Some("2026-02-01T00:00:00Z"),
            10,
        )
        .unwrap();
    let ok_a = hits_a.iter().any(|h| h.fact_id == "f_a");
    let no_b =
        !leak.iter().any(|h| h.fact_id == "f_b") && !hits_a.iter().any(|h| h.fact_id == "f_b");
    if ok_a && no_b {
        ok(
            "MTB-08",
            "tenant isolation (no cross-mission leak)",
            "mission A search never returns B facts",
        )
    } else {
        fail(
            "MTB-08",
            "tenant isolation (no cross-mission leak)",
            format!("ok_a={ok_a} no_b={no_b} hits_a={hits_a:?} leak={leak:?}"),
        )
    }
}

fn case_09_multi_tenant_registry_scale() -> CaseResult {
    let root = std::env::temp_dir().join(format!("mtb9-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let store = match MultiTenantStore::open(&root) {
        Ok(s) => s,
        Err(e) => {
            return fail(
                "MTB-09",
                "multi-tenant registry holds many missions",
                e.to_string(),
            )
        }
    };
    for i in 0..8 {
        let scope = TenantScope::new("tenant_scale", format!("mis_mtb9_{i}"));
        let mut g = TemporalGraph::new();
        let ep = format!("ep_{i}");
        g.add_episode(Episode {
            id: ep.clone(),
            mission_id: scope.mission_id.clone(),
            group_id: scope.group_id(),
            source: EpisodeSource::Text,
            content: format!("payload {i}"),
            content_digest: None,
            valid_at: "2026-01-01T00:00:00Z".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            actor_id: None,
        })
        .unwrap();
        g.upsert_node(seed_entity(
            "n1",
            "N1",
            &scope.mission_id,
            &scope.group_id(),
            "2026-01-01T00:00:00Z",
        ));
        g.upsert_node(seed_entity(
            "n2",
            "N2",
            &scope.mission_id,
            &scope.group_id(),
            "2026-01-01T00:00:00Z",
        ));
        g.assert_fact(relate_fact(
            &format!("f_{i}"),
            "n1",
            "n2",
            "NOTE",
            &format!("mission-marker-{i}"),
            &ep,
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00Z",
            &scope.mission_id,
            &scope.group_id(),
            EpistemicKind::Fact,
        ))
        .unwrap();
        store.put_graph(&scope, &g).unwrap();
    }
    let n = store.mission_count().unwrap();
    let listed = store.list("tenant_scale").unwrap();
    if n >= 8 && listed.len() == 8 {
        ok(
            "MTB-09",
            "multi-tenant registry holds many missions",
            format!("missions={n} listed={}", listed.len()),
        )
    } else {
        fail(
            "MTB-09",
            "multi-tenant registry holds many missions",
            format!("missions={n} listed={}", listed.len()),
        )
    }
}
