import { useState } from "react";
import { store } from "../store";
import { PageHeader, RiskPill, StatusPill, useStoreSelector } from "../components";

export function ActionsView() {
  const missions = useStoreSelector((s) => s.missions);
  const focus = (typeof window !== "undefined" ? new URLSearchParams(window.location.hash.split("?")[1] ?? "").get("mission") : null);
  const target = (focus ? missions.find((m) => m.id === focus) : missions[0]);
  const [capability, setCapability] = useState("git.branch.create");
  const [argv, setArgv] = useState("git checkout -b aevum/sec-fix");
  return (
    <>
      <PageHeader title="Actions" sub="Compose, sign and run Action Attestations against the Sentinel Kernel." />
      {target ? (
        <div className="card">
          <div className="card-h"><h2>Compose for {target.id} — {target.title}</h2><span style={{ color: "var(--text-faint)", fontSize: 11 }}>policy bundle: <code style={{ color: "var(--accent)" }}>{store.state.policy.bundle_digest.slice(0, 22)}…</code></span></div>
          <div className="card-b">
            <div style={{ display: "grid", gridTemplateColumns: "240px 1fr", gap: 14 }}>
              <div className="card">
                <div className="card-h"><h2>Capabilities</h2><span style={{ color: "var(--text-faint)", fontSize: 11 }}>tap to load</span></div>
                {[
                  "git.branch.create",
                  "git.commit",
                  "fs.read",
                  "fs.write",
                  "deployment.promote",
                  "evidence.attest",
                  "sh.execute",
                ].map((c) => (
                  <button key={c} className="btn ghost" style={{ width: "100%", justifyContent: "flex-start", marginBottom: 6 }} onClick={() => setCapability(c)}>
                    <RiskPill risk={"R0"} /> <span style={{ fontFamily: "var(--mono)", marginLeft: 6 }}>{c}</span>
                  </button>
                ))}
              </div>
              <div>
                <div className="field"><label>Capability</label>
                  <input value={capability} onChange={(e) => setCapability(e.target.value)} />
                </div>
                <div className="field"><label>argv (shell-safe)</label>
                  <input value={argv} onChange={(e) => setArgv(e.target.value)} placeholder="git checkout -b aevum/sec-fix" />
                </div>
                <button className="btn primary" onClick={() => {
                  const parts = argv.split(/\s+/).filter(Boolean);
                  store.signAndRun(target.id, capability, parts);
                }}>Sign &amp; Run</button>
                <button className="btn" style={{ marginLeft: 6 }} onClick={() => { setCapability("sh.execute"); setArgv("sh -c 'rm -rf /'"); }}>Inject a deny</button>
              </div>
            </div>
          </div>
        </div>
      ) : null}
      <div className="card" style={{ marginTop: 14 }}>
        <div className="card-h"><h2>Recent attestations</h2></div>
        <table className="t">
          <thead><tr><th>mission</th><th>capability</th><th>resource</th><th>risk</th><th>policy</th><th>status</th><th>signature</th></tr></thead>
          <tbody>
            {(target ? [target, ...missions.filter((m) => m.id !== target.id)].flatMap((m) => m.actions.map((a) => ({ a, m }))) : []).slice(0, 20).map(({ a, m }) => (
              <tr key={a.id}>
                <td><span style={{ fontFamily: "var(--mono)", color: "var(--text-mute)" }}>{m.id}</span></td>
                <td><span style={{ fontFamily: "var(--mono)" }}>{a.capability}</span></td>
                <td><span style={{ color: "var(--text-mute)" }}>{a.resource}</span></td>
                <td><RiskPill risk={a.risk_class} /></td>
                <td><StatusPill status={a.policy_decision.effect} /> <span style={{ color: "var(--text-faint)", fontFamily: "var(--mono)", fontSize: 11 }}>{a.policy_decision.rule_id}</span></td>
                <td><StatusPill status={a.status} /></td>
                <td><span style={{ fontFamily: "var(--mono)", color: "var(--text-faint)" }}>{a.signature_preview}</span></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
