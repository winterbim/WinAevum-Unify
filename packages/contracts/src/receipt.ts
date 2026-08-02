import type { ActionId, EvidenceId, LeaseId, ReceiptId } from "./ids.js";

/**
 * The runtime's signed declaration of *what actually happened*.
 * An Execution Receipt is the only place where observed effects are canonicalised.
 */
export type ReceiptStatus =
  | "succeeded"
  | "failed"
  | "cancelled"
  | "compensated";

export interface ObservedEffect {
  resource: string;
  observed: string;
  hash: string;
}

export interface ResourceUsage {
  cpuSeconds: number;
  memoryMbPeak: number;
  diskMbWritten: number;
  networkBytesEgressed: number;
}

export interface ArtifactRef {
  kind: "stdout" | "stderr" | "filesystem_diff" | "network_log" | "test_report";
  digest: string;
  bytes: number;
}

export interface ExecutionReceipt {
  schemaVersion: "aevum.execution-receipt/v1";
  id: ReceiptId;
  actionId: ActionId;
  leaseId: LeaseId;
  executorIdentity: string;
  startedAt: string;
  completedAt: string;
  status: ReceiptStatus;
  observedEffects: ObservedEffect[];
  stdoutArtifact?: ArtifactRef;
  stderrArtifact?: ArtifactRef;
  exitCode?: number;
  resourceUsage: ResourceUsage;
  evidenceIds: EvidenceId[];
  previousLedgerDigest: string;
  signature: { alg: "ed25519"; value: string; keyId: string };
}
