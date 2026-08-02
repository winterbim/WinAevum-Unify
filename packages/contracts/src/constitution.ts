// Mission Constitution — types, validator, scope-diff, digest (M1).
// Source: AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §10, §13, §22 (state machine).

import { canonicalJsonStringify, sha256 } from "./canonical.js";
import type { MissionId } from "./ids.js";

export type RiskClass = "R0" | "R1" | "R2" | "R3" | "R4" | "R5";
export type RiskRank = Record<RiskClass, number>;
export const RISK_RANK: RiskRank = { R0: 0, R1: 1, R2: 2, R3: 3, R4: 4, R5: 5 };

export type ProductionEffects = "forbidden" | "approval-required" | "allowed";
export type ExternalMessages = "forbidden" | "approval-required" | "allowed";

export interface MissionConstitution {
  schemaVersion: "aevum.mission-constitution/v1";
  missionId: MissionId;
  version: number;
  createdBy: string;
  objective: {
    statement: string;
    successOutcomes: string[];   // must have ≥1
  };
  scope: {
    repositories: string[];
    branchesRead: string[];
    branchesWrite: string[];
    allowedPaths: string[];
    deniedPaths: string[];
  };
  constraints: {
    productionEffects: ProductionEffects;
    secretExposure: "forbidden" | "approval-required" | "allowed";
    externalMessages: ExternalMessages;
  };
  budgets: {
    moneyEurMax: number;
    wallClockSecondsMax: number;
    toolCallsMax: number;
  };
  riskPolicy: {
    maxAutonomousRisk: RiskClass;
    humanApprovalFrom: RiskClass;
  };
  verification: {
    producerMustNotBeOnlyVerifier: boolean;
    requiredChecks: string[];
  };
  recovery: {
    requireSnapshot: boolean;
    requireRollbackTestFor: RiskClass[];
  };
  notBefore: string;
  expiresAt: string;
}

export interface ValidationError {
  path: string;
  message: string;
}

export interface ValidationOk { ok: true }
export interface ValidationFail { ok: false; errors: ValidationError[] }
export type ValidationResult = ValidationOk | ValidationFail;

const isString = (v: unknown): v is string => typeof v === "string";
const isNumber = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
const isArray = (v: unknown): v is unknown[] => Array.isArray(v);
const isPlainObject = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);

export function validateConstitution(input: unknown): ValidationResult {
  const errors: ValidationError[] = [];

  if (!isPlainObject(input)) {
    return { ok: false, errors: [{ path: "", message: "constitution must be an object" }] };
  }
  const c = input as Partial<MissionConstitution>;

  if (c.schemaVersion !== "aevum.mission-constitution/v1") {
    errors.push({ path: "schemaVersion", message: `unknown schema_version: ${c.schemaVersion}` });
  }
  if (!isString(c.missionId) || !c.missionId.startsWith("mis_")) {
    errors.push({ path: "missionId", message: "missionId must be a string starting with `mis_`" });
  }
  if (!isNumber(c.version) || c.version < 1) {
    errors.push({ path: "version", message: "version must be a positive integer" });
  }
  if (!isString(c.createdBy) || c.createdBy.length === 0) {
    errors.push({ path: "createdBy", message: "createdBy is required" });
  }
  if (!isPlainObject(c.objective)) {
    errors.push({ path: "objective", message: "objective must be an object" });
  } else {
    if (!isString(c.objective.statement) || c.objective.statement.length === 0) {
      errors.push({ path: "objective.statement", message: "objective.statement is required" });
    }
    if (!isArray(c.objective.successOutcomes) || c.objective.successOutcomes.length === 0) {
      errors.push({ path: "objective.successOutcomes", message: "must contain at least one success outcome" });
    }
  }
  if (!isPlainObject(c.scope)) {
    errors.push({ path: "scope", message: "scope must be an object" });
  } else {
    for (const k of ["repositories", "branchesRead", "branchesWrite", "allowedPaths", "deniedPaths"] as const) {
      if (!isArray(c.scope[k])) errors.push({ path: `scope.${k}`, message: "must be a list" });
    }
  }
  if (!isPlainObject(c.budgets)) {
    errors.push({ path: "budgets", message: "budgets must be an object" });
  } else {
    if (!isNumber(c.budgets.moneyEurMax) || c.budgets.moneyEurMax < 0) {
      errors.push({ path: "budgets.moneyEurMax", message: "moneyEurMax must be ≥0" });
    }
    if (!isNumber(c.budgets.wallClockSecondsMax) || c.budgets.wallClockSecondsMax <= 0) {
      errors.push({ path: "budgets.wallClockSecondsMax", message: "wallClockSecondsMax must be >0" });
    }
    if (!isNumber(c.budgets.toolCallsMax) || c.budgets.toolCallsMax <= 0) {
      errors.push({ path: "budgets.toolCallsMax", message: "toolCallsMax must be >0" });
    }
  }
  if (!isPlainObject(c.riskPolicy)) {
    errors.push({ path: "riskPolicy", message: "riskPolicy must be an object" });
  } else {
    const validRisk = ["R0", "R1", "R2", "R3", "R4", "R5"];
    if (!validRisk.includes(c.riskPolicy.maxAutonomousRisk)) {
      errors.push({ path: "riskPolicy.maxAutonomousRisk", message: "must be R0..R5" });
    }
    if (!validRisk.includes(c.riskPolicy.humanApprovalFrom)) {
      errors.push({ path: "riskPolicy.humanApprovalFrom", message: "must be R0..R5" });
    }
    if (
      validRisk.includes(c.riskPolicy.maxAutonomousRisk) &&
      validRisk.includes(c.riskPolicy.humanApprovalFrom) &&
      RISK_RANK[c.riskPolicy.maxAutonomousRisk as RiskClass] >
        RISK_RANK[c.riskPolicy.humanApprovalFrom as RiskClass]
    ) {
      errors.push({
        path: "riskPolicy",
        message: "maxAutonomousRisk cannot be greater than humanApprovalFrom (humans must gate stricter)",
      });
    }
  }

  return errors.length === 0 ? { ok: true } : { ok: false, errors };
}

