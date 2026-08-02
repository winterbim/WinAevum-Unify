// Council Fabric — agent registry + team selector (M5).
// Source: AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §14.

import type { MissionId } from "./ids.js";
import type { RiskClass } from "./constitution.js";
import { RISK_RANK } from "./constitution.js";

export type FunctionKey =
  | "recon"
  | "planner"
  | "producer"
  | "falsifier"
  | "verifier"
  | "guardian"
  | "arbiter"
  | "observer";

export interface ModelRef {
  provider: string;     // e.g. "anthropic", "openai", "mistral", "local"
  family: string;       // e.g. "claude-3.7", "gpt-4o", "mistral-small"
  version: string;      // semver
}

export interface AgentDefinition {
  id: string;
  function: FunctionKey;
  model: ModelRef;
  domains: string[];             // allowed domains: "code" | "docs" | "infra" etc.
  costPer1kTokensEur: number;
  skillTags: string[];
  notes?: string;
}

export interface AssembleCouncilInput {
  missionId: MissionId;
  preliminaryRisk: RiskClass;
  domains: string[];
  budget: { moneyEur: number; wallClockSeconds: number; tokens: number };
  independenceRequired: boolean;
  registry: AgentDefinition[];
}

export interface CouncilMember {
  agentId: string;
  function: FunctionKey;
  model: ModelRef;
  instanceId: string;            // ephemeral per mission id
  reasoningBudgetTokens: number;
}

export interface Council {
  missionId: MissionId;
  members: CouncilMember[];
  totalEstimatedCostEur: number;
  independenceAchieved: boolean;
}

export function defaultRegistry(): AgentDefinition[] {
  return [
    { id: "recon-claude", function: "recon", model: { provider: "anthropic", family: "claude-3-7-sonnet", version: "2026-04-01" }, domains: ["*"], costPer1kTokensEur: 0.003, skillTags: ["code", "docs", "infra", "search"], notes: "systematic inspector" },
    { id: "recon-mistral", function: "recon", model: { provider: "mistral", family: "mistral-small", version: "2026-04-01" }, domains: ["*"], costPer1kTokensEur: 0.0005, skillTags: ["code", "docs"] },
    { id: "planner-gpt", function: "planner", model: { provider: "openai", family: "gpt-4o", version: "2026-04-01" }, domains: ["code", "infra"], costPer1kTokensEur: 0.005, skillTags: ["decomposition", "risk"] },
    { id: "producer-claude", function: "producer", model: { provider: "anthropic", family: "claude-3-7-sonnet", version: "2026-04-01" }, domains: ["code", "docs"], costPer1kTokensEur: 0.003, skillTags: ["typescript", "rust", "patching"] },
    { id: "producer-local-codellama", function: "producer", model: { provider: "local", family: "codellama-7b", version: "2026-04-01" }, domains: ["code", "docs"], costPer1kTokensEur: 0.0, skillTags: ["small-patches"] },
    { id: "producer-llama-generalist", function: "producer", model: { provider: "local", family: "llama-3.3-70b", version: "2026-04-01" }, domains: ["*"], costPer1kTokensEur: 0.0, skillTags: ["generalist", "docs"] },
    { id: "falsifier-gemini", function: "falsifier", model: { provider: "google", family: "gemini-2.0-flash", version: "2026-04-01" }, domains: ["*"], costPer1kTokensEur: 0.001, skillTags: ["counterexample-search", "fuzz-thinking"] },
    { id: "falsifier-deepseek", function: "falsifier", model: { provider: "deepseek", family: "deepseek-r1", version: "2026-04-01" }, domains: ["*"], costPer1kTokensEur: 0.0008, skillTags: ["reasoning-heavy"] },
    { id: "verifier-gpt", function: "verifier", model: { provider: "openai", family: "gpt-4o", version: "2026-04-01" }, domains: ["*"], costPer1kTokensEur: 0.005, skillTags: ["static-analysis", "diff-review"] },
    { id: "verifier-llama", function: "verifier", model: { provider: "local", family: "llama-3.3-70b", version: "2026-04-01" }, domains: ["*"], costPer1kTokensEur: 0.0, skillTags: ["review", "policy-check"] },
    { id: "guardian-anthropic", function: "guardian", model: { provider: "anthropic", family: "claude-3-7-sonnet", version: "2026-04-01" }, domains: ["*"], costPer1kTokensEur: 0.003, skillTags: ["security", "data-classification"] },
    { id: "guardian-local", function: "guardian", model: { provider: "local", family: "secguard-v2", version: "2026-04-01" }, domains: ["*"], costPer1kTokensEur: 0.0, skillTags: ["static-rules"] },
    { id: "arbiter-mistral", function: "arbiter", model: { provider: "mistral", family: "mistral-large-2", version: "2026-04-01" }, domains: ["*"], costPer1kTokensEur: 0.002, skillTags: ["deterministic-resolution"] },
    { id: "observer-local", function: "observer", model: { provider: "local", family: "aevum-observer", version: "2026-04-01" }, domains: ["*"], costPer1kTokensEur: 0.0, skillTags: ["metrics"] },
  ];
}

