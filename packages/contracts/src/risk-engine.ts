// Risk Engine — implements blueprint §15.4 formula.
// risk_score = sum(weights) + (irreversible ? 5 : 0) - recovery_confidence - evidence_strength.
// Score clamped to [0, 100]. Bands map to R0..R5 per blueprint §15.2.

import type { RiskClass } from "./constitution.js";
import { RISK_RANK } from "./constitution.js";

export interface RiskInputs {
  impactWeight: number;       // 0..30
  irreversibilityWeight: number; // 0..30
  privilegeWeight: number;    // 0..20
  dataSensitivityWeight: number; // 0..25
  blastRadiusWeight: number;  // 0..20
  noveltyWeight: number;      // 0..20
  uncertaintyWeight: number;  // 0..20
  externalityWeight: number;  // 0..20
  recoveryConfidence: number; // 0..30 (subtracts)
  evidenceStrength: number;   // 0..30 (subtracts)
  reversible: boolean;        // adds 5 if false
}

export function computeRiskScore(r: RiskInputs): number {
  const sumAdd = r.impactWeight + r.irreversibilityWeight + r.privilegeWeight +
    r.dataSensitivityWeight + r.blastRadiusWeight + r.noveltyWeight +
    r.uncertaintyWeight + r.externalityWeight;
  const irreversPenalty = r.reversible ? 0 : 5;
  const raw = sumAdd + irreversPenalty - r.recoveryConfidence - r.evidenceStrength;
  return Math.max(0, Math.min(100, Math.round(raw)));
}

export function scoreToRiskClass(score: number): RiskClass {
  if (score <= 20) return "R0";
  if (score <= 40) return "R1";
  if (score <= 60) return "R2";
  if (score <= 80) return "R3";
  if (score <= 100) return "R4";
  return "R5";  // future-proof; should not be reached due to clamp
}

// Convenience: classify without manually composing weights.
export interface RiskFactorProfile {
  /** scope: single file (1) .. multi-tenant (10) */
  scope: number;
  /** touches secret (1), pii (2), public (0) */
  dataClass: 0 | 1 | 2;
  /** README (0) .. requires rewriting infra (5) */
  novelty: number;
  /** strong evidence (3), medium (1.5), none (0) — subtracts */
  evidenceStrength: number;
  /** branch revert (5), manual fix (2), no rollback (0) — subtracts */
  recoveryConfidence: number;
  /** writes outside /workspace or pushes to main */
  privilege: number;
  /** destructive (10) .. additive non-destructive (0) */
  destructionMagnitude: number;
}

const PROFILES: Record<RiskClass, (p: RiskFactorProfile, reversible: boolean) => RiskInputs> = {
  R0: () => ({
    impactWeight: 0, irreversibilityWeight: 0, privilegeWeight: 0,
    dataSensitivityWeight: 0, blastRadiusWeight: 0, noveltyWeight: 0,
    uncertaintyWeight: 0, externalityWeight: 0,
    recoveryConfidence: 5, evidenceStrength: 5, reversible: true,
  }),
  R1: (_p, rev) => ({
    impactWeight: 5, irreversibilityWeight: 0, privilegeWeight: 5,
    dataSensitivityWeight: 5, blastRadiusWeight: 5, noveltyWeight: 0,
    uncertaintyWeight: 0, externalityWeight: 0,
    recoveryConfidence: 10, evidenceStrength: 5, reversible: rev,
  }),
  R2: (_p, rev) => ({
    impactWeight: 10, irreversibilityWeight: 5, privilegeWeight: 5,
    dataSensitivityWeight: 5, blastRadiusWeight: 10, noveltyWeight: 5,
    uncertaintyWeight: 5, externalityWeight: 5,
    recoveryConfidence: 10, evidenceStrength: 10, reversible: rev,
  }),
  R3: (p, rev) => ({
    impactWeight: 15 + 2 * p.scope, irreversibilityWeight: 10,
    privilegeWeight: 10 + p.privilege, dataSensitivityWeight: 10 * p.dataClass,
    blastRadiusWeight: 15, noveltyWeight: 5 + p.novelty,
    uncertaintyWeight: 10, externalityWeight: 15,
    recoveryConfidence: p.recoveryConfidence, evidenceStrength: 2 * p.evidenceStrength,
    reversible: rev,
  }),
  R4: (p, rev) => ({
    impactWeight: 20 + 2 * p.scope, irreversibilityWeight: 20,
    privilegeWeight: 15 + p.privilege, dataSensitivityWeight: 15 * p.dataClass,
    blastRadiusWeight: 20, noveltyWeight: 10 + p.novelty,
    uncertaintyWeight: 15, externalityWeight: 20,
    recoveryConfidence: Math.max(0, p.recoveryConfidence - 5),
    evidenceStrength: p.evidenceStrength,
    reversible: rev,
  }),
  R5: (p) => ({
    impactWeight: 30 + p.destructionMagnitude, irreversibilityWeight: 30,
    privilegeWeight: 20 + p.privilege, dataSensitivityWeight: 20 * Math.max(p.dataClass, 1),
    blastRadiusWeight: 25, noveltyWeight: 15 + p.novelty,
    uncertaintyWeight: 20, externalityWeight: 25,
    recoveryConfidence: 0, evidenceStrength: 0, reversible: false,
  }),
};

export function classForProfile(p: RiskFactorProfile, reversible = true): RiskClass {
  // Probe each class, return the one whose profile returns a score in the same band.
  const order: RiskClass[] = ["R5", "R4", "R3", "R2", "R1", "R0"];
  for (const cls of order) {
    const score = computeRiskScore(PROFILES[cls](p, reversible));
    if (RISK_RANK[scoreToRiskClass(score)] === RISK_RANK[cls]) return cls;
  }
  return "R0";
}
