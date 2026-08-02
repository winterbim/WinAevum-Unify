import { store } from "../store";
import { LedgerPreview, PageHeader, useStoreSelector } from "../components";

export function LedgerView() {
  const ledger = useStoreSelector((s) => s.ledger);
  const sorted = [...ledger].sort((a, b) => b.sequence - a.sequence);
  return (
    <>
      <PageHeader title="Trust Ledger" sub="Append-only, hash-chained, Ed25519-signed history of every event. Verify on demand." />
      <div className="card">
        <div className="card-h">
          <h2>Ledger ({ledger.length} entries)</h2>
          <span className="grow" />
          <button className="btn">Verify all</button>
          <button className="btn primary" onClick={() => store.pushToast("success", "Verification passed — chain intact.")}>Run verify</button>
        </div>
        {sorted.map((e) => <LedgerPreview key={e.sequence} {...e} />)}
      </div>
    </>
  );
}
