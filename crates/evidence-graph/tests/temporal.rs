//! Temporal context graph tests.

use aevum_evidence_graph::{
    hybrid_search, may_authorize, relate_fact, seed_entity, EdgeKind, Episode, EpisodeSource,
    EpistemicKind, Fact, FirewallVerdict, SearchRecipe, TemporalError, TemporalGraph,
};

fn ep(id: &str, attested: bool) -> Episode {
    Episode {
        id: id.to_string(),
        mission_id: "mis_demo".into(),
        group_id: "grp_demo".into(),
        source: if attested {
            EpisodeSource::Attested
        } else {
            EpisodeSource::Text
        },
        content: format!("payload {id}"),
        content_digest: if attested {
            Some(format!("sha256:{id}"))
        } else {
            None
        },
        valid_at: "2026-08-02T10:00:00Z".into(),
        created_at: "2026-08-02T10:00:01Z".into(),
        actor_id: Some("agt_recon".into()),
    }
}

#[test]
fn episode_provenance_required_for_facts() {
    let mut g = TemporalGraph::new();
    g.upsert_node(seed_entity(
        "n1",
        "Repo",
        "mis_demo",
        "grp",
        "2026-08-02T10:00:00Z",
    ));
    g.upsert_node(seed_entity(
        "n2",
        "Rust",
        "mis_demo",
        "grp",
        "2026-08-02T10:00:00Z",
    ));
    let f = relate_fact(
        "f1",
        "n1",
        "n2",
        "USES_LANGUAGE",
        "repo uses Rust",
        "ep_missing",
        "2026-08-02T10:00:00Z",
        "2026-08-02T10:00:01Z",
        "mis_demo",
        "grp",
        EpistemicKind::Fact,
    );
    let err = g.assert_fact(f).unwrap_err();
    assert!(matches!(err, TemporalError::UnknownEpisode(_)));
}

#[test]
fn bi_temporal_as_of_query() {
    let mut g = TemporalGraph::new();
    g.add_episode(ep("ep1", true)).unwrap();
    g.upsert_node(seed_entity(
        "n1",
        "Policy",
        "mis_demo",
        "grp",
        "2026-08-02T09:00:00Z",
    ));
    g.upsert_node(seed_entity(
        "n2",
        "R4",
        "mis_demo",
        "grp",
        "2026-08-02T09:00:00Z",
    ));

    let f = relate_fact(
        "f_old",
        "n1",
        "n2",
        "BLOCKS",
        "policy blocks R4 without approval",
        "ep1",
        "2026-08-01T00:00:00Z",
        "2026-08-02T10:00:01Z",
        "mis_demo",
        "grp",
        EpistemicKind::Fact,
    );
    g.assert_fact(f).unwrap();

    // Superseding fact at 11:00
    g.add_episode(ep("ep2", true)).unwrap();
    let f2 = relate_fact(
        "f_new",
        "n1",
        "n2",
        "BLOCKS",
        "policy blocks R4 and R5 without approval",
        "ep2",
        "2026-08-02T11:00:00Z",
        "2026-08-02T11:00:01Z",
        "mis_demo",
        "grp",
        EpistemicKind::Fact,
    );
    g.assert_fact(f2).unwrap();

    // Old fact invalidated at new valid_at
    assert!(!g.is_fact_valid_at("f_old", "2026-08-02T11:00:00Z").unwrap());
    assert!(g.is_fact_valid_at("f_new", "2026-08-02T11:30:00Z").unwrap());
    // Historical: before supersession, old fact still true
    assert!(g.is_fact_valid_at("f_old", "2026-08-02T10:30:00Z").unwrap());

    let at_1030 = g.facts_as_of(Some("2026-08-02T10:30:00Z"));
    assert_eq!(at_1030.len(), 1);
    assert_eq!(at_1030[0].id, "f_old");

    let at_1130 = g.facts_as_of(Some("2026-08-02T11:30:00Z"));
    assert_eq!(at_1130.len(), 1);
    assert_eq!(at_1130[0].id, "f_new");
}

#[test]
fn hypothesis_cannot_authorize_action() {
    let mut g = TemporalGraph::new();
    g.add_episode(ep("ep1", true)).unwrap();
    g.upsert_node(seed_entity(
        "claim",
        "maybe safe",
        "mis_demo",
        "grp",
        "2026-08-02T10:00:00Z",
    ));
    g.upsert_node(seed_entity(
        "action",
        "git.push",
        "mis_demo",
        "grp",
        "2026-08-02T10:00:00Z",
    ));

    let mut f = relate_fact(
        "f_auth",
        "claim",
        "action",
        "AUTHORIZES",
        "hypothesis authorizes push",
        "ep1",
        "2026-08-02T10:00:00Z",
        "2026-08-02T10:00:01Z",
        "mis_demo",
        "grp",
        EpistemicKind::Hypothesis,
    );
    f.kind = EdgeKind::Authorizes;
    f.fact_digest = Some("sha256:auth".into());

    let err = g.assert_fact(f).unwrap_err();
    assert!(matches!(err, TemporalError::Rejected(_)));
}

