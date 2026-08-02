/* store imported lazily */
import { PageHeader, useStoreSelector } from "../components";

export function CouncilView() {
  const missions = useStoreSelector((s) => s.missions);
  const map = new Map<string, { provider: string; family: string; role: string; missions: Set<string>; domains: string[] }>();
  for (const m of missions) {
    for (const c of m.council) {
      const k = `${c.provider}/${c.family}/${c.role}`;
      const e = map.get(k);
      if (e) { e.missions.add(m.id); }
      else { map.set(k, { provider: c.provider, family: c.family, role: c.role, missions: new Set([m.id]), domains: [...c.domains] }); }
    }
  }
  const rows = Array.from(map.values()).sort((a, b) => b.missions.size - a.missions.size);
  return (
    <>
      <PageHeader title="Council" sub="All agents currently committed to at least one mission — provenance & diversity at a glance." />
      <div className="card">
        <div className="card-h"><h2>Providers / families / roles</h2><span style={{ color: "var(--text-faint)", fontSize: 11 }}>dedup by role, sorted by assignment count</span></div>
        <table className="t">
          <thead><tr><th>Provider</th><th>Family</th><th>Role</th><th>Domains</th><th>Missions</th></tr></thead>
          <tbody>
            {rows.map((r) => (
              <tr key={`${r.provider}/${r.family}/${r.role}`}>
                <td><span style={{ fontFamily: "var(--mono)", color: "var(--accent)" }}>{r.provider}</span></td>
                <td>{r.family}</td>
                <td><span className="pill">{r.role}</span></td>
                <td><span style={{ fontFamily: "var(--mono)", color: "var(--text-mute)" }}>{r.domains.join(",")}</span></td>
                <td>{Array.from(r.missions).length}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
