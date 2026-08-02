/**
 * Seed sample data — exactly what the user sees on first load.
 *
 * The seed is deterministic so that screenshots and the audit evidence
 * never drift. Each entry references a coherent mission graph (1 mission
 * ↔ N council members ↔ N actions ↔ N evidence ↔ N ledger entries).
 */
import type {
  Approval,
  ActionAttestation,
  CouncilMember,
  EvidenceItem,
  LedgerEntry,
  Mission,
  MissionConstitution,
  PolicyBundle,
} from "./types";

function iso(daysAgo: number, hourOfDay = 9): string {
  const d = new Date("2026-08-02T00:00:00Z");
  d.setUTCDate(d.getUTCDate() - daysAgo);
  d.setUTCHours(hourOfDay);
  return d.toISOString();
}

function makeCouncil(_missionId: string, domain: string[], risk: "R2" | "R3"): CouncilMember[] {
  const base: CouncilMember[] = [
    { agent_id: "recon-claude", role: "recon", provider: "anthropic", family: "claude-3-7-sonnet", version: "2026-04-01", domains: ["*"], reasoning_budget_tokens: 4000 },
    { agent_id: "producer-claude", role: "producer", provider: "anthropic", family: "claude-3-7-sonnet", version: "2026-04-01", domains: ["code", ...domain], reasoning_budget_tokens: 12000 },
  ];
  if (risk === "R2") return base.concat([
    { agent_id: "falsifier-gemini", role: "falsifier", provider: "google", family: "gemini-2.0-flash", version: "2026-04-01", domains: ["*"], reasoning_budget_tokens: 6000 },
    { agent_id: "verifier-gpt", role: "verifier", provider: "openai", family: "gpt-4o", version: "2026-04-01", domains: ["*"], reasoning_budget_tokens: 8000 },
    { agent_id: "guardian-local", role: "guardian", provider: "local", family: "aevum-policy-engine", version: "2026-04-01", domains: ["*"], reasoning_budget_tokens: 1500 },
  ]);
  return base.concat([
    { agent_id: "falsifier-gemini", role: "falsifier", provider: "google", family: "gemini-2.0-flash", version: "2026-04-01", domains: ["*"], reasoning_budget_tokens: 6000 },
    { agent_id: "verifier-gpt", role: "verifier", provider: "openai", family: "gpt-4o", version: "2026-04-01", domains: ["*"], reasoning_budget_tokens: 8000 },
    { agent_id: "guardian-local", role: "guardian", provider: "local", family: "aevum-policy-engine", version: "2026-04-01", domains: ["*"], reasoning_budget_tokens: 2000 },
    { agent_id: "planner-gpt", role: "planner", provider: "openai", family: "gpt-4o", version: "2026-04-01", domains: ["code"], reasoning_budget_tokens: 4000 },
    { agent_id: "arbiter-local", role: "arbiter", provider: "local", family: "aevum-arbiter", version: "2026-04-01", domains: ["*"], reasoning_budget_tokens: 1500 },
  ]);
}

function makeConstitution(opts: { missionId: string; risk: "R2" | "R3"; title: string; domains: string[]; summary: string }): MissionConstitution {
  return {
    schema: "aevum.mission-constitution/v1",
    mission_id: opts.missionId,
    version: 1,
    created_by: "spiffe://local.aevum/agent/producer-claude",
    created_at: iso(0, 9),
    objective: {
      title: opts.title,
      summary: opts.summary,
      success_outcomes: ["merge_request_applied", "tests_passing", "evidence_attached"],
      failure_outcomes: ["policy_denied", "tests_failing", "evidence_stale"],
    },
    scope: {
      repositories: ["github:winterbim/aevum-unify"],
      paths_write: ["packages/**", "crates/**", "apps/**", "docs/**", ".project/**"],
      paths_read: ["**"],
      branches_write: ["aevum/**"],
      branches_protected: ["main"],
      rollout: { environment: "staging", max_blast_radius: "single_repository" },
    },
    risk: {
      preliminary_class: opts.risk,
      irreversible: false,
      recovery_strategy: "delete_branch",
      recovery_verified: true,
      approval_required: opts.risk === "R3",
    },
    evidence: {
      required: ["repo_state", "tests_log", "lint_log", "dependency_audit"],
      minimum_completeness: 0.6,
    },
    budget: { money_eur: 5, wall_clock_seconds: 1800, tokens: 240_000 },
    expiry: "2026-09-01T00:00:00Z",
    domains: opts.domains,
  };
}

