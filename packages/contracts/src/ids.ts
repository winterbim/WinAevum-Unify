/**
 * Canonical identifiers for Aevum Unify domain objects.
 * Schema version: aevum.ids/v1 (M0 skeleton).
 *
 * The matching Rust enum lives in `crates/sentinel-kernel/src/ids.rs`
 * (planned M0+1).
 */

export type MissionId = `mis_${string}`;
export type ClaimId = `clm_${string}`;
export type EvidenceId = `evd_${string}`;
export type DecisionId = `dec_${string}`;
export type ActionId = `act_${string}`;
export type AttestationId = `aat_${string}`;
export type LeaseId = `lea_${string}`;
export type ApprovalId = `apr_${string}`;
export type ReceiptId = `rcp_${string}`;
export type PolicyDecisionId = `pdec_${string}`;
export type AgentId = `agt_${string}`;
export type TenantId = `ten_${string}`;
export type EventId = `evt_${string}`;

export const IdPrefix = {
  Mission: "mis_",
  Claim: "clm_",
  Evidence: "evd_",
  Decision: "dec_",
  Action: "act_",
  Attestation: "aat_",
  Lease: "lea_",
  Approval: "apr_",
  Receipt: "rcp_",
  PolicyDecision: "pdec_",
  Agent: "agt_",
  Tenant: "ten_",
  Event: "evt_",
} as const;

export function isIdOf<T extends string>(value: unknown, prefix: string): value is T {
  return typeof value === "string" && value.startsWith(prefix) && value.length > prefix.length;
}
