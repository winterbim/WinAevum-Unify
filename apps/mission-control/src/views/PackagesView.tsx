import { store } from "../store";
import { PageHeader, useStoreSelector } from "../components";

interface Bundle { id: string; missionId: string; title: string; created: string; digest: string; sizeKb: number; status: "verified" | "pending" | "tampered"; }

const SEED: Bundle[] = [
  { id: "pkg_mis_01", missionId: "mis_01", title: "Refactor identities", created: "2026-08-02T09:30:00Z", digest: "sha256:9af1…b402", sizeKb: 14, status: "verified" },
  { id: "pkg_mis_04", missionId: "mis_04", title: "Bump attestation version", created: "2026-08-01T18:00:00Z", digest: "sha256:dde4…018c", sizeKb: 22, status: "verified" },
  { id: "pkg_mis_05", missionId: "mis_05", title: "Add council diversity gate", created: "2026-08-01T11:00:00Z", digest: "sha256:5b1f…cc20", sizeKb: 17, status: "pending" },
  { id: "pkg_pilot", missionId: "mis_07", title: "Refresh ledger", created: "2026-07-30T22:00:00Z", digest: "sha256:1033…9911", sizeKb: 9, status: "tampered" },
];

export function PackagesView() {
  const missions = useStoreSelector((s) => s.missions);
  const merged = SEED.map((b) => ({ ...b, title: missions.find((m) => m.id === b.missionId)?.title ?? b.title }));
  return (
    <>
      <PageHeader title="Evidence Packages" sub="Verifiable bundles of a mission's constitution, ledger and attestations. SHA-256 content-addressed." />
      <div className="card">
        <div className="card-h"><h2>{merged.length} packages</h2><span className="grow" /><button className="btn">+ Build new package</button></div>
        <table className="t">
          <thead><tr><th>id</th><th>mission</th><th>title</th><th>created</th><th>digest</th><th>size</th><th>status</th><th></th></tr></thead>
          <tbody>
            {merged.map((b) => (
              <tr key={b.id}>
                <td><span style={{ fontFamily: "var(--mono)" }}>{b.id}</span></td>
                <td><span style={{ fontFamily: "var(--mono)", color: "var(--text-mute)" }}>{b.missionId}</span></td>
                <td>{b.title}</td>
                <td><span style={{ fontFamily: "var(--mono)", color: "var(--text-faint)" }}>{b.created}</span></td>
                <td><span style={{ fontFamily: "var(--mono)" }}>{b.digest}</span></td>
                <td>{b.sizeKb} KB</td>
                <td>{b.status === "verified" ? <span className="pill allow">verified</span> : b.status === "pending" ? <span className="pill queued">pending</span> : <span className="pill deny">tampered</span>}</td>
                <td>{b.status === "tampered" ? <button className="btn danger" onClick={() => store.pushToast("warning", "Tampered package blocked: re-bundle required.")}>Quarantine</button> : <button className="btn ghost" onClick={() => store.pushToast("success", "Package verified end-to-end.")}>Verify</button>}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
