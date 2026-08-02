import { useEffect, useState } from "react";
import { Brand, NavItem, TopBar, ToastStack } from "./components";
import { Link } from "./link";
import { DashboardView } from "./views/Dashboard";
import { MissionsView } from "./views/MissionsView";
import { CouncilView } from "./views/CouncilView";
import { EvidenceView } from "./views/EvidenceView";
import { ActionsView } from "./views/ActionsView";
import { PoliciesView } from "./views/PoliciesView";
import { ApprovalsView } from "./views/ApprovalsView";
import { LedgerView } from "./views/LedgerView";
import { SettingsView } from "./views/SettingsView";
import { GoldenPathView } from "./views/GoldenPathView";
import { PackagesView } from "./views/PackagesView";
import { store } from "./store";

const NAV: Array<{ id: string; label: string; section: string }> = [
  { id: "dashboard", label: "Dashboard", section: "Mission Control" },
  { id: "missions", label: "Missions", section: "Mission Control" },
  { id: "council", label: "Council", section: "Mission Control" },
  { id: "evidence", label: "Evidence", section: "Authority" },
  { id: "actions", label: "Actions", section: "Authority" },
  { id: "policies", label: "Policies", section: "Authority" },
  { id: "approvals", label: "Approvals", section: "Tenant" },
  { id: "ledger", label: "Trust Ledger", section: "Tenant" },
  { id: "settings", label: "Settings", section: "Tenant" },
  { id: "goldenpath", label: "Golden Path", section: "Workflow" },
  { id: "packages", label: "Packages", section: "Workflow" },
];

function useRoute(): [string, (r: string) => void] {
  const [route, setRoute] = useState(() => Link.current());
  useEffect(() => Link.subscribe((r) => setRoute(r)), []);
  return [route, Link.go];
}

function App() {
  const [route, go] = useRoute();
  const [pendingCount] = useState(() => store.state.missions.reduce((s, m) => s + m.approvals.filter((a) => a.decision === "pending").length, 0));
  const [evidenceCount] = useState(() => store.state.missions.reduce((s, m) => s + m.evidence.length, 0));

  let body: React.ReactNode;
  let trail: string[] = [];
  switch (route.split("?")[0]) {
    case "missions": body = <MissionsView />; trail = ["mission control", "missions"]; break;
    case "council": body = <CouncilView />; trail = ["mission control", "council"]; break;
    case "evidence": body = <EvidenceView />; trail = ["authority", "evidence"]; break;
    case "actions": body = <ActionsView />; trail = ["authority", "actions"]; break;
    case "policies": body = <PoliciesView />; trail = ["authority", "policies"]; break;
    case "approvals": body = <ApprovalsView />; trail = ["tenant", "approvals"]; break;
    case "ledger": body = <LedgerView />; trail = ["tenant", "ledger"]; break;
    case "settings": body = <SettingsView />; trail = ["tenant", "settings"]; break;
    case "goldenpath": body = <GoldenPathView />; trail = ["workflow", "golden path"]; break;
    case "packages": body = <PackagesView />; trail = ["workflow", "packages"]; break;
    case "dashboard":
    default:
      body = <DashboardView />; trail = ["mission control", "dashboard"]; break;
  }

  const sections = Array.from(new Set(NAV.map((n) => n.section)));

  return (
    <div className="shell">
      <aside className="sidebar">
        <Brand />
        <div className="sidebar-search" style={{ position: "relative" }}>
          <input placeholder="Search…" />
          <span className="kbd-key" style={{ position: "absolute", right: 18, top: "50%", transform: "translateY(-50%)", pointerEvents: "none" }}>⌘K</span>
        </div>
        {sections.map((section) => (
          <div key={section}>
            <div className="nav-section">{section}</div>
            <div className="nav-list">
              {NAV.filter((n) => n.section === section).map((n) => {
                const count =
                  n.id === "approvals" ? pendingCount :
                  n.id === "evidence" ? evidenceCount :
                  undefined;
                return (
                  <NavItem key={n.id} active={route.startsWith(n.id)} onClick={() => go(n.id)} count={count}>
                    {n.label}
                  </NavItem>
                );
              })}
            </div>
          </div>
        ))}
        <div className="sidebar-foot">
          <div className="signoff">
            <span className="signoff-avatar">WF</span>
            <span className="signoff-name">Winter Fernandes</span>
            <span className="signoff-key">admin</span>
          </div>
          <div style={{ padding: "4px 6px 0", fontFamily: "var(--mono)", color: "var(--text-faint)" }}>
            <span style={{ display: "block" }}>aevum-cli v0.3.0-local</span>
            <span style={{ display: "block" }}>kernel: <span style={{ color: "var(--green)" }}>ok</span> · policy: bound</span>
          </div>
        </div>
      </aside>
      <main className="main">
        <TopBar trail={trail} />
        <div className="surface">{body}</div>
      </main>
      <ToastStack />
    </div>
  );
}

export default App;
