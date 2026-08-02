/* store imported lazily */
import { PageHeader, RiskPill, StatusPill, useStoreSelector } from "../components";
import { useState } from "react";

interface GoldenStep {
  id: string;
  label: string;
  detail: string;
  status: "done" | "active" | "queued" | "blocked";
}

const STEPS_TEMPLATE: Omit<GoldenStep, "status">[] = [
  { id: "1", label: "Constitution drafted", detail: "scope + risk + evidence requirements" },
  { id: "2", label: "Council assembled",  detail: "5–7 agents with diversity gate" },
  { id: "3", label: "Evidence attached",   detail: "tests, lint, dependency audit, repo state" },
  { id: "4", label: "Branch created",      detail: "aevum/<slug> (no shell, argv typed)" },
  { id: "5", label: "Patch tested",        detail: "all tests green, lint clean" },
  { id: "6", label: "Review",              detail: "Falsifier + Verifier + Guardian" },
  { id: "7", label: "Approval",            detail: "Risk-weighted: R3+ requires human approval" },
  { id: "8", label: "PR opened (no merge)", detail: "Edge submits PR; never auto-merged" },
];

export function GoldenPathView() {
  const missions = useStoreSelector((s) => s.missions);
  const [missionId, setMissionId] = useState(missions[0]?.id ?? "");
  const m = missions.find((x) => x.id === missionId) ?? missions[0];
  const completedActionIds = new Set((m?.actions ?? []).filter((a) => a.status === "executed").map((a) => a.capability));
  const deniedActionIds = new Set((m?.actions ?? []).filter((a) => a.status === "denied").map((a) => a.capability));

  const steps: GoldenStep[] = STEPS_TEMPLATE.map((s, idx) => {
    let status: GoldenStep["status"] = "queued";
    if (m) {
      if (idx === 0) status = "done";
      if (idx === 1) status = m.council.length > 0 ? "done" : "active";
      if (idx === 2) status = m.evidence.length >= 3 ? "done" : m.council.length > 0 ? "active" : "queued";
      if (idx === 3) status = completedActionIds.has("git.branch.create") ? "done" : m.evidence.length >= 3 ? "active" : "queued";
      if (idx === 4) status = m.actions.some((a) => a.capability === "git.commit" && a.status === "executed") ? "done" : completedActionIds.has("git.branch.create") ? "active" : "queued";
      if (idx === 5) status = m.actions.length > 3 ? "done" : m.status === "approved" || m.status === "executing" ? "active" : "queued";
      if (idx === 6) status = m.approvals.every((a) => a.decision === "approved") && m.approvals.length > 0 ? "done" : m.risk === "R3" || m.risk === "R2" ? "active" : "queued";
      if (idx === 7) status = m.status === "completed" ? "done" : m.approvals.every((a) => a.decision === "approved") ? "active" : "queued";
    }
    if (deniedActionIds.has("git.branch.create")) status = "blocked";
    return { ...s, status };
  });

  const pr = m ? `https://github.com/winterbim/aevum-unify/pull/new/${(m.title.toLowerCase().replace(/[^a-z0-9]+/g, "-").slice(0, 24))}` : "";

  return (
    <>
      <PageHeader
        title="Golden Path"
        sub="Eight steps from constitution to PR. Never auto-merged."
        actions={
          <select value={missionId} onChange={(e) => setMissionId(e.target.value)} className="btn">
            {missions.map((mm) => <option key={mm.id} value={mm.id}>{mm.id} — {mm.title}</option>)}
          </select>
        }
      />

      <div className="card">
        <div className="card-h">
          <h2>{m?.id} — {m?.title}</h2>
          {m ? <><span style={{ marginLeft: 8 }}><RiskPill risk={m.risk} /></span><span style={{ marginLeft: 6 }}><StatusPill status={m.status} /></span></> : null}
          <span style={{ color: "var(--text-faint)", fontSize: 11, marginLeft: "auto" }}>risk-weighted</span>
        </div>
        <div className="steps" role="list">
          {steps.map((s, i) => (
            <div key={s.id} className={`step ${s.status === "active" ? "active" : ""} ${s.status === "done" ? "done" : ""}`} role="listitem">
              <span className="dot">{s.status === "done" ? "✓" : i + 1}</span>
              <div>
                <div style={{ fontWeight: 600 }}>{s.label}</div>
                <div style={{ color: "var(--text-faint)", fontSize: 11 }}>{s.detail}</div>
              </div>
            </div>
          ))}
        </div>
        <div className="card-b" style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <button className="btn primary" disabled={steps[7].status !== "active"}>{steps[7].status === "done" ? "PR already opened" : "Open PR (no merge)"}</button>
          {m && pr ? <a className="btn ghost" href={pr} target="_blank" rel="noreferrer">Mock PR #{m.id.replace(/[^0-9]/g, "")}</a> : null}
          <span style={{ color: "var(--text-faint)", fontSize: 11, marginLeft: 6 }}>PR is opened by the Edge agent; humans review and merge only after the Gate votes Allow.</span>
        </div>
      </div>

      <div className="card" style={{ marginTop: 14 }}>
        <div className="card-h"><h2>Policy invariants enforced on this path</h2></div>
        <div className="card-b" style={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 10 }}>
          <Invariant k="D04" v="Writes against `main` are denied." ok />
          <Invariant k="D14" v="`sh -c` execution paths are denied." ok />
          <Invariant k="D16" v="argv with shell metacharacters `;&|>$` is rejected." ok />
          <Invariant k="D17" v="Constitution drift before merge transitions to blocked." ok={(m?.status ?? "draft") !== "completed"} />
          <Invariant k="M3"  v="Council must include an in-domain verifier for R3+." ok={(m?.council ?? []).length >= 5} />
          <Invariant k="M7"  v="Privileges must be re-issued before TTL on R3+." ok={(m?.approvals ?? []).every((a) => a.decision === "approved" || a.decision === "pending")} />
          <Invariant k="M8"  v="Edge PR is opened by an agent; humans merge." ok={true} />
          <Invariant k="M9"  v="Provider diversity gate excludes co-routed producer/verifier." ok={(() => { if (!m) return false; const ps = new Set((m.council ?? []).filter((c) => c.role === "producer" || c.role === "verifier").map((c) => c.provider)); return ps.size >= 2; })()} />
        </div>
      </div>
    </>
  );
}

function Invariant({ k, v, ok }: { k: string; v: string; ok: boolean }) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "60px 1fr 28px", gap: 12, padding: "10px 12px", border: "1px solid var(--line)", borderRadius: 6, background: "var(--bg-2)", alignItems: "center" }}>
      <span style={{ fontFamily: "var(--mono)", color: "var(--accent)" }}>{k}</span>
      <span style={{ color: "var(--text)" }}>{v}</span>
      <span className={`pill ${ok ? "allow" : "deny"}`}>{ok ? "✓" : "✗"}</span>
    </div>
  );
}
