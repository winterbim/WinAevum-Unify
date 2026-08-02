import type { EvidenceId, MissionId } from "./ids.js";

/**
 * An evidence item is content-addressed and references the source it came from.
 * The Trust Ledger binds the digest to the event that recorded it.
 */
export interface SourceRef {
  kind: "filesystem" | "git" | "http" | "tool" | "policy" | "human" | "sandbox";
  locator: string;
  /** SHA-256 of the source bytes. */
  sourceDigest: string;
}

export interface EvidenceItem {
  id: EvidenceId;
  missionId: MissionId;
  statement: string;
  source: SourceRef;
  /** Provisional integrity score in [0, 1]. Final score is computed (D-quality rule). */
  integrityScore?: number;
  observedAt: string;
  validUntil?: string;
}
