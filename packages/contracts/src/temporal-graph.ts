/**
 * Temporal Decision & Evidence Graph contracts.
 *
 * Bi-temporal + episode provenance for the Decision & Evidence Graph,
 * constrained by Aevum blueprint §11 ontology and epistemic firewall.
 */

import type { MissionId } from "./ids.js";

export type NodeKind =
  | "objective"
  | "constraint"
  | "claim"
  | "evidence"
  | "hypothesis"
  | "option"
  | "objection"
  | "experiment"
  | "decision"
  | "action_intent"
  | "outcome"
  | "lesson"
  | "episode"
  | "entity";

export type EdgeKind =
  | "supports"
  | "refutes"
  | "depends_on"
  | "derived_from"
  | "conflicts_with"
  | "tests"
  | "selected_over"
  | "authorizes"
  | "produced"
  | "verified_by"
  | "invalidated_by"
  | "relates_to"
  | "mentions";

export type EpisodeSource =
  | "message"
  | "json"
  | "text"
  | "fact_triple"
  | "attested";

export type EpistemicKind =
  | "fact"
  | "inference"
  | "hypothesis"
  | "recommendation"
  | "unknown";

export interface GraphNode {
  id: string;
  kind: NodeKind;
  name: string;
  summary: string;
  missionId: MissionId | string;
  groupId: string;
  createdAt: string;
  embedding?: number[];
}

export interface Episode {
  id: string;
  missionId: MissionId | string;
  groupId: string;
  source: EpisodeSource;
  content: string;
  /** Required for primary-evidence eligibility when source === "attested". */
  contentDigest?: string;
  /** Event time. */
  validAt: string;
  /** Transaction time. */
  createdAt: string;
  actorId?: string;
}

/**
 * Bi-temporal fact.
 * validAt/invalidAt = event time; createdAt/expiredAt = transaction time.
 */
export interface Fact {
  id: string;
  kind: EdgeKind;
  sourceNodeId: string;
  targetNodeId: string;
  name: string;
  fact: string;
  epistemic: EpistemicKind;
  episodeIds: string[];
  validAt: string;
  invalidAt?: string;
  createdAt: string;
  expiredAt?: string;
  factDigest?: string;
  groupId: string;
  missionId: MissionId | string;
}

/** D01 / §11.5 — only facts may authorize actions. */
export function mayAuthorizeAction(epistemic: EpistemicKind): boolean {
  return epistemic === "fact";
}

/** Event-time window: point ∈ [validAt, invalidAt). Ignores expiredAt. */
export function isFactActiveAt(
  fact: Pick<Fact, "validAt" | "invalidAt">,
  pointIso: string,
): boolean {
  if (pointIso < fact.validAt) return false;
  if (fact.invalidAt && pointIso >= fact.invalidAt) return false;
  return true;
}

/** Currently believed true (transaction-time + not invalidated). */
export function isFactCurrent(
  fact: Pick<Fact, "invalidAt" | "expiredAt">,
): boolean {
  return !fact.expiredAt && !fact.invalidAt;
}

export function isPrimaryEvidenceEligible(ep: Episode): boolean {
  return ep.source === "attested" && Boolean(ep.contentDigest);
}
