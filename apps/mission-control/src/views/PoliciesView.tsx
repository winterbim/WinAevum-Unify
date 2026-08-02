/* store imported lazily */
import { PageHeader, useStoreSelector } from "../components";

export function PoliciesView() {
  const policy = useStoreSelector((s) => s.policy);
  return (
    <>
      <PageHeader title="Policies" sub="The active Rego-inspired policy bundle and its evaluation rules." />
      <div className="card">
        <div className="card-h">
          <h2>{policy.version}</h2>
          <span className="grow" />
          <span style={{ fontFamily: "var(--mono)", color: "var(--accent)" }}>{policy.bundle_digest}</span>
        </div>
        {policy.rules.map((r, i) => (
          <div className="policy-row" key={r.id}>
            <span className="num">{(i + 1).toString().padStart(2, "0")}</span>
            <div className="desc">
              <div className="name">{r.id}</div>
              <div className="reason">{r.reason}</div>
            </div>
            <span className={`pill ${r.effect}`}>{r.effect}</span>
          </div>
        ))}
      </div>
    </>
  );
}
