import { describe, it, expect } from "vitest";
import { computeRiskScore, scoreToRiskClass, type RiskInputs } from "./risk-engine.js";
import { RISK_RANK } from "./constitution.js";

const base: RiskInputs = {
  impactWeight: 10,
  irreversibilityWeight: 5,
  privilegeWeight: 5,
  dataSensitivityWeight: 5,
  blastRadiusWeight: 5,
  noveltyWeight: 5,
  uncertaintyWeight: 5,
  externalityWeight: 5,
  recoveryConfidence: 0,   // -10 if all defaults
  evidenceStrength: 5,     // -5 if all defaults
  reversible: true,
};

describe("risk-engine", () => {
  it("returns 0 when all weights are 0 (no risk)", () => {
    const score = computeRiskScore({
      impactWeight: 0, irreversibilityWeight: 0, privilegeWeight: 0,
      dataSensitivityWeight: 0, blastRadiusWeight: 0, noveltyWeight: 0,
      uncertaintyWeight: 0, externalityWeight: 0, recoveryConfidence: 0,
      evidenceStrength: 0, reversible: true,
    });
    expect(score).toBe(0);
  });

  it("clamps to [0, 100]", () => {
    const huge = computeRiskScore({
      impactWeight: 1000, irreversibilityWeight: 1000, privilegeWeight: 1000,
      dataSensitivityWeight: 1000, blastRadiusWeight: 1000, noveltyWeight: 1000,
      uncertaintyWeight: 1000, externalityWeight: 1000, recoveryConfidence: 1000,
      evidenceStrength: -1000, reversible: false,
    });
    expect(huge).toBe(100);
    const neg = computeRiskScore({
      ...base,
      impactWeight: 1, recoveryConfidence: 100, evidenceStrength: 100,
    });
    expect(neg).toBe(0);
  });

  it("scoreToRiskClass maps 0..20 → R0", () => {
    expect(scoreToRiskClass(0)).toBe("R0");
    expect(scoreToRiskClass(20)).toBe("R0");
  });

  it("scoreToRiskClass maps 21..40 → R1", () => {
    expect(scoreToRiskClass(21)).toBe("R1");
    expect(scoreToRiskClass(40)).toBe("R1");
  });

  it("scoreToRiskClass maps 81..100 → R4", () => {
    expect(scoreToRiskClass(81)).toBe("R4");
    expect(scoreToRiskClass(100)).toBe("R4");
  });

  it("scoreToRiskClass boundary values align with blueprint §15.4 (R0..R4)", () => {
    // Blueprint §15.2 describes R0..R4 classes; R5 is a regulatory slot used
    // externally (e.g. payment/health) and is added by the autonomy governor
    // (M7) rather than by score alone.
    const cases: Array<[number, keyof typeof RISK_RANK]> = [
      [0, "R0"], [20, "R0"], [21, "R1"], [40, "R1"], [41, "R2"],
      [60, "R2"], [61, "R3"], [80, "R3"], [81, "R4"], [100, "R4"],
    ];
    for (const [v, expected] of cases) {
      expect(scoreToRiskClass(v)).toBe(expected);
    }
  });

  it("deny-by-default: a high-impact irreversible action lands ≥ R4", () => {
    const s = computeRiskScore({
      impactWeight: 25, irreversibilityWeight: 30, privilegeWeight: 25,
      dataSensitivityWeight: 25, blastRadiusWeight: 20, noveltyWeight: 15,
      uncertaintyWeight: 15, externalityWeight: 20, recoveryConfidence: 0,
      evidenceStrength: 0, reversible: false,
    });
    expect(RISK_RANK[scoreToRiskClass(s)]).toBeGreaterThanOrEqual(RISK_RANK.R4);
  });
});
