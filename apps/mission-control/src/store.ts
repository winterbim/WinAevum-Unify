/**
 * Reactive in-memory store for the Mission Control demo.
 *
 * - Persists to localStorage so reloads don't lose data.
 * - Exposes actions that mutate the store. All mutations are routed through
 *   `setState` so React rerenders and audit log entries are deterministic.
 * - Performs policy evaluation locally (Rego-style rules from seedPolicyBundle)
 *   so the UI is faithful to the canonical contract.
 */

import { seedLedger, seedMissions, seedPolicyBundle } from "./seed";
import type {
  ActionAttestation,
  CouncilMember,
  LedgerEntry,
  Mission,
  MissionConstitution,
  PolicyBundle,
  PolicyEffect,
  RiskClass,
  Toast,
} from "./types";

const KEY = "aevum.unify.state.v1";

function uuid(prefix: string): string { return `${prefix}_${Math.random().toString(36).slice(2, 10).toLowerCase()}`; }

function now(): string { return new Date("2026-08-02T12:00:00Z").toISOString(); }
function hashHex(input: string): string {
  // Stable, in-browser synthetic digest (NOT cryptographic — only used for UX).
  let h = 5381;
  for (let i = 0; i < input.length; i++) h = (h * 33 + input.charCodeAt(i)) >>> 0;
  return ("00000000" + h.toString(16)).slice(-8);
}

export interface State {
  missions: Mission[];
  ledger: LedgerEntry[];
  policy: PolicyBundle;
  toasts: Toast[];
  selectedMissionId: string | null;
  ledgerSeq: number; // counter used when appending new entries
}

function defaultState(): State {
  return {
    missions: seedMissions.map((m) => ({ ...m, actions: [...m.actions], evidence: [...m.evidence], council: [...m.council] })),
    ledger: [...seedLedger],
    policy: seedPolicyBundle,
    toasts: [],
    selectedMissionId: seedMissions[0]?.id ?? null,
    ledgerSeq: seedLedger.length + 1,
  };
}

function load(): State {
  if (typeof window === "undefined") return defaultState();
  try {
    const raw = window.localStorage.getItem(KEY);
    if (!raw) return defaultState();
    const parsed = JSON.parse(raw) as Partial<State>;
    const def = defaultState();
    return { ...def, ...parsed, missions: parsed.missions ?? def.missions, ledger: parsed.ledger ?? def.ledger, policy: parsed.policy ?? def.policy, toasts: [] };
  } catch {
    return defaultState();
  }
}

function persist(s: State): void {
  if (typeof window === "undefined") return;
  try {
    const rest: State = { ...s, toasts: [] };
    window.localStorage.setItem(KEY, JSON.stringify(rest));
  } catch {
    // storage quota or disabled; ignore — UI still works.
  }
}

type Listener = (s: State) => void;

class Store {
  state: State;
  private listeners = new Set<Listener>();

  constructor() {
    this.state = load();
  }

  subscribe(l: Listener): () => void {
    this.listeners.add(l);
    return () => this.listeners.delete(l);
  }

  private setState(updater: (s: State) => State): void {
    const next = updater(this.state);
    this.state = next;
    persist(next);
    this.listeners.forEach((l) => l(next));
  }

  pushToast(kind: Toast["kind"], message: string, ttl_ms = 4000): void {
    const id = uuid("toast");
    this.setState((s) => ({ ...s, toasts: [...s.toasts, { id, kind, message, ttl_ms }] }));
    if (typeof window !== "undefined") {
      window.setTimeout(() => this.dismissToast(id), ttl_ms);
    }
  }

  dismissToast(id: string): void {
    this.setState((s) => ({ ...s, toasts: s.toasts.filter((t) => t.id !== id) }));
  }

  selectMission(id: string): void {
    this.setState((s) => ({ ...s, selectedMissionId: id }));
  }

