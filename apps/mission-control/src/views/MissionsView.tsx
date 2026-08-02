/* store imported lazily */
import { ConstitutionSummary, PageHeader, RiskPill, StatusPill, useStoreSelector } from "../components";
import type { Mission } from "../types";
import { Link } from "../link";

export function MissionsView() {
  const missions = useStoreSelector((s) => s.missions);
  const focus = useFocusFromRoute();
  const focused = missions.find((m) => m.id === focus) ?? missions[0];
  return (
    <>
      <PageHeader title="Missions" sub="8 active missions. Click a row to inspect its constitution, council, evidence, and ledger." />
      <div className="card">
        <div className="card-h"><h2>All missions</h2><span style={{ color: "var(--text-faint)", fontSize: 11 }}>click to focus on the right panel</span></div>
        <table className="t">
          <thead><tr><th>id</th><th>title</th><th>risk</th><th>status</th><th>council</th><th>evidence</th><th>actions</th></tr></thead>
          <tbody>
            {missions.map((m) => (
              <tr key={m.id} onClick={() => focusMission(m.id)} style={{ cursor: "pointer", background: focused?.id === m.id ? "rgba(99,102,241,0.06)" : undefined }}>
                <td><span style={{ fontFamily: "var(--mono)" }}>{m.id}</span></td>
                <td>{m.title}</td>
                <td><RiskPill risk={m.risk} /></td>
                <td><StatusPill status={m.status} /></td>
                <td>{m.council.length}</td>
                <td>{m.evidence.length}</td>
                <td>{m.actions.length}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {focused ? <MissionInspector mission={focused} /> : null}
    </>
  );
}

export function focusMission(id: string) {
  const h = (typeof window !== "undefined" ? window.location.hash : "#missions") + "";
  const base = h.split("?")[0] || "missions";
  if (typeof window !== "undefined") window.location.hash = `${base}?focus=${id}`;
}

export function useFocusFromRoute(): string | null {
  if (typeof window === "undefined") return null;
  const h = window.location.hash.replace(/^#/, "");
  const q = h.split("?")[1];
  if (!q) return null;
  const params = new URLSearchParams(q);
  return params.get("focus");
}

function MissionInspector({ mission }: { mission: Mission }) {
  return (
    <div className="card" style={{ marginTop: 14 }}>
      <div className="card-h">
        <h2>{mission.id} — {mission.title}</h2>
        <span style={{ color: "var(--text-faint)", fontFamily: "var(--mono)" }}>v{mission.constitution.version} · {mission.tenant}</span>
        <span style={{ marginLeft: 12 }}><RiskPill risk={mission.risk} /></span>
        <span style={{ marginLeft: 6 }}><StatusPill status={mission.status} /></span>
        <span style={{ marginLeft: 8 }}>
          <button className="btn" onClick={() => Link.go(`actions?mission=${mission.id}`)}>Run action</button>
        </span>
        <span style={{ marginLeft: 6 }}>
          <button className="btn primary" onClick={() => Link.go(`evidence?mission=${mission.id}`)}>Inspect evidence</button>
        </span>
      </div>
      <div className="card-b">
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14 }}>
          <div className="card">
            <div className="card-h"><h2>Constitution</h2></div>
            <ConstitutionSummary c={mission.constitution} />
          </div>
          <div className="card">
            <div className="card-h"><h2>Council</h2><span style={{ color: "var(--text-faint)", fontSize: 11 }}>{mission.council.length} members</span></div>
            <div className="council-grid">
              {mission.council.map((c) => (
                <div className="council-card" key={c.agent_id}>
                  <div className="ident"><span className="avatar">{c.role.slice(0, 2).toUpperCase()}</span>{c.role}</div>
                  <div className="role">{c.provider} · {c.family}</div>
                  <div className="meta"><span>domains: {(c.domains.length === 1 && c.domains[0] === "*") ? "*" : c.domains.join(",")}</span></div>
                  <div className="meta"><span>reasoning: {c.reasoning_budget_tokens}t</span></div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