export type ScopeDiffKind = "unchanged" | "narrowing" | "expanding" | "conflicting";

export interface ScopeDiff {
  kind: ScopeDiffKind;
  addedBranchesRead?: string[];
  addedBranchesWrite?: string[];
  removedBranchesWrite?: string[];
  addedRepositories?: string[];
  riskPolicyChanges?: { from: RiskClass; to: RiskClass };
}

const setOf = (arr: string[] | undefined): Set<string> => new Set(arr ?? []);

function branchesExpanded(prev: string[], next: string[]): { added: string[]; removed: string[] } {
  const p = setOf(prev);
  const n = setOf(next);
  const added: string[] = [];
  const removed: string[] = [];
  n.forEach((x) => { if (!p.has(x)) added.push(x); });
  p.forEach((x) => { if (!n.has(x)) removed.push(x); });
  return { added, removed };
}

export function diffConstitutionScope(prev: MissionConstitution, next: MissionConstitution): ScopeDiff {
  if (prev.version === next.version && canonicalJsonStringify(prev) === canonicalJsonStringify(next)) {
    return { kind: "unchanged" };
  }
  const rd = branchesExpanded(prev.scope.branchesRead, next.scope.branchesRead);
  const wd = branchesExpanded(prev.scope.branchesWrite, next.scope.branchesWrite);
  const rp = branchesExpanded(prev.scope.repositories, next.scope.repositories);

  const riskBumped =
    RISK_RANK[next.riskPolicy.maxAutonomousRisk] > RISK_RANK[prev.riskPolicy.maxAutonomousRisk] ||
    RISK_RANK[next.riskPolicy.humanApprovalFrom] < RISK_RANK[prev.riskPolicy.humanApprovalFrom];

  // Conflict if "conflicting"—e.g. a denied path appearing in allowed, or branchesWrite allowing main.
  if (next.scope.branchesWrite.some((b) => b === "main" || b.startsWith("/main") )) {
    return {
      kind: "conflicting",
      ...(rd.added.length ? { addedBranchesRead: rd.added } : {}),
      ...(wd.added.length ? { addedBranchesWrite: wd.added } : {}),
      ...(wd.removed.length ? { removedBranchesWrite: wd.removed } : {}),
    };
  }

  const expanding =
    rd.added.length > 0 ||
    wd.added.length > 0 ||
    rp.added.length > 0 ||
    riskBumped;

  if (expanding) {
    const diff: ScopeDiff = { kind: "expanding" };
    if (rd.added.length) diff.addedBranchesRead = rd.added;
    if (wd.added.length) diff.addedBranchesWrite = wd.added;
    if (rp.added.length) diff.addedRepositories = rp.added;
    if (riskBumped) diff.riskPolicyChanges = { from: prev.riskPolicy.maxAutonomousRisk, to: next.riskPolicy.maxAutonomousRisk };
    return diff;
  }

  return {
    kind: "narrowing",
    ...(wd.removed.length ? { removedBranchesWrite: wd.removed } : {}),
  };
}

export function computeConstitutionDigest(c: MissionConstitution): string {
  // Use canonical JSON without the volatile timestamps for stability.
  const stable = {
    schemaVersion: c.schemaVersion,
    missionId: c.missionId,
    version: c.version,
    createdBy: c.createdBy,
    objective: c.objective,
    scope: c.scope,
    constraints: c.constraints,
    budgets: c.budgets,
    riskPolicy: c.riskPolicy,
    verification: c.verification,
    recovery: c.recovery,
  };
  return "sha256:" + sha256(canonicalJsonStringify(stable));
}