  /** Evaluate a policy decision locally, returning the matching effect. */
  evaluate(action: { capability: string; resource: string; riskClass: RiskClass; path?: string }): { effect: PolicyEffect; ruleId: string; reason: string } {
    const p = this.state.policy;
    const path = action.path ?? action.resource ?? "";
    for (const rule of p.rules) {
      if (rule.path_pattern && !new RegExp(rule.path_pattern).test(path)) continue;
      if (rule.capability_glob && !new RegExp("^" + rule.capability_glob.replace(/\*/g, ".*") + "$").test(action.capability)) continue;
      if (rule.score_risk && !rule.score_risk.includes(action.riskClass)) continue;
      return { effect: rule.effect, ruleId: rule.id, reason: rule.reason };
    }
    return { effect: "deny", ruleId: "default-deny", reason: "No matching allow rule — fail-closed" };
  }

  signAndRun(missionId: string, capability: string, argv: string[]): ActionAttestation {
    const mission = this.state.missions.find((m) => m.id === missionId);
    if (!mission) throw new Error("mission not found");
    const decision = this.evaluate({ capability, resource: argv.join(" "), riskClass: mission.risk });
    const id = uuid("act");
    const sigPreview = `ed25519:${hashHex(id + capability + now()).slice(0, 4)}…${hashHex(id + capability).slice(0, 4)}`;
    const action: ActionAttestation = {
      id,
      mission_id: missionId,
      capability,
      resource: mission.constitution.scope.repositories[0] ?? "github:local",
      risk_class: mission.risk,
      status: decision.effect === "allow" ? "executed" : decision.effect === "require_approval" ? "queued" : "denied",
      signature_preview: sigPreview,
      policy_decision: { effect: decision.effect, rule_id: decision.ruleId, bundle_digest: this.state.policy.bundle_digest },
      created_at: now(),
      receipt: decision.effect === "allow"
        ? { code: 0, stdout: `executed: ${capability} ${argv.join(" ")}`, stderr: "", duration_ms: 24, side_effects: [`attestation.recorded:${id}`] }
        : { code: 1, stdout: "", stderr: decision.reason, duration_ms: 1, side_effects: [] },
    };
    this.setState((s) => {
      const missions = s.missions.map((m) => m.id === missionId ? { ...m, actions: [...m.actions, action] } : m);
      const ledgerSeq = s.ledgerSeq + 1;
      const newEntry: LedgerEntry = {
        sequence: s.ledgerSeq,
        event_type: decision.effect === "allow" ? "action.executed" : "policy.denied",
        schema_version: "aevum.ledger/v1",
        tenant_id: mission.tenant,
        mission_id: mission.id,
        correlation_id: mission.id,
        causation_id: action.id,
        actor_id: `spiffe://local.aevum/agent/${argv[0] ?? "operator"}`,
        occurred_at: now(),
        payload: { capability, argv, decision },
        previous_digest: s.ledger[s.ledger.length - 1]?.digest ?? "sha256:000",
        digest: `sha256:rn${hashHex(action.id)}`,
        signature: { alg: "ed25519", value: hashHex(action.id + "sig").padEnd(64, "0").slice(0, 64), key_id: "spiffe://local.aevum/ledger-authority" },
      };
      return {
        ...s,
        missions,
        ledger: [...s.ledger, newEntry],
        ledgerSeq: ledgerSeq,
      };
    });
    this.pushToast(
      decision.effect === "allow" ? "success" : decision.effect === "deny" ? "error" : "warning",
      `${decision.effect === "allow" ? "✓" : decision.effect === "deny" ? "✗" : "⏸"} ${capability} → ${decision.ruleId}`,
    );
    return action;
  }

  challengeEvidence(missionId: string, evidenceId: string, reason: string): void {
    this.setState((s) => ({
      ...s,
      missions: s.missions.map((m) => m.id !== missionId ? m : {
        ...m,
        evidence: m.evidence.map((e) => e.id !== evidenceId ? e : { ...e, status: "challenged", challenge: { by: "spiffe://local.aevum/role/falsifier", reason } }),
      }),
    }));
    this.pushToast("warning", `Evidence challenged: ${evidenceId}`);
  }

  decideApproval(missionId: string, approvalId: string, decision: "approved" | "rejected", reason: string): void {
    this.setState((s) => ({
      ...s,
      missions: s.missions.map((m) => m.id !== missionId ? m : {
        ...m,
        approvals: m.approvals.map((a) => a.id !== approvalId ? a : { ...a, decision, reason, decided_at: now() }),
      }),
    }));
    this.pushToast(decision === "approved" ? "success" : "error", `Approval ${decision}: ${approvalId}`);
  }