function pickCheapest(candidates: AgentDefinition[], _budgetRemaining: number): AgentDefinition {
  if (candidates.length === 0) throw new Error("pickCheapest: empty candidate set");
  const sorted = [...candidates].sort((a, b) => a.costPer1kTokensEur - b.costPer1kTokensEur);
  const top = sorted[0];
  if (!top) throw new Error("pickCheapest: empty candidate set after sort");
  return top;
}

function pickInDomain(candidates: AgentDefinition[], domains: string[]): AgentDefinition[] {
  if (!candidates.length) return [];
  if (domains.length === 0 || domains.includes("*")) return candidates;
  const exact = candidates.filter((c) =>
    c.domains.includes("*") || c.domains.some((d) => domains.includes(d)),
  );
  // If no exact match but at least one candidate has the wildcard domain,
  // fall back to wildcard candidates rather than failing.
  if (exact.length === 0) {
    return candidates.filter((c) => c.domains.includes("*"));
  }
  return exact;
}

function pickForFunction(
  fn: FunctionKey,
  registry: AgentDefinition[],
  domains: string[],
  excludes: string[] = [],
): AgentDefinition {
  const all = registry.filter((a) => a.function === fn && !excludes.includes(a.id));
  if (!all.length) throw new Error(`Council: no agent available for function ${fn} (registry=${registry.length}, excluded=${excludes.length})`);
  const eligible = pickInDomain(all, domains);
  if (!eligible.length) throw new Error(`Council: no in-domain agent for function ${fn} (domains=${domains.join(",")})`);
  return pickCheapest(eligible, Infinity);
}

export function assembleCouncil(input: AssembleCouncilInput): Council {
  const requires: FunctionKey[] =
    RISK_RANK[input.preliminaryRisk] >= RISK_RANK.R3
      ? ["recon", "planner", "producer", "falsifier", "verifier", "guardian", "arbiter"]
      : RISK_RANK[input.preliminaryRisk] === RISK_RANK.R2
        ? ["recon", "producer", "falsifier", "verifier", "guardian"]
        : ["recon", "producer", "verifier"];

  const members: CouncilMember[] = [];
  const excludesByProvider: string[] = [];

  for (const fn of requires) {
    let chosen: AgentDefinition;
    if (input.independenceRequired && (fn === "producer" || fn === "verifier") && members.length > 0) {
      const previousProvider = members.find((m) => m.function === "producer")?.model.provider;
      const candidates = input.registry.filter((a) => a.function === fn && (!previousProvider || a.model.provider !== previousProvider));
      const eligible = pickInDomain(candidates, input.domains);
      if (eligible.length > 0) {
        chosen = pickCheapest(eligible, Infinity);
      } else {
        chosen = pickForFunction(fn, input.registry, input.domains, excludesByProvider);
      }
    } else {
      chosen = pickForFunction(fn, input.registry, input.domains, excludesByProvider);
    }
    members.push({
      agentId: chosen.id,
      function: chosen.function,
      model: chosen.model,
      instanceId: `inst_${input.missionId}_${chosen.id}`,
      reasoningBudgetTokens: Math.min(20000, Math.floor(input.budget.tokens / requires.length)),
    });
    excludesByProvider.push(chosen.model.provider);
  }

  // Observer is always present and costs zero. Prefer the caller-supplied
  // registry, fall back to defaultRegistry() if absent.
  const observerPool = [...input.registry];
  let observer = observerPool.find((a) => a.id === "observer-local") ?? defaultRegistry().find((a) => a.id === "observer-local")!;
  members.push({
    agentId: observer.id,
    function: observer.function,
    model: observer.model,
    instanceId: `inst_${input.missionId}_observer`,
    reasoningBudgetTokens: 0,
  });

  const costLookup = new Map(input.registry.map((a) => [a.id, a.costPer1kTokensEur]));
  for (const a of defaultRegistry()) if (!costLookup.has(a.id)) costLookup.set(a.id, a.costPer1kTokensEur);
  const estimatedCost = members.reduce((s, m) => {
    const c = costLookup.get(m.agentId);
    const cost = typeof c === "number" ? c : 0;
    return s + (cost * m.reasoningBudgetTokens) / 1000;
  }, 0);

  return {
    missionId: input.missionId,
    members,
    totalEstimatedCostEur: Number(estimatedCost.toFixed(4)),
    independenceAchieved: input.independenceRequired
      ? (() => {
          const producer = members.find((m) => m.function === "producer");
          const verifier = members.find((m) => m.function === "verifier");
          return !!(producer && verifier && producer.model.provider !== verifier.model.provider);
        })()
      : true,
  };
}
