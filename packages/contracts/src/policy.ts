// Policy evaluator — minimal Rego-inspired rule engine for Action Attestation.
// Source: blueprint §15, §18, Annexe C.

import { canonicalJsonStringify } from "./canonical.js";
import { sha256 } from "./canonical.js";
import type { RiskClass } from "./constitution.js";

export type PolicyEffect =
  | "allow"
  | "deny"
  | "require_approval"
  | "require_more_evidence"
  | "require_safer_alternative"
  | "defer_to_domain_policy";

export interface PolicyInput {
  type: string;
  riskClass: RiskClass;
  resource: {
    repository?: string;
    path?: string;
    workspaceRoot?: string;
  };
  intent: {
    branch?: string;
    recoveryStrategy?: string;
    [k: string]: unknown;
  };
  evidence: {
    testsPass?: boolean;
    dependencyAudit?: boolean;
    lintPass?: boolean;
    [k: string]: boolean | undefined;
  };
  irreversible: boolean;
  environment: "production" | "staging" | "local" | "any";
  dataClassification: 0 | 1 | 2 | 3;
  approval?: { status: "approved" | "rejected" | "pending" | "n/a" };
}

export interface PolicyRule {
  id: string;
  description: string;
  effect: PolicyEffect;
  reason: string;
  match(input: PolicyInput): boolean;
}

export interface PolicyBundle {
  rules: PolicyRule[];
  version: string;
  bundleDigest(): string;
}

export interface PolicyDecision {
  id: string;
  effect: PolicyEffect;
  ruleId: string;
  reason: string;
  bundleDigest: string;
  decidedAt: string;
}

function denyIfPathHidden(input: PolicyInput): boolean {
  if (!input.resource.path) return false;
  const p = input.resource.path;
  return (
    p.includes("/.env") ||
    p.includes("/secrets/") ||
    p.includes("/.ssh/") ||
    p.includes("/.aws/") ||
    p.includes("/etc/passwd") ||
    p.includes("/.git/") ||
    p.includes("/proc/")
  );
}

function denyIfPathTraversal(input: PolicyInput): boolean {
  if (!input.resource.path) return false;
  const p = input.resource.path.replace(/\\/g, "/");
  return p.split("/").includes("..");
}

function denyMainBranch(input: PolicyInput): boolean {
  return input.type.startsWith("git.") && input.intent.branch === "main";
}

export function defaultPolicyBundle(): PolicyBundle {
  const rules: PolicyRule[] = [
    {
      id: "deny.path.hidden-files",
      description: "Block writes/reads on hidden credential paths.",
      effect: "deny",
      reason: "D-rule: touch on hidden credential path (.env/.ssh/.aws/secrets/etc/passwd)",
      match: denyIfPathHidden,
    },
    {
      id: "deny.path.traversal",
      description: "Reject any reference that escapes the workspace.",
      effect: "deny",
      reason: "D-rule: path contains a `..` traversal component",
      match: denyIfPathTraversal,
    },
    {
      id: "deny.git.main",
      description: "Refuse writes against `main`.",
      effect: "deny",
      reason: "D-rule: writes against `main` are explicitly forbidden",
      match: denyMainBranch,
    },
    {
      id: "deny.r5-by-default",
      description: "R5 actions require a dedicated bundle entry.",
      effect: "deny",
      reason: "D-rule: R5 never allowed by default — override required",
      match: (i) => i.riskClass === "R5",
    },
    {
      id: "require-approval.production-deploy",
      description: "Production deployments always require explicit human approval.",
      effect: "require_approval",
      reason: "R3-rule: production deployment without R0-R3 risk",
      match: (i) => i.type === "deployment.promote" && i.environment === "production",
    },
    {
      id: "allow.git-branch-create",
      description: "Allow branch creation when recovery exists and evidence is solid.",
      effect: "allow",
      reason: "Allow-rule: branch create on R2 with delete_branch recovery and tests+audit pass",
      match: (i) =>
        i.type === "git.branch.create" &&
        i.riskClass === "R2" &&
        (i.evidence.testsPass ?? false) &&
        (i.evidence.dependencyAudit ?? false) &&
        typeof i.intent.branch === "string" &&
        i.intent.branch.startsWith("aevum/") &&
        i.intent.recoveryStrategy === "delete_branch",
    },
    {
      id: "default-deny",
      description: "Deny when no rule matches.",
      effect: "deny",
      reason: "No matching allow rule — fail-closed",
      match: () => true,
    },
  ];
  return {
    rules,
    version: "aevum.policy/v1.0.0",
    bundleDigest() {
      return "sha256:" + sha256(canonicalJsonStringify({ version: this.version, rules: rules.map((r) => ({ id: r.id, effect: r.effect })) }));
    },
  };
}

export function evaluatePolicy(bundle: PolicyBundle, input: PolicyInput): PolicyDecision {
  const bundleDigest = bundle.bundleDigest();
  for (const rule of bundle.rules) {
    if (rule.match(input)) {
      const id = "pdec_" + sha256(canonicalJsonStringify({ bundleDigest, ruleId: rule.id, input })).slice(0, 24);
      return {
        id,
        effect: rule.effect,
        ruleId: rule.id,
        reason: rule.reason,
        bundleDigest,
        decidedAt: new Date().toISOString(),
      };
    }
  }
  // Unreachable because of default-deny, but TS wants it.
  return {
    id: "pdec_unreachable",
    effect: "deny",
    ruleId: "default-deny",
    reason: "unreachable",
    bundleDigest,
    decidedAt: new Date().toISOString(),
  };
}
