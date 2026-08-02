import { useEffect, useMemo, useRef, useState } from "react";
import { store } from "./store";
import type { LedgerEntry, MissionConstitution, RiskClass } from "./types";
import { Link } from "./link";

function icon(path: string, size = 16) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.6} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={path} />
    </svg>
  );
}

export function Brand() {
  return (
    <div className="brand">
      <div className="brand-mark" aria-hidden="true">AV</div>
      <span className="brand-name">Aevum Unify</span>
      <span className="brand-meta">v0.3.0-local</span>
    </div>
  );
}


export function useStoreSelector<T>(selector: (s: typeof store.state) => T): T {
  const [, force] = useState(0);
  useEffect(() => store.subscribe(() => force((x) => x + 1)), []);
  return selector(store.state);
}

export function NavItem(props: { active: boolean; onClick: () => void; children: React.ReactNode; count?: number }) {
  return (
    <button className={props.active ? "active" : ""} onClick={props.onClick}>
      <span>{props.children}</span>
      {props.count !== undefined ? <span className="badge count">{props.count}</span> : null}
    </button>
  );
}

export function Crumbs(props: { trail: string[] }) {
  return (
    <div className="crumbs">
      <span className="breadcrumbs">workspace</span>
      {props.trail.map((t, i) => (
        <span key={i}><span className="breadcrumbs">/</span> <strong style={{ color: "var(--text-faint)", fontWeight: 500 }}>{t}</strong></span>
      ))}
    </div>
  );
}

export function TopBar(props: { trail: string[] }) {
  const [pOpen, setPOpen] = useState(false);
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPOpen((v) => !v);
      }
      if (e.key === "Escape") setPOpen(false);
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  return (
    <div className="topbar">
      <Crumbs trail={props.trail} />
      <div className="topbar-actions">
        <div className="search-wrap">
          <span className="ico">{icon("<circle cx='11' cy='11' r='7'/><path d='m21 21-4.3-4.3' />")}</span>
          <input className="search" placeholder="Search missions, agents, ledger…" onFocus={() => setPOpen(true)} onKeyDown={(e) => { if (e.key === "Enter" || e.key === "/") { (e.currentTarget as HTMLInputElement).blur(); setPOpen(true); } }} />
          <span className="kbd-key" style={{ position: "absolute", right: 12, top: "50%", transform: "translateY(-50%)" }}>⌘K</span>
        </div>
        <button className="btn ghost" onClick={() => { store.reset(); }}>{icon("<polyline points='3 12 3 6 9 6'/><line x1='3' y1='12' x2='21' y2='12'/>")} Reset</button>
        <button className="btn primary" onClick={() => { setPOpen(true); }}>{icon("<line x1='12' y1='5' x2='12' y2='19'/><line x1='5' y1='12' x2='19' y2='12'/>")} New</button>
      </div>
      {pOpen ? <CommandPalette onClose={() => setPOpen(false)} /> : null}
    </div>
  );
}

