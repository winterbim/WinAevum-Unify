use aevum_evidence_graph::{EdgeKind, EpistemicKind, TemporalGraph};
use aevum_memory_fabric::{
    assemble, ingest_remote_as_inference, promote_to_authorize, AssemblyRequest, Embedder,
    MemoryBackend, NativeBackend, RemoteFact,
};

fn seeded(dir: &std::path::Path) -> NativeBackend {
    let caps = ["git.branch.create", "process.exec.argv"];
    let g = TemporalGraph::seed_for_mission(
        "mis_fab",
        r#"{"mission_id":"mis_fab"}"#,
        "sha256:deadbeef",
        &caps,
        "2026-08-07T20:00:00Z",
    )
    .unwrap();
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join("graph.json");
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&g.to_snapshot()).unwrap(),
    )
    .unwrap();
    NativeBackend::open(dir).unwrap()
}

#[test]
fn native_roundtrip_persist() {
    let tmp = tempfile::tempdir().unwrap();
    let b = seeded(tmp.path());
    assert_eq!(b.name(), "native");
    assert!(b
        .graph()
        .capability_authorized("git.branch.create", "2026-08-07T21:00:00Z"));
    b.save().unwrap();
    let b2 = NativeBackend::open(tmp.path()).unwrap();
    assert_eq!(b2.graph().fact_count(), b.graph().fact_count());
}

#[test]
fn assembly_prefers_authorizing_facts_for_capability() {
    let tmp = tempfile::tempdir().unwrap();
    let b = seeded(tmp.path());
    let ctx = assemble(
        &b,
        &AssemblyRequest {
            query: "constitution authorizes".into(),
            as_of: Some("2026-08-07T21:00:00Z".into()),
            intended_capability: Some("git.branch.create".into()),
            limit: 5,
            include_remote: false,
            mission_id: None,
        },
    )
    .unwrap();
    assert!(!ctx.authorizing_fact_ids.is_empty());
    assert!(ctx.hits.iter().any(|h| h.hit.may_authorize));
    assert!(ctx.assembly_score > 0.0);
}

#[test]
fn remote_inference_cannot_authorize_until_promoted() {
    let tmp = tempfile::tempdir().unwrap();
    let mut b = seeded(tmp.path());
    let remotes = vec![RemoteFact {
        uuid: "g-1".into(),
        fact: "repo uses dangerous shell".into(),
        name: "SHELL_HINT".into(),
        valid_at: Some("2026-08-07T20:00:00Z".into()),
        invalid_at: None,
        group_id: Some("aevum".into()),
    }];
    let ids = ingest_remote_as_inference(b.graph_mut(), "mis_fab", &remotes).unwrap();
    assert_eq!(ids.len(), 1);
    let f = b.graph().fact(&ids[0]).unwrap();
    assert!(matches!(f.epistemic, EpistemicKind::Inference));
    // Attempting to use as authorizes directly would need EdgeKind::Authorizes —
    // inference relates_to must not unlock capability.
    assert!(!b
        .graph()
        .capability_authorized("secrets.read", "2026-08-07T21:00:00Z"));

    let auth_id = promote_to_authorize(
        b.graph_mut(),
        "mis_fab",
        &ids[0],
        "bench.promoted",
        r#"{"verified":true,"observation":"repo uses dangerous shell"}"#,
    )
    .unwrap();
    assert!(b
        .graph()
        .capability_authorized("bench.promoted", "2026-08-07T21:00:00Z"));
    let af = b.graph().fact(&auth_id).unwrap();
    assert!(matches!(af.kind, EdgeKind::Authorizes));
    assert!(matches!(af.epistemic, EpistemicKind::Fact));
    b.save().unwrap();
}

#[test]
fn sqlite_migrates_from_json_and_roundtrips() {
    let tmp = tempfile::tempdir().unwrap();
    let _ = seeded(tmp.path());
    let sb = aevum_memory_fabric::SqliteBackend::open(tmp.path()).unwrap();
    assert_eq!(sb.name(), "sqlite");
    assert!(sb.graph().fact_count() > 0);
    sb.save().unwrap();
    assert!(tmp.path().join("graph.sqlite").exists());
    let sb2 = aevum_memory_fabric::SqliteBackend::open(tmp.path()).unwrap();
    assert_eq!(sb2.graph().fact_count(), sb.graph().fact_count());
}

#[test]
fn hashing_embedder_is_deterministic_and_search_uses_it() {
    let emb = aevum_memory_fabric::HashingEmbedder::new(64);
    let a = emb.embed(&["rust toolchain authorize".into()]).unwrap();
    let b = emb.embed(&["rust toolchain authorize".into()]).unwrap();
    assert_eq!(a, b);
    assert_eq!(a[0].len(), 64);

    let tmp = tempfile::tempdir().unwrap();
    let backend = seeded(tmp.path());
    // Fill node embeddings then semantic search
    let mut g =
        aevum_evidence_graph::TemporalGraph::from_snapshot(backend.graph().to_snapshot()).unwrap();
    let n = aevum_memory_fabric::ensure_node_embeddings(&mut g, &emb).unwrap();
    assert!(n > 0);
    let hits = aevum_memory_fabric::semantic_hybrid_search(
        &g,
        "constitution authorizes git",
        Some("2026-08-07T21:00:00Z"),
        5,
        &emb,
    )
    .unwrap();
    assert!(!hits.is_empty());
}

#[test]
fn fts_candidates_after_sqlite_save() {
    let tmp = tempfile::tempdir().unwrap();
    let _ = seeded(tmp.path());
    let sb = aevum_memory_fabric::SqliteBackend::open(tmp.path()).unwrap();
    sb.save().unwrap();
    let cands = sb
        .fts_candidates("constitution authorizes", Some("2026-08-07T21:00:00Z"), 10)
        .unwrap();
    assert!(!cands.is_empty(), "FTS should return candidates");
}

#[test]
fn deterministic_ingest_uses_reference_time() {
    let mut g = aevum_evidence_graph::TemporalGraph::new();
    let ref_t = "2019-03-01T00:00:00Z";
    let r = aevum_memory_fabric::ingest_structured_json(
        &mut g,
        "mis_x",
        ref_t,
        r#"{"facts":[{"source":"a","target":"b","name":"USES","fact":"a uses b"}]}"#,
        false,
    )
    .unwrap();
    assert_eq!(r.reference_time, ref_t);
    for f in g.facts_as_of(Some(ref_t)) {
        if f.episode_ids.contains(&r.episode_id) {
            assert_eq!(f.valid_at, ref_t);
        }
    }
}
