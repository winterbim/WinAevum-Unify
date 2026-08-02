import type { ClaimId, EvidenceId, MissionId } from "./ids.js";

/**
 * Epistemic status of a claim. D01: a claim is not a proof until evidence attests it.
 */
export type ClaimKind =
  | "fact"
  | "inference"
  | "hypothesis"
  | "recommendation"
  | "unknown";

export type ClaimStatus =
  | "proposed"
  | "supported"
  | "contested"
  | "rejected"
  | "superseded";

export interface ActorRef {
  principalId: string;
  agentDefinition?: string;
  instanceId?: string;
}

export interface Claim {
  id: ClaimId;
  missionId: MissionId;
  kind: ClaimKind;
  statement: string;
  author: ActorRef;
  confidence?: number;
  evidenceIds: EvidenceId[];
  challengedBy: string[];
  status: ClaimStatus;
  createdAt: string;        // ISO-8601 with timezone
  validUntil?: string;      // ISO-8601 with timezone
}
