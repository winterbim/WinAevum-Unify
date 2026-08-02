import type {
  ActionId,
  ApprovalId,
  AttestationId,
  EvidenceId,
  LeaseId,
  MissionId,
  PolicyDecisionId,
} from "./ids.js";
import type { ActorRef } from "./claim.js";

// RiskClass is canonically defined in `./constitution.ts` and re-exported via
// `index.ts`. Do not re-declare it here; just import the type for use below.
import type { RiskClass } from "./constitution.js";

export interface MissionRef {
  missionId: MissionId;
  constitutionVersion: number;
  /** SHA-256 of the canonical Constitution JSON. */
  constitutionDigest: string;
}

export interface IntentSpec {
  capability: string;
  resource: string;
  /** SHA-256 of canonical parameters JSON. */
  parametersDigest: string;
  expectedEffects: string[];
  forbiddenEffects: string[];
}

export interface EvidenceBundle {
  required: string[];
  attachedIds: EvidenceId[];
  /** 0..1 coverage of the required evidence. */
  completeness: number;
}

export interface RiskSpec {
  class: RiskClass;
  score: number;            // 0..100
  reversible: boolean;
  blastRadius: string;
}

export interface AuthoritySpec {
  /** SHA-256 of the policy bundle that authorised this attestation. */
  policyBundleDigest: string;
  policyDecisionId: PolicyDecisionId;
  approvalIds: ApprovalId[];
  notBefore: string;
  expiresAt: string;
  maxUses: number;
}

export interface RecoverySpec {
  strategy: string;          // e.g. "delete_branch"
  verified: boolean;
}

/**
 * Action Attestation is the *proof-carrying* envelope the Sentinel Kernel
 * re-evaluates at commit time.
 */
export interface ActionAttestation {
  schemaVersion: "aevum.action-attestation/v1";
  attestationId: AttestationId;
  actionId: ActionId;
  missionRef: MissionRef;
  actor: ActorRef & { councilRole: string };
  intent: IntentSpec;
  evidence: EvidenceBundle;
  risk: RiskSpec;
  authority: AuthoritySpec;
  recovery: RecoverySpec;
  nonce: string;
  /** ed25519 base64-url(signature) over canonical JSON without this field. */
  signature?: string;
}

/** State machine (M3): */
export type AttestationState =
  | "DRAFT"
  | "EVIDENCE_READY"
  | "POLICY_EVALUATED"
  | "APPROVAL_PENDING"
  | "PREPARED"
  | "LEASED"
  | "COMMITTED"
  | "VERIFIED"
  | "CLOSED"
  | "DENIED"
  | "EXPIRED"
  | "CANCELLED"
  | "FAILED"
  | "COMPENSATED"
  | "ROLLED_BACK";

/** Helper: id relationship invariants documented for the builder. */
export interface AttestationEdgeRefs {
  lease?: LeaseId;
  approval?: ApprovalId[];
}
