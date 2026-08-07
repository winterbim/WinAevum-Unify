import { PageHeader, useStoreSelector } from "../components";
import { useMemo, useState } from "react";

/**
 * Temporal Decision & Evidence Graph view (P1).
 * Mirrors unify graph status / authorize / falsify semantics in the local store.
 */

type EdgeKind =
  | "authorizes"
  | "supports"
  | "refutes"
  | "relates_to"
  | "derived_from";

interface GraphFact {
  id: string;
  kind: EdgeKind;
  source: string;
  target: string;
  fact: string;
  epistemic: "fact" | "inference" | "hypothesis";
  active: boolean;
}

export function GraphView() {
  const missions = useStoreSelector((s) => s.missions);
  const [missionId, setMissionId] = useState(missions[0]?.id ?? "");
  const m = missions.find((x) => x.id === missionId) ?? missions[0];
  const [cap, setCap] = useState("git.branch.create");
  const [authLog, setAuthLog] = useState<string[]>([]);

  const facts: GraphFact[] = useMemo(() => {
    if (!m) return [];
    const out: GraphFact[] = [];
    // Constitution → capability authorizations derived from executed/denied actions
    out.push({
      id: "fact:constitution",
      kind: "supports",
      source: "claim:constitution",
      target: "mission",
      fact: `constitution ${m.constitution.mission_id} binding`,
      epistemic: "fact",
      active: true,
    });
    for (const a of m.actions) {
      out.push({
        id: `fact:auth:${a.capability}:${a.id}`,
        kind: "authorizes",
        source: "claim:constitution",
        target: `action:${a.capability}`,
        fact: a.status === "denied" ? `DENIED ${a.capability}` : `authorizes ${a.capability}`,
        epistemic: "fact",
        active: a.status !== "denied",
      });
    }
    for (const e of m.evidence) {
      if (e.status === "challenged") {
        out.push({
          id: `fact:challenge:${e.id}`,
          kind: "refutes",
          source: "role:falsifier",
          target: e.id,
          fact: e.challenge?.reason ?? "challenged",
          epistemic: "hypothesis",
          active: true,
        });
      } else {
        out.push({
          id: `fact:ev:${e.id}`,
          kind: "supports",
          source: e.id,
          target: "claim:constitution",
          fact: e.summary || e.title,
          epistemic: "fact",
          active: e.status === "fresh",
        });
      }
    }
    return out;
  }, [m]);

  const authorizing = facts.filter((f) => f.kind === "authorizes" && f.active);
  const challenges = facts.filter((f) => f.kind === "refutes");

  function authorize() {
    if (!m) return;
    setAuthLog((prev) => [
      `✓ authorized ${cap} for ${m.id} (local graph — mirror of unify graph authorize)`,
      ...prev,
    ].slice(0, 8));
  }

  return (
    <>
      <PageHeader
        title="Temporal Graph"
        sub="Decision & Evidence Graph — authorize edges gate real effects. Hypothesis cannot authorize."
        actions={
          <select value={missionId} onChange={(e) => setMissionId(e.target.value)} className="btn">
            {missions.map((mm) => (
              <option key={mm.id} value={mm.id}>
                {mm.id} — {mm.title}
              </option>
            ))}
          </select>
        }
      />

      <div className="card">
        <div className="card-h">
          <h2>Status</h2>
          <span style={{ color: "var(--text-faint)", fontSize: 11, marginLeft: "auto" }}>
            {facts.length} facts · {authorizing.length} authorizes · {challenges.length} challenges
          </span>
        </div>
        <div className="card-b" style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12 }}>
          <Stat label="Episodes (evidence)" value={String(m?.evidence.length ?? 0)} />
          <Stat label="Active authorizes" value={String(authorizing.length)} />
          <Stat label="Falsifier challenges" value={String(challenges.length)} />
        </div>
      </div>

      <div className="card" style={{ marginTop: 14 }}>
        <div className="card-h"><h2>Authorize capability</h2></div>
        <div className="card-b" style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <input
            className="btn"
            style={{ flex: 1, textAlign: "left" }}
            value={cap}
            onChange={(e) => setCap(e.target.value)}
            placeholder="capability e.g. git.pr.create"
          />
          <button className="btn primary" type="button" onClick={authorize}>
            Authorize
          </button>
        </div>
        {authLog.length > 0 ? (
          <div className="card-b" style={{ borderTop: "1px solid var(--line)" }}>
            {authLog.map((l, i) => (
              <div key={i} style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--text-faint)" }}>
                {l}
              </div>
            ))}
          </div>
        ) : null}
      </div>

      <div className="card" style={{ marginTop: 14 }}>
        <div className="card-h"><h2>Facts</h2></div>
        <div className="card-b" style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {facts.map((f) => (
            <div
              key={f.id}
              style={{
                display: "grid",
                gridTemplateColumns: "110px 1fr 90px",
                gap: 10,
                padding: "8px 10px",
                border: "1px solid var(--line)",
                borderRadius: 6,
                background: "var(--bg-2)",
                opacity: f.active ? 1 : 0.45,
              }}
            >
              <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--accent)" }}>{f.kind}</span>
              <span style={{ fontSize: 12 }}>
                <code>{f.source}</code> → <code>{f.target}</code>
                <div style={{ color: "var(--text-faint)", marginTop: 2 }}>{f.fact}</div>
              </span>
              <span className={`pill ${f.epistemic === "fact" ? "allow" : "deny"}`}>{f.epistemic}</span>
            </div>
          ))}
        </div>
      </div>
    </>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ padding: 12, border: "1px solid var(--line)", borderRadius: 6, background: "var(--bg-2)" }}>
      <div style={{ color: "var(--text-faint)", fontSize: 11 }}>{label}</div>
      <div style={{ fontSize: 22, fontWeight: 600, marginTop: 4 }}>{value}</div>
    </div>
  );
}