  createMission(input: { title: string; summary: string; domains: string[]; risk: RiskClass }): Mission {
    const id = uuid("mis");
    const consti: MissionConstitution = {
      schema: "aevum.mission-constitution/v1",
      mission_id: id,
      version: 1,
      created_by: "spiffe://local.aevum/agent/ui-operator",
      created_at: now(),
      objective: { title: input.title, summary: input.summary, success_outcomes: ["merge_request_applied"], failure_outcomes: ["policy_denied"] },
      scope: {
        repositories: ["github:winterbim/aevum-unify"],
        paths_write: ["packages/**", "crates/**", "apps/**", "docs/**"],
        paths_read: ["**"],
        branches_write: ["aevum/**"],
        branches_protected: ["main"],
        rollout: { environment: "staging", max_blast_radius: "single_repository" },
      },
      risk: { preliminary_class: input.risk, irreversible: false, recovery_strategy: "delete_branch", recovery_verified: true, approval_required: input.risk === "R3" || input.risk === "R4" },
      evidence: { required: ["repo_state", "tests_log"], minimum_completeness: 0.6 },
      budget: { money_eur: 2, wall_clock_seconds: 1200, tokens: 120_000 },
      expiry: "2026-09-15T00:00:00Z",
      domains: input.domains,
    };
    const council: CouncilMember[] = [
      { agent_id: "recon-claude", role: "recon", provider: "anthropic", family: "claude-3-7-sonnet", version: "2026-04-01", domains: ["*"], reasoning_budget_tokens: 4000 },
      { agent_id: "producer-claude", role: "producer", provider: "anthropic", family: "claude-3-7-sonnet", version: "2026-04-01", domains: input.domains, reasoning_budget_tokens: 10000 },
      { agent_id: "verifier-gpt", role: "verifier", provider: "openai", family: "gpt-4o", version: "2026-04-01", domains: ["*"], reasoning_budget_tokens: 6000 },
    ];
    const mission: Mission = {
      id,
      title: input.title,
      status: "draft",
      risk: input.risk,
      constitution: consti,
      council,
      approvals: [{ id: uuid("apr"), mission_id: id, decision: "pending", reviewer: "spiffe://local.aevum/role/human-admin", reason: "Awaiting review", decided_at: now() }],
      actions: [],
      evidence: [
        { id: uuid("evd"), mission_id: id, kind: "repo_state", title: "Repo state", summary: "clean, on main", digest: `sha256:${hashHex(id)}`, captured_at: now(), freshness_window: 600, status: "fresh" },
      ],
      ledger_seq: this.state.ledgerSeq + 1,
      updated_at: now(),
      tenant: "ten_local",
    };
    this.setState((s) => ({
      ...s,
      missions: [mission, ...s.missions],
      ledger: [...s.ledger, {
        sequence: s.ledgerSeq,
        event_type: "mission.drafted",
        schema_version: "aevum.ledger/v1",
        tenant_id: mission.tenant,
        mission_id: mission.id,
        correlation_id: mission.id,
        causation_id: null,
        actor_id: "spiffe://local.aevum/agent/ui-operator",
        occurred_at: now(),
        payload: { title: input.title, risk: input.risk },
        previous_digest: s.ledger[s.ledger.length - 1]?.digest ?? "sha256:000",
        digest: `sha256:draf${hashHex(id)}`,
        signature: { alg: "ed25519", value: hashHex(id + "draft").padEnd(64, "0").slice(0, 64), key_id: "spiffe://local.aevum/ledger-authority" },
      }],
      ledgerSeq: s.ledgerSeq + 1,
      selectedMissionId: mission.id,
    }));
    this.pushToast("success", `Mission created: ${mission.title}`);
    return mission;
  }

  reset(): void {
    const fresh = defaultState();
    this.setState(() => fresh);
    this.pushToast("info", "Store reset to seed data");
  }
}

export const store = new Store();