#[test]
fn fact_with_digest_may_authorize() {
    let mut g = TemporalGraph::new();
    g.add_episode(ep("ep1", true)).unwrap();
    g.upsert_node(seed_entity(
        "claim",
        "tests pass",
        "mis_demo",
        "grp",
        "2026-08-02T10:00:00Z",
    ));
    g.upsert_node(seed_entity(
        "action",
        "git.pr.create",
        "mis_demo",
        "grp",
        "2026-08-02T10:00:00Z",
    ));

    let mut f = relate_fact(
        "f_auth",
        "claim",
        "action",
        "AUTHORIZES",
        "confirmed tests authorize PR",
        "ep1",
        "2026-08-02T10:00:00Z",
        "2026-08-02T10:00:01Z",
        "mis_demo",
        "grp",
        EpistemicKind::Fact,
    );
    f.kind = EdgeKind::Authorizes;
    f.fact_digest = Some("sha256:auth".into());
    g.assert_fact(f).unwrap();

    let v = g
        .authorization_allowed("claim", "action", "2026-08-02T10:05:00Z")
        .unwrap();
    assert_eq!(v, FirewallVerdict::Allow);
}

#[test]
fn hybrid_keyword_search_finds_fact() {
    let mut g = TemporalGraph::new();
    g.add_episode(ep("ep1", true)).unwrap();
    g.upsert_node(seed_entity(
        "n1",
        "Toolchain",
        "mis_demo",
        "grp",
        "2026-08-02T10:00:00Z",
    ));
    g.upsert_node(seed_entity(
        "n2",
        "Rust",
        "mis_demo",
        "grp",
        "2026-08-02T10:00:00Z",
    ));
    g.assert_fact(relate_fact(
        "f1",
        "n1",
        "n2",
        "USES",
        "repository uses Rust 1.82 toolchain",
        "ep1",
        "2026-08-02T10:00:00Z",
        "2026-08-02T10:00:01Z",
        "mis_demo",
        "grp",
        EpistemicKind::Fact,
    ))
    .unwrap();

    let hits = hybrid_search(
        &g,
        &SearchRecipe::new("rust toolchain").as_of("2026-08-02T10:05:00Z"),
    );
    assert!(!hits.is_empty());
    assert_eq!(hits[0].fact_id, "f1");
}

#[test]
fn event_journal_records_invalidation() {
    let mut g = TemporalGraph::new();
    g.add_episode(ep("ep1", true)).unwrap();
    g.upsert_node(seed_entity(
        "a",
        "A",
        "mis_demo",
        "grp",
        "2026-08-02T10:00:00Z",
    ));
    g.upsert_node(seed_entity(
        "b",
        "B",
        "mis_demo",
        "grp",
        "2026-08-02T10:00:00Z",
    ));
    g.assert_fact(relate_fact(
        "f1",
        "a",
        "b",
        "REL",
        "a relates to b",
        "ep1",
        "2026-08-02T10:00:00Z",
        "2026-08-02T10:00:01Z",
        "mis_demo",
        "grp",
        EpistemicKind::Fact,
    ))
    .unwrap();
    g.invalidate_fact("f1", "2026-08-02T12:00:00Z", "2026-08-02T12:00:01Z")
        .unwrap();
    assert!(g.event_log().iter().any(
        |e| matches!(e, aevum_evidence_graph::GraphEvent::FactInvalidated { id, .. } if id == "f1")
    ));
}

#[test]
fn may_authorize_rejects_empty_provenance() {
    let f = Fact {
        id: "x".into(),
        kind: EdgeKind::Authorizes,
        source_node_id: "a".into(),
        target_node_id: "b".into(),
        name: "AUTH".into(),
        fact: "x".into(),
        epistemic: EpistemicKind::Fact,
        episode_ids: vec![],
        valid_at: "2026-08-02T10:00:00Z".into(),
        invalid_at: None,
        created_at: "2026-08-02T10:00:00Z".into(),
        expired_at: None,
        fact_digest: Some("sha256:x".into()),
        group_id: "g".into(),
        mission_id: "m".into(),
    };
    assert!(matches!(may_authorize(&f), FirewallVerdict::Deny(_)));
}

#[test]
fn provenance_coverage_counts_attested_episodes() {
    let mut g = TemporalGraph::new();
    g.add_episode(ep("ep_att", true)).unwrap();
    g.add_episode(ep("ep_txt", false)).unwrap();
    g.upsert_node(seed_entity(
        "a",
        "A",
        "mis_demo",
        "grp",
        "2026-08-02T10:00:00Z",
    ));
    g.upsert_node(seed_entity(
        "b",
        "B",
        "mis_demo",
        "grp",
        "2026-08-02T10:00:00Z",
    ));
    let mut f = relate_fact(
        "f1",
        "a",
        "b",
        "REL",
        "mixed provenance",
        "ep_att",
        "2026-08-02T10:00:00Z",
        "2026-08-02T10:00:01Z",
        "mis_demo",
        "grp",
        EpistemicKind::Fact,
    );
    f.episode_ids.push("ep_txt".into());
    g.assert_fact(f).unwrap();
    let cov = g.provenance_coverage("f1").unwrap();
    assert!((cov - 0.5).abs() < 1e-9);
}
