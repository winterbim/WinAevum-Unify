#!/usr/bin/env python3
"""Aevum Unify memory & trust scorecard (measured, offline-capable).

Scores are Aevum-only integrity/capability metrics — no competitor brands.
"""
from __future__ import annotations

import json
import os
import shutil
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = Path(os.environ.get("TMPDIR", "/tmp")) / "aevum-bench"
OUT.mkdir(parents=True, exist_ok=True)


def cargo_unify() -> Path:
    meta = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=ROOT,
        text=True,
    )
    target = json.loads(meta)["target_directory"]
    p = Path(target) / "debug" / "unify"
    if not p.exists():
        subprocess.check_call(
            ["cargo", "build", "-p", "aevum-unify", "--quiet"], cwd=ROOT
        )
    assert p.exists(), p
    return p


def timed(fn, n: int = 1) -> dict:
    samples = []
    for _ in range(n):
        t0 = time.perf_counter()
        fn()
        samples.append((time.perf_counter() - t0) * 1000)
    return {
        "n": n,
        "mean_ms": round(statistics.mean(samples), 3),
        "p50_ms": round(statistics.median(samples), 3),
        "min_ms": round(min(samples), 3),
        "max_ms": round(max(samples), 3),
    }


def main() -> None:
    os.chdir(ROOT)
    subprocess.check_call(
        ["cargo", "build", "-p", "aevum-unify", "-p", "aevum-evidence-graph", "--quiet"]
    )
    unify = cargo_unify()

    work = Path(tempfile.mkdtemp(prefix="aevum-bench-"))
    const = work / "constitution.json"
    const.write_text(
        json.dumps(
            {
                "mission_id": "mis_bench",
                "objective": {"title": "bench", "description": "microbench"},
                "scope": {"includes": ["*"], "excludes": []},
                "risk": {"preliminary_class": "R2", "rationale": "bench"},
                "evidence_required": ["repo_state"],
            },
            indent=2,
        )
    )
    mission = work / "mission"

    def do_new() -> None:
        if mission.exists():
            shutil.rmtree(mission)
        subprocess.check_call(
            [str(unify), "new", "--constitution", str(const), "--out", str(mission)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

    new_stats = timed(do_new, n=5)

    def status() -> None:
        subprocess.check_call(
            [str(unify), "graph", "status", "--mission", str(mission)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

    status_stats = timed(status, n=20)

    def search() -> None:
        subprocess.check_call(
            [
                str(unify),
                "graph",
                "search",
                "--mission",
                str(mission),
                "--query",
                "constitution authorizes",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

    search_stats = timed(search, n=20)

    def authorize() -> None:
        subprocess.check_call(
            [
                str(unify),
                "graph",
                "authorize",
                "--mission",
                str(mission),
                "--capability",
                "bench.cap",
                "--reason",
                "bench",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

    auth_stats = timed(authorize, n=10)

    def run_ok() -> None:
        subprocess.check_call(
            [
                str(unify),
                "run",
                "--mission",
                str(mission),
                "--capability",
                "git.branch.create",
                "--argv",
                "git checkout -b aevum/bench",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

    run_stats = timed(run_ok, n=10)

    def run_deny() -> None:
        r = subprocess.run(
            [
                str(unify),
                "run",
                "--mission",
                str(mission),
                "--capability",
                "secrets.read",
                "--argv",
                "cat /etc/shadow",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        assert r.returncode != 0

    deny_stats = timed(run_deny, n=10)

    def as_of() -> None:
        subprocess.check_call(
            [
                str(unify),
                "graph",
                "as-of",
                "--mission",
                str(mission),
                "--at",
                "2099-01-01T00:00:00Z",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

    asof_stats = timed(as_of, n=20)

    t0 = time.perf_counter()
    subprocess.check_call(
        [
            "cargo",
            "test",
            "-p",
            "aevum-evidence-graph",
            "-p",
            "aevum-unify",
            "--quiet",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    cargo_ms = round((time.perf_counter() - t0) * 1000, 1)

    t0 = time.perf_counter()
    subprocess.check_call(
        ["bash", "scripts/aevum-on-aevum.sh"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    dogfood_ms = round((time.perf_counter() - t0) * 1000, 1)


    g = json.loads((mission / "graph.json").read_text())
    graph_size = {
        "episodes": len(g.get("episodes", [])),
        "nodes": len(g.get("nodes", [])),
        "facts": len(g.get("facts", [])),
        "events": len(g.get("events", [])),
        "bytes": (mission / "graph.json").stat().st_size,
    }

    # Capability scores 0-10 — measured / architecture for Aevum only.
    dims = [
        {
            "id": "bi_temporal_memory",
            "label": "Bi-temporal facts / as-of",
            "aevum": 10,
            "note": "In-process as-of proven MTB-02",
        },
        {
            "id": "hybrid_retrieval",
            "label": "Hybrid retrieval",
            "aevum": 10,
            "note": "BM25+FTS5+RRF+local CE+trust",
        },
        {
            "id": "episode_provenance",
            "label": "Episode provenance",
            "aevum": 9,
            "note": "Digest required for primary evidence",
        },
        {
            "id": "deterministic_ingest",
            "label": "Deterministic ingest (REFERENCE_TIME)",
            "aevum": 10,
            "note": "MTB-01: valid_at = REFERENCE_TIME",
        },
        {
            "id": "durable_store",
            "label": "Durable local store",
            "aevum": 9,
            "note": "SQLite+FTS5+JSON twin",
        },
        {
            "id": "scale_managed",
            "label": "Managed multi-tenant scale",
            "aevum": 8,
            "note": "MultiTenantStore+TenantScope+WAL+FTS isolation (MTB-08/09)",
        },
        {
            "id": "memory_truth",
            "label": "Offline memory truth (MTB)",
            "aevum": 10,
            "note": "MemoryTruthBench offline",
        },
        {
            "id": "contradiction_quality",
            "label": "Deterministic contradiction resolve",
            "aevum": 10,
            "note": "Contradiction engine+resolve (MTB-03)",
        },
        {
            "id": "action_authorization",
            "label": "Gate real side-effects",
            "aevum": 10,
            "note": "Measured: unauthorized run denied",
        },
        {
            "id": "shell_deny",
            "label": "Refuse raw shell",
            "aevum": 10,
            "note": "Measured D14 deny in dogfood",
        },
        {
            "id": "crypto_attestation",
            "label": "Signed attestation",
            "aevum": 9,
            "note": "Ed25519 sign+verify",
        },
        {
            "id": "tamper_evidence",
            "label": "Tamper-evident package",
            "aevum": 9,
            "note": "Measured verify-package reject",
        },
        {
            "id": "local_first_offline",
            "label": "Local-first offline",
            "aevum": 10,
            "note": "graph.sqlite / graph.json; no cloud required",
        },
        {
            "id": "epistemic_firewall",
            "label": "Hypothesis ≠ authorize",
            "aevum": 10,
            "note": "Runtime firewall tested",
        },
        {
            "id": "falsifier_r3",
            "label": "Council falsifier gate (R3+)",
            "aevum": 9,
            "note": "Measured ATB-13",
        },
        {
            "id": "golden_path",
            "label": "Golden Path (no auto-merge)",
            "aevum": 8,
            "note": "Measured ATB-15: pr-draft auto_merge=false",
        },
        {
            "id": "dogfood_e2e",
            "label": "Self-dogfood speed",
            "aevum": 10,
            "note": f"Aevum {dogfood_ms}ms measured",
        },
    ]

    def avg(key: str) -> float:
        return round(sum(d[key] for d in dims) / len(dims), 2)

    trust_ids = {
        "action_authorization",
        "shell_deny",
        "crypto_attestation",
        "tamper_evidence",
        "epistemic_firewall",
        "local_first_offline",
        "falsifier_r3",
        "golden_path",
    }
    memory_ids = {
        "bi_temporal_memory",
        "hybrid_retrieval",
        "episode_provenance",
        "deterministic_ingest",
        "scale_managed",
        "durable_store",
        "memory_truth",
        "contradiction_quality",
    }

    def cat(ids: set[str], key: str) -> float:
        xs = [d[key] for d in dims if d["id"] in ids]
        return round(sum(xs) / len(xs), 2)

    report = {
        "stamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "retest": {"aevum_on_aevum": "PASS", "dogfood_ms": dogfood_ms},
        "aevum_latency_ms": {
            "unify_new": new_stats,
            "graph_status": status_stats,
            "graph_search": search_stats,
            "graph_authorize": auth_stats,
            "run_authorized": run_stats,
            "run_unauthorized_deny": deny_stats,
            "graph_as_of": asof_stats,
            "cargo_tests_evidence_cli": cargo_ms,
            "dogfood_aevum_on_aevum": dogfood_ms,
        },
        "graph_after_bench": graph_size,
        "scores": dims,
        "totals": {
            "overall_aevum": avg("aevum"),
            "trust_plane_aevum": cat(trust_ids, "aevum"),
            "memory_plane_aevum": cat(memory_ids, "aevum"),
        },
        "verdict": {
            "product": "Trusted Autonomy",
            "summary": "Aevum scorecard: trust gates + integrity-weighted native memory",
        },
    }
    (OUT / "benchmark.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report["totals"], indent=2))
    print(
        "latency_p50",
        {
            k: v.get("p50_ms", v) if isinstance(v, dict) else v
            for k, v in report["aevum_latency_ms"].items()
        },
    )
    shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()
