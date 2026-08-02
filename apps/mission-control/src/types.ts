/**
 * TypeScript port of the canonical Aevum contracts (M0–M7).
 * Field-level types are 1:1 with packages/contracts/src/*.ts. Any change to
 * the contract must be mirrored here. We intentionally keep this file
 * dependency-free — the types live in `contracts/` and a consumer should
 * later replace these by importing the package via `pnpm exec node`.
 */

export type RiskClass = "R0" | "R1" | "R2" | "R3" | "R4" | "R5";

export type MissionStatus =
  | "draft"
  | "constitutional_review"
  | "approved"
  | "executing"
  | "completed"
  | "failed"
  | "expired";

export type CouncilRole =
  | "recon"
  | "planner"
  | "producer"
  | "falsifier"
  | "verifier"
  | "guardian"
  | "arbiter"
  | "observer";

export type PolicyEffect =
  | "allow"
  | "deny"
  | "require_approval"
  | "require_more_evidence"
  | "require_safer_alternative"
  | "defer_to_domain_policy";

export interface MissionConstitution {
  schema: "aevum.mission-constitution/v1";
  mission_id: string;
  version: number;
  created_by: string;
  created_at: string;
  objective: {
    title: string;
    summary: string;
    success_outcomes: string[];
    failure_outcomes: string[];
  };
  scope: {
    repositories: string[];
    paths_write: string[];
    paths_read: string[];
    branches_write: string[];
    branches_protected: string[];
    rollout: { environment: "production" | "staging" | "local"; max_blast_radius: string };
  };
  risk: {
    preliminary_class: RiskClass;
    irreversible: boolean;
    recovery_strategy: string;
    recovery_verified: boolean;
    approval_required: boolean;
  };
  evidence: {
    required: string[];
    minimum_completeness: number;
  };
  budget: { money_eur: number; wall_clock_seconds: number; tokens: number };
  expiry: string;
  domains: string[];
}

export interface CouncilMember {
  agent_id: string;
  role: CouncilRole;
  provider: string;
  family: string;
  version: string;
  domains: string[];
  reasoning_budget_tokens: number;
}

export interface Mission {
  id: string;
  title: string;
  status: MissionStatus;
  risk: RiskClass;
  constitution: MissionConstitution;
  council: CouncilMember[];
  approvals: Approval[];
  actions: ActionAttestation[];
  evidence: EvidenceItem[];
  ledger_seq: number;
  updated_at: string;
  tenant: string;
}

export interface Approval {
  id: string;
  mission_id: string;
  decision: "approved" | "rejected" | "pending";
  reviewer: string;
  reason: string;
  decided_at: string;
}

export interface ActionAttestation {
  id: string;
  mission_id: string;
  capability: string;
  resource: string;
  risk_class: RiskClass;
  status: "queued" | "authorised" | "denied" | "executed" | "failed";
  signature_preview: string;
  policy_decision: { effect: PolicyEffect; rule_id: string; bundle_digest: string };
  created_at: string;
  receipt?: { code: number; stdout: string; stderr: string; duration_ms: number; side_effects: string[] };
}

export interface EvidenceItem {
  id: string;
  mission_id: string;
  kind:
    | "repo_state"
    | "tests_log"
    | "lint_log"
    | "build_log"
    | "dependency_audit"
    | "user_signoff"
    | "constitution_digest"
    | "policy_decision"
    | "objection";
  title: string;
  summary: string;
  digest: string;
  captured_at: string;
  freshness_window: number;
  status: "fresh" | "stale" | "challenged";
  challenge?: { by: string; reason: string };
}

export interface LedgerEntry {
  sequence: number;
  event_type: string;
  schema_version: string;
  tenant_id: string;
  mission_id: string;
  correlation_id: string;
  causation_id: string | null;
  actor_id: string;
  occurred_at: string;
  payload: Record<string, unknown>;
  previous_digest: string;
  digest: string;
  signature: { alg: "ed25519"; value: string; key_id: string };
}

export interface PolicyRule {
  id: string;
  description: string;
  effect: PolicyEffect;
  reason: string;
  score_risk?: RiskClass[];
  capability_glob?: string;
  path_pattern?: string;
}

export interface PolicyBundle {
  version: string;
  bundle_digest: string;
  rules: PolicyRule[];
}

export interface Toast {
  id: string;
  kind: "info" | "success" | "warning" | "error";
  message: string;
  ttl_ms?: number;
}
