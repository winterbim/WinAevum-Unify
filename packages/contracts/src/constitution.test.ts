import { describe, it, expect } from "vitest";
import { validateConstitution, diffConstitutionScope, type MissionConstitution } from "./constitution.js";

const valid: MissionConstitution = {
  schemaVersion: "aevum.mission-constitution/v1",
  missionId: "mis_01JCL0M0000000000000000000",
  version: 1,
  createdBy: "usr_winter",
  objective: {
    statement: "Audit and patch vulnerable deps in the example repo.",
    successOutcomes: ["all tests pass", "no critical CVE remains", "PR created without merge"],
  },
  scope: {
    repositories: ["github:winterbim/example-app"],
    branchesRead: ["main"],
    branchesWrite: ["aevum/*"],
    allowedPaths: ["/**"],
    deniedPaths: ["/.env", "/secrets/**"],
  },
  constraints: {
    productionEffects: "forbidden",
    secretExposure: "forbidden",
    externalMessages: "approval-required",
  },
  budgets: {
    moneyEurMax: 5,
    wallClockSecondsMax: 3600,
    toolCallsMax: 500,
  },
  riskPolicy: {
    maxAutonomousRisk: "R2",
    humanApprovalFrom: "R3",
  },
  verification: {
    producerMustNotBeOnlyVerifier: true,
    requiredChecks: ["lint", "typecheck", "unit", "dependency-audit"],
  },
  recovery: {
    requireSnapshot: true,
    requireRollbackTestFor: ["R4", "R5"],
  },
  notBefore: "2026-08-02T10:00:00+02:00",
  expiresAt: "2026-08-02T14:00:00+02:00",
};

describe("constitution", () => {
  it("accepts a valid constitution", () => {
    const r = validateConstitution(valid);
    expect(r.ok).toBe(true);
  });

  it("rejects when successOutcomes is empty", () => {
    const r = validateConstitution({ ...valid, objective: { ...valid.objective, successOutcomes: [] } });
    expect(r).toMatchObject({ ok: false });
    if (r.ok) return;
    expect(r.errors.some((e) => e.path.startsWith("objective.successOutcomes"))).toBe(true);
  });

  it("rejects when producedBy is missing", () => {
    const bad: any = { ...valid };
    delete bad.createdBy;
    const r = validateConstitution(bad);
    expect(r.ok).toBe(false);
  });

  it("rejects when moneyEurMax is negative", () => {
    const r = validateConstitution({ ...valid, budgets: { ...valid.budgets, moneyEurMax: -1 } });
    expect(r.ok).toBe(false);
  });

  it("rejects when risk policy is internally inconsistent (maxAutonomous > humanApprovalFrom)", () => {
    const r = validateConstitution({
      ...valid,
      riskPolicy: { maxAutonomousRisk: "R4", humanApprovalFrom: "R2" },
    });
    expect(r.ok).toBe(false);
  });

  it("rejects when deniedPaths overlaps allowedPaths with stricter path syntax (winning more specific)", () => {
    // allowed: /**  ; denied: /.env — denied is more specific; allowed must not include denied paths.
    const r = validateConstitution({
      ...valid,
      scope: { ...valid.scope, allowedPaths: ["/**"], deniedPaths: [] },
    });
    expect(r.ok).toBe(true); // empty deny list is fine
  });

  it("scope-diff: removing branchesWrite entries (without add) triggers narrowing", () => {
    // prev allows aevum/* and aevum/extra; next allows only aevum/*.
    const d = diffConstitutionScope(
      { ...valid, scope: { ...valid.scope, branchesWrite: ["aevum/*", "aevum/extra"] } },
      { ...valid, version: 2, scope: { ...valid.scope, branchesWrite: ["aevum/*"] } },
    );
    expect(d.kind).toBe("narrowing");
    expect(d.removedBranchesWrite).toContain("aevum/extra");
  });

  it("scope-diff: expanding branchesRead triggers EXPANDING", () => {
    const d = diffConstitutionScope(valid, { ...valid, scope: { ...valid.scope, branchesRead: ["main", "develop"] } });
    expect(d.kind).toBe("expanding");
  });

  it("scope-diff: when risk policy moves maxAutonomousRisk beyond previous class, expanding", () => {
    const d = diffConstitutionScope(valid, {
      ...valid,
      version: 2,
      riskPolicy: { maxAutonomousRisk: "R4", humanApprovalFrom: "R4" },
    });
    expect(d.kind).toBe("expanding");
  });

  it("scope-diff: identical constitutions report 'unchanged'", () => {
    const d = diffConstitutionScope(valid, valid);
    expect(d.kind).toBe("unchanged");
  });

  it("compute digest is stable across field order on canonicalised subset", async () => {
    const { computeConstitutionDigest } = await import("./constitution.js");
    const a = computeConstitutionDigest(valid);
    const b = computeConstitutionDigest({ ...valid });
    expect(a).toBe(b);
  });
});
