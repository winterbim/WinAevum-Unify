import { store } from "../store";
import { PageHeader, useStoreSelector, StatusPill } from "../components";

export function EvidenceView() {
  const missions = useStoreSelector((s) => s.missions);
  const focus = (typeof window !== "undefined" ? new URLSearchParams(window.location.hash.split("?")[1] ?? "").get("mission") : null);
  const filtered = (focus ? missions.filter((m) => m.id === focus) : missions);
  return (
    <>
      <PageHeader title="Evidence" sub="All evidence attached to all missions. Click challenge to push back." />
      {filtered.map((m) => (
        <div className="card" key={m.id} style={{ marginTop: 14 }}>
          <div className="card-h"><h2>{m.id} — {m.title}</h2><span style={{ color: "var(--text-faint)", fontSize: 11 }}>{m.evidence.length} evidence</span></div>
          <table className="t">
            <thead><tr><th>kind</th><th>title</th><th>digest</th><th>fresh for</th><th>status</th><th></th></tr></thead>
            <tbody>
              {m.evidence.map((e) => (
                <tr key={e.id}>
                  <td><span className="pill">{e.kind}</span></td>
                  <td><div style={{ fontWeight: 600 }}>{e.title}</div><div style={{ color: "var(--text-faint)", fontSize: 11 }}>{e.summary}</div></td>
                  <td><span style={{ fontFamily: "var(--mono)", color: "var(--text-mute)" }}>{e.digest}</span></td>
                  <td><span style={{ fontFamily: "var(--mono)" }}>{Math.round(e.freshness_window / 60)}m</span></td>
                  <td><StatusPill status={e.status} /></td>
                  <td><button className="btn ghost" onClick={() => store.challengeEvidence(m.id, e.id, "Challenge requested by Falsifier")}>Challenge</button></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ))}
    </>
  );
}
