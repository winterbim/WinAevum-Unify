import { store } from "../store";
import { PageHeader, useStoreSelector } from "../components";

export function SettingsView() {
  const policy = useStoreSelector((s) => s.policy);
  const missions = useStoreSelector((s) => s.missions);
  return (
    <>
      <PageHeader title="Settings" sub="Tenant configuration. Local-first: data is held in your browser's localStorage." />
      <div className="card">
        <div className="card-h"><h2>Tenant</h2></div>
        <div className="card-b" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14 }}>
          <div className="field"><label>Tenant id</label><input value="ten_local" readOnly /></div>
          <div className="field"><label>Workspace root</label><input value="/home/wina/aevum unifiy/aevum-unify" readOnly /></div>
          <div className="field"><label>Authority</label><input value="spiffe://local.aevum/ledger-authority" readOnly /></div>
          <div className="field"><label>Signing alg</label><input value="ed25519" readOnly /></div>
        </div>
      </div>
      <div className="card" style={{ marginTop: 14 }}>
        <div className="card-h"><h2>Bundle digest</h2></div>
        <div className="card-b">
          <pre className="code" style={{ borderRadius: "var(--radius-l)" }}>{policy.bundle_digest}</pre>
        </div>
      </div>
      <div className="card" style={{ marginTop: 14 }}>
        <div className="card-h"><h2>Danger zone</h2></div>
        <div className="card-b" style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <button className="btn danger" onClick={() => store.reset()}>Reset all local data</button>
          <span style={{ color: "var(--text-faint)", fontSize: 11 }}>Resets the in-browser store to seed. (Missions currently in store: {missions.length})</span>
        </div>
      </div>
    </>
  );
}