export const seedPolicyBundle: PolicyBundle = {
  version: "aevum.policy/v1.0.0",
  bundle_digest: "sha256:b6dcdbc4c66c3a4f14c2b6c1d2fa2e2c7eab9b3e7f6c2a5d3b9c0e8a4d2b3f0a",
  rules: [
    { id: "deny.path.hidden-files", description: "Block writes/reads on hidden credential paths (.env, /etc/passwd, /secrets)", effect: "deny", reason: "Touch on hidden credential path", path_pattern: "/(\\.env|/secrets|/etc/passwd|\\.ssh|\\.aws)" },
    { id: "deny.path.traversal", description: "Reject any reference that escapes the workspace", effect: "deny", reason: "Path contains a `..` traversal component", path_pattern: "\\.\\./|\\.\\.\\\\\\\\" },
    { id: "deny.git.main", description: "Refuse writes against `main`", effect: "deny", reason: "Writes against `main` are forbidden", capability_glob: "git.*" },
    { id: "deny.r5-by-default", description: "R5 actions require a dedicated bundle entry", effect: "deny", reason: "R5 never allowed by default" },
    { id: "require-approval.production-deploy", description: "Production deployments always require human approval", effect: "require_approval", reason: "Production deployment is destructive" },
    { id: "deny.sh.execute", description: "Reject sh / sh-c execution paths", effect: "deny", reason: "sh -c style execution is FORBIDDEN on the agentic path (§16.4)", capability_glob: "sh.*" },
    { id: "allow.git.branch-create", description: "Allow branch creation when recovery exists and evidence is solid", effect: "allow", reason: "Branch create on R2 with delete_branch recovery and tests+audit pass" },
    { id: "default-deny", description: "Deny when no rule matches", effect: "deny", reason: "No matching allow rule — fail-closed" },
  ],
};

const seedCouncilConstitution = (missionId: string, idx: number) =>
  makeConstitution({
    missionId,
    risk: idx % 4 === 3 ? "R3" : "R2",
    title: ["Refactor identities", "Patch readme", "Harden policy engine", "Bump attestation version", "Add council diversity gate", "Migrate to local-first mode", "Refresh ledger", "Wire capability grants"][idx % 8],
    domains: ["code"],
    summary: "One of 8 demo missions that ships with Mission Control — a self-contained, easily readable mission graph to demo the UI and audit flow.",
  });

const councilCached: CouncilMember[][] = [];
function councilFor(missionId: string, risk: "R2" | "R3"): CouncilMember[] {
  const key = `${missionId}-${risk}`;
  let c = councilCached.find((arr) => arr[0]?.agent_id === key);
  if (!c) {
    c = makeCouncil(missionId, ["code"], risk);
    councilCached.push(c);
  }
  return c ?? makeCouncil(missionId, ["code"], risk);
}

const apCache: Approval[][] = [];
function approvalFor(missionId: string): Approval[] {
  let a = apCache.find((arr) => arr[0]?.mission_id === missionId);
  if (!a) {
    a = [
      { id: "apr_01", mission_id: missionId, decision: "approved", reviewer: "spiffe://local.aevum/role/human-admin", reason: "Risk profile acceptable", decided_at: iso(0, 8) },
    ];
    apCache.push(a);
  }
  return a ?? [{ id: "apr_01", mission_id: missionId, decision: "approved", reviewer: "x", reason: "y", decided_at: iso(0, 8) }];
}

const evCache: EvidenceItem[][] = [];
function evidenceFor(missionId: string): EvidenceItem[] {
  let ev = evCache.find((arr) => arr[0]?.mission_id === missionId);
  if (!ev) {
    ev = [
      { id: "evd_1", mission_id: missionId, kind: "repo_state", title: "Git state snapshot", summary: "git status: clean, on aevum/sec-fix, branch 01JC...", digest: "sha256:9c2f1e", captured_at: iso(0, 9), freshness_window: 600, status: "fresh" },
      { id: "evd_2", mission_id: missionId, kind: "tests_log", title: "cargo test --workspace", summary: "23 tests passed / 0 failed / 0 ignored", digest: "sha256:7a92ab", captured_at: iso(0, 9), freshness_window: 600, status: "fresh" },
      { id: "evd_3", mission_id: missionId, kind: "lint_log", title: "pnpm -r lint", summary: "OK — 0 errors, 0 warnings", digest: "sha256:1b7d8a", captured_at: iso(0, 9), freshness_window: 600, status: "fresh" },
      { id: "evd_4", mission_id: missionId, kind: "dependency_audit", title: "cargo audit", summary: "0 advisories in 142 crates", digest: "sha256:22f4bb", captured_at: iso(0, 9), freshness_window: 1800, status: "fresh" },
    ];
    evCache.push(ev);
  }
  return ev ?? [];
}

