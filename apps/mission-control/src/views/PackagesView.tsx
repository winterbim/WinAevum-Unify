import { store } from "../store";
import { PageHeader, useStoreSelector } from "../components";

interface Bundle {
  id: string;
  missionId: string;
  title: string;
  created: string;
  digest: string;
  auditDigest: string;
  slopDigest: string;
  ledgerNonEmpty: boolean;
  sizeKb: number;
  status: "verified" | "pending" | "tampered";
}

const SEED: Bundle[] = [
  {
    id: "pkg_hub_01",
    missionId: "mis_01",
    title: "Hub evidence (ledger+audit+slop)",
    created: "2026-08-08T12:00:00Z",
    digest: "sha256:aevum…phare",
    auditDigest: "sha256:audit…01",
    slopDigest: "sha256:slop…clean",
    ledgerNonEmpty: true,
    sizeKb: 18,
    status: "verified",
  },
  {
    id: "pkg_parallel_a",
    missionId: "mis_01",
    title: "Parallel variant A",
    created: "2026-08-08T12:05:00Z",
    digest: "sha256:par…a",
    auditDigest: "sha256:none",
    slopDigest: "sha256:none",
    ledgerNonEmpty: false,
    sizeKb: 12,
    status: "pending",
  },
  {
    id: "pkg_parallel_b",
    missionId: "mis_01",
    title: "Parallel variant B",
    created: "2026-08-08T12:05:00Z",
    digest: "sha256:par…b",
    auditDigest: "sha256:none",
    slopDigest: "sha256:none",
    ledgerNonEmpty: false,
    sizeKb: 12,
    status: "pending",
  },
];

export function PackagesView() {
  const missions = useStoreSelector((s) => s.missions);
  const merged = SEED.map((b) => ({
    ...b,
    title: missions.find((m) => m.id === b.missionId)?.title ?? b.title,
  }));
  return (
    <>
      <PageHeader
        title="Evidence Packages"
        sub="Hub packages bind ledger + audit_trail_digest + slop_report_digest + temporal_graph_digest. Never empty after effects."
      />
      <div className="card">
        <div className="card-h">
          <h2>{merged.length} packages</h2>
          <span className="grow" />
          <button
            className="btn"
            onClick={() =>
              store.pushToast(
                "success",
                "CLI: unify package --mission … && unify parallel --constitution … --out …",
              )
            }
          >
            + Build / parallel
          </button>
        </div>
        <table className="t">
          <thead>
            <tr>
              <th>id</th>
              <th>mission</th>
              <th>title</th>
              <th>ledger</th>
              <th>audit</th>
              <th>slop</th>
              <th>digest</th>
              <th>status</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {merged.map((b) => (
              <tr key={b.id}>
                <td>
                  <span style={{ fontFamily: "var(--mono)" }}>{b.id}</span>
                </td>
                <td>
                  <span style={{ fontFamily: "var(--mono)", color: "var(--text-mute)" }}>
                    {b.missionId}
                  </span>
                </td>
                <td>{b.title}</td>
                <td>
                  {b.ledgerNonEmpty ? (
                    <span className="pill allow">bound</span>
                  ) : (
                    <span className="pill queued">empty</span>
                  )}
                </td>
                <td>
                  <span style={{ fontFamily: "var(--mono)", fontSize: 11 }}>{b.auditDigest}</span>
                </td>
                <td>
                  <span style={{ fontFamily: "var(--mono)", fontSize: 11 }}>{b.slopDigest}</span>
                </td>
                <td>
                  <span style={{ fontFamily: "var(--mono)" }}>{b.digest}</span>
                </td>
                <td>
                  {b.status === "verified" ? (
                    <span className="pill allow">verified</span>
                  ) : b.status === "pending" ? (
                    <span className="pill queued">pending</span>
                  ) : (
                    <span className="pill deny">tampered</span>
                  )}
                </td>
                <td>
                  <button
                    className="btn ghost"
                    onClick={() =>
                      store.pushToast("success", "Package verified — auto_merge=false")
                    }
                  >
                    Verify
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="card" style={{ marginTop: 14 }}>
        <div className="card-h">
          <h2>Parallel compare (best-of-N)</h2>
        </div>
        <div className="card-b" style={{ color: "var(--text-mute)", fontSize: 13 }}>
          <code>unify parallel --constitution c.json --out /tmp/aevum-parallel --n 3</code> writes{" "}
          <code>compare.json</code>. Pick a winner manually — never auto-merge.
        </div>
      </div>
    </>
  );
}
