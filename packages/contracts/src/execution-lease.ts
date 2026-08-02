import type { ActionId, ApprovalId, LeaseId, MissionId } from "./ids.js";
import type { CapabilityGrant } from "./capability.js";
import type { ActorRef } from "./claim.js";
import type { PolicyDecisionId } from "./ids.js";

/**
 * An Execution Lease exchanges an attestation against a one-time authorisation.
 * Maximum uses is typically 1; only idempotent capabilities may be multi-use.
 */
export interface ExecutionLease {
  id: LeaseId;
  subject: ActorRef;
  missionId: MissionId;
  actionId: ActionId;
  grants: CapabilityGrant[];
  issuedAt: string;
  expiresAt: string;
  nonce: string;
  policyDecisionId: PolicyDecisionId;
  approvalId: ApprovalId;
  revocationEndpoint: string;
  signature: { alg: "ed25519"; value: string; keyId: string };
  maxUses: number;
  remainingUses: number;
}