const actCache: ActionAttestation[][] = [];
function actionsFor(missionId: string): ActionAttestation[] {
  let acts = actCache.find((arr) => arr[0]?.mission_id === missionId);
  if (!acts) {
    acts = [
      { id: "act_01", mission_id: missionId, capability: "git.branch.create", resource: "github:winterbim/aevum-unify", risk_class: "R2", status: "executed", signature_preview: "ed25519:9ab1…ff01", policy_decision: { effect: "allow", rule_id: "allow.git.branch-create", bundle_digest: seedPolicyBundle.bundle_digest }, created_at: iso(0, 9), receipt: { code: 0, stdout: "Switched to a new branch 'aevum/sec-fix'", stderr: "", duration_ms: 84, side_effects: ["branch.created:aevum/sec-fix"] } },
      { id: "act_02", mission_id: missionId, capability: "git.commit", resource: "github:winterbim/aevum-unify", risk_class: "R2", status: "executed", signature_preview: "ed25519:7f03…a1c0", policy_decision: { effect: "allow", rule_id: "allow.git.branch-create", bundle_digest: seedPolicyBundle.bundle_digest }, created_at: iso(0, 9), receipt: { code: 0, stdout: "[aevum/sec-fix 9c2f1e] patch", stderr: "", duration_ms: 122, side_effects: ["commit.created:9c2f1e"] } },
      { id: "act_03", mission_id: missionId, capability: "fs.write", resource: "/secrets/api.key", risk_class: "R2", status: "denied", signature_preview: "ed25519:8b40…2210", policy_decision: { effect: "deny", rule_id: "deny.path.hidden-files", bundle_digest: seedPolicyBundle.bundle_digest }, created_at: iso(0, 9), receipt: { code: 1, stdout: "", stderr: "D-rule: touch on hidden credential path", duration_ms: 2, side_effects: ["policy.deny"] } },
    ];
    actCache.push(acts);
  }
  return acts ?? [];
}

const missionCache: Mission[] = [];
function missionFor(idx: number): Mission {
  let m = missionCache.find((x) => x.id === `mis_${(idx + 1).toString().padStart(2, "0")}`);
  if (!m) {
    const id = `mis_${(idx + 1).toString().padStart(2, "0")}`;
    const risk = idx % 4 === 3 ? "R3" : "R2";
    m = {
      id,
      title: seedCouncilConstitution(id, idx).objective.title,
      status: (["draft", "constitutional_review", "approved", "executing", "completed"] as const)[idx % 5],
      risk,
      constitution: seedCouncilConstitution(id, idx),
      council: councilFor(id, risk as "R2" | "R3"),
      approvals: approvalFor(id),
      actions: actionsFor(id),
      evidence: evidenceFor(id),
      ledger_seq: 7 + idx,
      updated_at: iso(0, 9 + (idx % 8)),
      tenant: "ten_local",
    };
    missionCache.push(m);
  }
  return m!;
}

export const seedMissions: Mission[] = Array.from({ length: 8 }, (_, i) => missionFor(i));

/** Generate a coherent ledger — 1 mission.created per mission + ~3 steps each. */
export const seedLedger: LedgerEntry[] = seedMissions.flatMap((m, mIdx) => {
  const base: LedgerEntry[] = [{
    sequence: 1 + mIdx * 6,
    event_type: "mission.created",
    schema_version: "aevum.ledger/v1",
    tenant_id: m.tenant,
    mission_id: m.id,
    correlation_id: m.id,
    causation_id: null,
    actor_id: "spiffe://local.aevum/agent/producer-claude",
    occurred_at: iso(mIdx),
    payload: { title: m.title, risk: m.risk },
    previous_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    digest: `sha256:dgt0${mIdx}a`,
    signature: { alg: "ed25519", value: "00".repeat(32) + `${mIdx.toString(16).padStart(2, "0")}`, key_id: "spiffe://local.aevum/ledger-authority" },
  },
  {
    sequence: 2 + mIdx * 6,
    event_type: "mission.constitution.accepted",
    schema_version: "aevum.ledger/v1",
    tenant_id: m.tenant,
    mission_id: m.id,
    correlation_id: m.id,
    causation_id: `${m.id}.created`,
    actor_id: "spiffe://local.aevum/role/human-admin",
    occurred_at: iso(mIdx),
    payload: { constitution_digest: m.constitution.version === 1 ? "sha256:c001" : "sha256:c002" },
    previous_digest: `sha256:dgt0${mIdx}a`,
    digest: `sha256:dgt0${mIdx}b`,
    signature: { alg: "ed25519", value: "01".repeat(32) + `${mIdx.toString(16).padStart(2, "0")}`, key_id: "spiffe://local.aevum/ledger-authority" },
  }];
  for (let s = 0; s < 4; s++) {
    const seq = 3 + s + mIdx * 6;
    const prev = `sha256:dgt0${mIdx}${String.fromCharCode(97 + s)}`;
    base.push({
      sequence: seq,
      event_type: ["policy.evaluated", "action.attested", "evidence.attached", "receipt.emitted"][s],
      schema_version: "aevum.ledger/v1",
      tenant_id: m.tenant,
      mission_id: m.id,
      correlation_id: m.id,
      causation_id: `${m.id}.step-${s}`,
      actor_id: ["spiffe://local.aevum/role/verifier", "spiffe://local.aevum/agent/producer-claude", "spiffe://local.aevum/agent/recon-claude", "spiffe://local.aevum/agent/local"][s % 4],
      occurred_at: iso(mIdx),
      payload: { rule_id: s === 0 ? "allow.git.branch-create" : "deny.sh.execute", capability: s === 1 ? "git.branch.create" : "fs.write" },
      previous_digest: prev,
      digest: `sha256:dgt${mIdx}${String.fromCharCode(97 + s + 1)}`,
      signature: { alg: "ed25519", value: String(s + 2).padStart(2, "0").repeat(32) + "00", key_id: "spiffe://local.aevum/ledger-authority" },
    });
  }
  return base;
});
