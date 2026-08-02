import { describe, it, expect } from "vitest";
import { assembleCouncil, defaultRegistry, type AgentDefinition, type FunctionKey } from "./agents.js";

describe("council", () => {
  it("R0 documentary mission: minimal team", () => {
    const team = assembleCouncil({
      missionId: "mis_01" as any,
      preliminaryRisk: "R0",
      domains: ["docs"],
      budget: { moneyEur: 1, wallClockSeconds: 600, tokens: 10000 },
      independenceRequired: false,
      registry: defaultRegistry(),
    });
    const names = team.members.map((m) => m.function).sort();
    expect(names).toContain("recon");
    expect(names).toContain("producer");
    expect(names).toContain("verifier");
  });

  it("R3 code mission: includes planner, falsifier, guardian", () => {
    const team = assembleCouncil({
      missionId: "mis_01" as any,
      preliminaryRisk: "R3",
      domains: ["code"],
      budget: { moneyEur: 5, wallClockSeconds: 3600, tokens: 500000 },
      independenceRequired: true,
      registry: defaultRegistry(),
    });
    const names = team.members.map((m) => m.function).sort();
    expect(names).toEqual(["arbiter", "falsifier", "guardian", "observer", "planner", "producer", "recon", "verifier"]);
    expect(team.independenceAchieved).toBe(true);
  });

  it("Diversity Gate: producer and verifier are not from the same provider", () => {
    const team = assembleCouncil({
      missionId: "mis_01" as any,
      preliminaryRisk: "R3",
      domains: ["code"],
      budget: { moneyEur: 5, wallClockSeconds: 3600, tokens: 500000 },
      independenceRequired: true,
      registry: defaultRegistry(),
    });
    const producer = team.members.find((m) => m.function === "producer")!;
    const verifier = team.members.find((m) => m.function === "verifier")!;
    expect(producer.model.provider).not.toBe(verifier.model.provider);
  });

  it("throws when registry contains no candidate for required function", () => {
    const stripped: AgentDefinition[] = defaultRegistry().filter((a) => a.function !== "falsifier");
    expect(() =>
      assembleCouncil({
        missionId: "mis_01" as any,
        preliminaryRisk: "R3",
        domains: ["code"],
        budget: { moneyEur: 5, wallClockSeconds: 3600, tokens: 500000 },
        independenceRequired: true,
        registry: stripped,
      })
    ).toThrow(/falsifier/);
  });

  it("each member carries the canonical function key from the blueprint", () => {
    const team = assembleCouncil({
      missionId: "mis_01" as any,
      preliminaryRisk: "R2",
      domains: ["code"],
      budget: { moneyEur: 2, wallClockSeconds: 1800, tokens: 100000 },
      independenceRequired: false,
      registry: defaultRegistry(),
    });
    const validFunctions: FunctionKey[] = ["recon", "planner", "producer", "falsifier", "verifier", "guardian", "arbiter", "observer"];
    for (const m of team.members) {
      expect(validFunctions).toContain(m.function);
    }
  });
});
