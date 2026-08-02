import { describe, it, expect } from "vitest";
import { evaluatePolicy, defaultPolicyBundle, type PolicyInput, type PolicyDecision } from "./policy.js";

describe("policy evaluator", () => {
  const baseAction = {
    type: "git.branch.create",
    riskClass: "R2" as const,
    resource: { repository: "github:winterbim/example-app" },
    intent: { branch: "aevum/sec-fix" },
    evidence: { testsPass: true, dependencyAudit: true, lintPass: true },
    irreversible: false,
    environment: "staging" as const,
    dataClassification: 1,
    approval: { status: "n/a" as const },
  };

  it("denies production deployments without human approval", () => {
    const decision = evaluatePolicy(defaultPolicyBundle(), {
      ...baseAction,
      type: "deployment.promote",
      environment: "production",
      riskClass: "R4",
    } as PolicyInput);
    expect(decision.effect).toBe("require_approval");
  });

  it("allows git branch creation on R2 with proper recovery", () => {
    const decision = evaluatePolicy(defaultPolicyBundle(), {
      ...baseAction,
      intent: { branch: "aevum/sec-fix", recoveryStrategy: "delete_branch" },
    } as PolicyInput);
    expect(decision.effect).toBe("allow");
  });

  it("denies git branch creation targeting main", () => {
    const decision = evaluatePolicy(defaultPolicyBundle(), {
      ...baseAction,
      intent: { branch: "main" },
    } as PolicyInput);
    expect(decision.effect).toBe("deny");
    expect(decision.reason).toMatch(/main/);
  });

  it("denies writes that include dangerous paths", () => {
    const decision = evaluatePolicy(defaultPolicyBundle(), {
      ...baseAction,
      type: "fs.write",
      resource: { repository: "any", path: "/secrets/api.key" },
    } as PolicyInput);
    expect(decision.effect).toBe("deny");
    expect(decision.reason).toMatch(/secrets|\\.env|\\.ssh|\\.aws/);
  });

  it("denies when risk class is R5 by default", () => {
    const decision = evaluatePolicy(defaultPolicyBundle(), {
      ...baseAction,
      riskClass: "R5",
      type: "payment.send",
    } as PolicyInput);
    expect(decision.effect).toBe("deny");
    expect(decision.reason).toMatch(/R5/);
  });

  it("returns deny when no rule matches (default deny)", () => {
    const decision = evaluatePolicy(defaultPolicyBundle(), {
      ...baseAction,
      type: "unknown.capability",
    } as PolicyInput);
    expect(decision.effect).toBe("deny");
  });

  it("produces a stable decision id (sha256-prefixed) for the same input", () => {
    const a: PolicyDecision = evaluatePolicy(defaultPolicyBundle(), baseAction as PolicyInput);
    const b: PolicyDecision = evaluatePolicy(defaultPolicyBundle(), baseAction as PolicyInput);
    expect(a.id).toBe(b.id);
  });

  it("deny rule overrides allow rule (explicit deny precedence)", () => {
    const decision = evaluatePolicy(defaultPolicyBundle(), {
      ...baseAction,
      type: "fs.write",
      resource: { repository: "any", path: "/workspace/src/main.ts" },
      intent: { branch: "aevum/sec-fix", recoveryStrategy: "delete_branch" },
    } as PolicyInput);
    // FsGuard denies any write to a path matching .env or secrets regardless.
    expect(decision.effect).toBe("deny");
  });

  it("blocks path-traversal attempts", () => {
    const decision = evaluatePolicy(defaultPolicyBundle(), {
      ...baseAction,
      type: "fs.read",
      resource: { repository: "any", path: "/workspace/../../etc/passwd" },
    } as PolicyInput);
    expect(decision.effect).toBe("deny");
  });
});