function CommandPalette(props: { onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const s = useStoreSelector((s) => s);
  const items = useMemo(() => {
    const all = [
      { label: "Go to Dashboard", meta: "surface", go: () => Link.go("dashboard") },
      { label: "Go to Missions", meta: "surface", go: () => Link.go("missions") },
      { label: "Go to Council", meta: "surface", go: () => Link.go("council") },
      { label: "Go to Evidence", meta: "surface", go: () => Link.go("evidence") },
      { label: "Go to Actions", meta: "surface", go: () => Link.go("actions") },
      { label: "Go to Policies", meta: "surface", go: () => Link.go("policies") },
      { label: "Go to Approvals", meta: "surface", go: () => Link.go("approvals") },
      { label: "Go to Ledger", meta: "surface", go: () => Link.go("ledger") },
      { label: "Go to Settings", meta: "surface", go: () => Link.go("settings") },
      ...s.missions.map((m) => ({ label: `Open mission ${m.id} — ${m.title}`, meta: `risk ${m.risk}`, go: () => Link.go(`missions?focus=${m.id}`) })),
      { label: "Reset demo data", meta: "action", go: () => store.reset() },
    ];
    if (!query.trim()) return all;
    const q = query.toLowerCase();
    return all.filter((i) => i.label.toLowerCase().includes(q) || i.meta.toLowerCase().includes(q));
  }, [query, s.missions]);
  useEffect(() => { ref.current?.querySelector<HTMLInputElement>("input")?.focus(); }, []);
  function commit() {
    const item = items[active];
    if (item) { item.go(); props.onClose(); }
  }
  return (
    <div className="palette-bg" onMouseDown={(e) => e.target === e.currentTarget && props.onClose()}>
      <div className="palette" ref={ref}>
        <div className="head">
          <span style={{ color: "var(--text-faint)" }}>{icon("M12 5v14M5 12h14")}</span>
          <input
            placeholder="Jump to…"
            value={query}
            onChange={(e) => { setQuery(e.target.value); setActive(0); }}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") { setActive((a) => Math.min(items.length - 1, a + 1)); e.preventDefault(); }
              else if (e.key === "ArrowUp") { setActive((a) => Math.max(0, a - 1)); e.preventDefault(); }
              else if (e.key === "Enter") { commit(); }
            }}
          />
          <span className="kbd-key">esc</span>
        </div>
        <div className="list">
          {items.length === 0 ? <div className="empty">No matches</div> : items.map((it, i) => (
            <div key={i} className={`row ${i === active ? "active" : ""}`} onMouseEnter={() => setActive(i)} onClick={commit}>
              <div>
                <div className="label">{it.label}</div>
                <div className="meta">{it.meta}</div>
              </div>
              <div className="key">↵</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export function ToastStack() {
  const toasts = useStoreSelector((s) => s.toasts);
  return (
    <div className="toast-stack">
      {toasts.map((t) => (
        <div key={t.id} className={`toast ${t.kind}`}>
          {t.kind === "success" ? "✓" : t.kind === "error" ? "✗" : t.kind === "warning" ? "⏸" : "⏵"} {t.message}
        </div>
      ))}
    </div>
  );
}

export function RiskPill({ risk }: { risk: RiskClass }) {
  return <span className={`pill ${risk.toLowerCase()}`}><span className="dot" />{risk}</span>;
}

export function StatusPill({ status }: { status: string }) {
  const cls = ["draft", "queued"].includes(status) ? "queued" : (["completed", "approved", "allow"].includes(status) ? "allow" : (["denied", "failed", "rejected"].includes(status) ? "deny" : "review"));
  return <span className={`pill ${cls}`}>{status}</span>;
}

export function KPI(props: { label: string; value: string | number; delta?: string; tone?: "up" | "down" | undefined }) {
  return (
    <div className="card kpi">
      <div className="label">{props.label}</div>
      <div className="value">{props.value}</div>
      {props.delta ? <div className={`delta ${props.tone ?? ""}`}>{props.delta}</div> : null}
    </div>
  );
}

export function PageHeader(props: { title: string; sub?: string; actions?: React.ReactNode }) {
  return (
    <div className="page-head">
      <h1>{props.title}</h1>
      {props.sub ? <span className="sub">{props.sub}</span> : null}
      <div className="actions">{props.actions}</div>
    </div>
  );
}

export function LedgerPreview({ sequence, event_type, payload, occurred_at, actor_id, digest }: LedgerEntry) {
  return (
    <div className="ledger-entry">
      <span className="seq">#{sequence.toString().padStart(3, "0")}</span>
      <div className="ev">
        <span className="type">{event_type}</span>
        <span className="meta"> · {actor_id} · {occurred_at}</span>
        <div className="meta">{JSON.stringify(payload).slice(0, 100)}</div>
      </div>
      <span className="dig">{digest.slice(0, 18)}…</span>
    </div>
  );
}

export function ConstitutionSummary({ c }: { c: MissionConstitution }) {
  return (
    <pre className="code" aria-label="Constitution JSON">
{c ? null : null}
{JSON.stringify(c, null, 2)}
    </pre>
  );
}

export function AddMissionDialog(props: { onClose: () => void }) {
  const [title, setTitle] = useState("");
  const [summary, setSummary] = useState("");
  const [risk, setRisk] = useState<RiskClass>("R2");
  const [domains, setDomains] = useState("code");
  function submit() {
    if (!title.trim()) return;
    store.createMission({ title, summary, risk, domains: domains.split(",").map((d) => d.trim()).filter(Boolean) });
    props.onClose();
  }
  return (
    <div className="palette-bg" onMouseDown={(e) => e.target === e.currentTarget && props.onClose()}>
      <div className="palette" style={{ width: 560 }}>
        <div className="head">
          <span style={{ color: "var(--text-faint)" }}>{icon("M12 5v14M5 12h14")}</span>
          <input readOnly value="Create new mission" style={{ flex: 1 }} />
        </div>
        <div style={{ padding: 14 }}>
          <div className="field"><label>Title</label><input value={title} onChange={(e) => setTitle(e.target.value)} placeholder="e.g. Wire capability engine to attestation pipeline" /></div>
          <div className="field"><label>Summary</label><textarea value={summary} onChange={(e) => setSummary(e.target.value)} placeholder="What does success look like?" /></div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12 }}>
            <div className="field"><label>Risk</label>
              <select value={risk} onChange={(e) => setRisk(e.target.value as RiskClass)}>
                {["R0","R1","R2","R3","R4"].map((r) => <option key={r}>{r}</option>)}
              </select>
            </div>
            <div className="field"><label>Domains</label><input value={domains} onChange={(e) => setDomains(e.target.value)} placeholder="code, docs" /></div>
            <div className="field"><label>Recovery</label>
              <select defaultValue="delete_branch"><option>delete_branch</option><option>revert_merge</option><option>rotate_secrets</option></select>
            </div>
          </div>
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 12 }}>
            <button className="btn ghost" onClick={props.onClose}>Cancel</button>
            <button className="btn primary" onClick={submit}>Create</button>
          </div>
        </div>
      </div>
    </div>
  );
}

export function MissionPicker() {
  const missionId = useStoreSelector((s) => s.selectedMissionId);
  const missions = useStoreSelector((s) => s.missions);
  return (
    <select value={missionId ?? ""} onChange={(e) => store.selectMission(e.target.value)} className="btn" style={{ minWidth: 240 }}>
      {missions.map((m) => (<option key={m.id} value={m.id}>{m.id} · {m.title}</option>))}
    </select>
  );
}
