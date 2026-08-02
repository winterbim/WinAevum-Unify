import { store } from "../store";
import { PageHeader, StatusPill, useStoreSelector } from "../components";

export function ApprovalsView() {
  const missions = useStoreSelector((s) => s.missions);
  return (
    <>
      <PageHeader title="Approvals" sub="All approval decisions across the tenant. Default-deny applies." />
      <div className="card">
        {missions.flatMap((m) => m.approvals.map((a) => ({ a, m }))).map(({ a, m }) => (
          <div className="approval-row" key={a.id}>
            <span className="pill">{a.decision === "pending" ? "review" : a.decision}</span>
            <div>
              <div className="title">{m.title}</div>
              <div className="meta"><span>{a.id} · {m.id}</span> · <span>{a.reviewer}</span> · <span>{a.decided_at}</span></div>
              <div style={{ fontSize: 11, color: "var(--text-faint)", marginTop: 4 }}>{a.reason}</div>
            </div>
            <span style={{ display: "flex", gap: 6 }}>
              <button className="btn primary" onClick={() => store.decideApproval(m.id, a.id, "approved", "Reviewed and approved")}>Approve</button>
              <button className="btn danger" onClick={() => store.decideApproval(m.id, a.id, "rejected", "Risk exceeds tenant policy")}>Reject</button>
            </span>
            <span><StatusPill status={a.decision} /></span>
          </div>
        ))}
      </div>
    </>
  );
}
