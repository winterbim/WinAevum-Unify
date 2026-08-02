/* store imported lazily */
import { KPI, LedgerPreview, PageHeader, RiskPill, StatusPill, useStoreSelector } from "../components";

export function DashboardView() {
  const missions = useStoreSelector((s) => s.missions);
  const ledger = useStoreSelector((s) => s.ledger);
  const approved = missions.filter((m) => m.approvals.some((a) => a.decision === "approved")).length;
  const inFlight = missions.filter((m) => m.status === "executing" || m.status === "constitutional_review").length;
  const denied = missions.reduce((s, m) => s + m.actions.filter((a) => a.status === "denied").length, 0);
  const allowed = missions.reduce((s, m) => s + m.actions.filter((a) => a.status === "executed").length, 0);
  const recentLedger = ledger.slice(-12).reverse();
  return (
    <>
      <PageHeader title="Dashboard" sub="A live overview of the Audit & Authority stack." />
      <div className="kpi-grid">
        <KPI label="Total missions" value={missions.length} />
        <KPI label="Approved" value={approved} delta={`${((approved / Math.max(1, missions.length)) * 100).toFixed(0)}% of total`} tone="up" />
        <KPI label="In flight" value={inFlight} />
        <KPI label="Allow vs Deny" value={`${allowed} / ${denied}`} delta={denied > 0 ? "policy strict" : "policy lenient"} tone={denied > 0 ? "down" : "up"} />
      </div>
      <div className="card" style={{ marginTop: 14 }}>
        <div className="card-h"><h2>Recent ledger activity</h2><span style={{ color: "var(--text-faint)", marginLeft: 8, fontSize: 11 }}>(last 12 events, newest first)</span></div>
        {recentLedger.map((e) => <LedgerPreview key={e.sequence} {...e} />)}
      </div>
      <div className="card" style={{ marginTop: 14 }}>
        <div className="card-h"><h2>Missions under attention</h2><span className="grow" /><span style={{ fontSize: 11, color: "var(--text-faint)" }}>sorted by recent activity</span></div>
        <table className="t">
          <thead><tr><th>Mission</th><th>Title</th><th>Risk</th><th>Status</th><th>Council size</th><th>Approvals</th></tr></thead>
          <tbody>
            {missions.slice(0, 6).map((m) => (
              <tr key={m.id}>
                <td><span style={{ fontFamily: "var(--mono)", color: "var(--text-mute)" }}>{m.id}</span></td>
                <td>{m.title}</td>
                <td><RiskPill risk={m.risk} /></td>
                <td><StatusPill status={m.status} /></td>
                <td><span style={{ fontFamily: "var(--mono)" }}>{m.council.length}</span></td>
                <td>{m.approvals.map((a) => <StatusPill key={a.id} status={a.decision} />)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
