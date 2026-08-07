import { describe, expect, it } from "vitest";
import {
  isFactActiveAt,
  isFactCurrent,
  isPrimaryEvidenceEligible,
  mayAuthorizeAction,
  type Episode,
  type Fact,
} from "./temporal-graph.js";

const baseFact: Fact = {
  id: "f1",
  kind: "relates_to",
  sourceNodeId: "a",
  targetNodeId: "b",
  name: "USES",
  fact: "a uses b",
  epistemic: "fact",
  episodeIds: ["ep1"],
  validAt: "2026-08-02T10:00:00Z",
  createdAt: "2026-08-02T10:00:01Z",
  groupId: "g",
  missionId: "mis_1",
};

describe("temporal-graph", () => {
  it("only facts may authorize", () => {
    expect(mayAuthorizeAction("fact")).toBe(true);
    expect(mayAuthorizeAction("hypothesis")).toBe(false);
    expect(mayAuthorizeAction("inference")).toBe(false);
  });

  it("bi-temporal window is half-open [validAt, invalidAt)", () => {
    const f: Fact = {
      ...baseFact,
      invalidAt: "2026-08-02T12:00:00Z",
    };
    expect(isFactActiveAt(f, "2026-08-02T11:00:00Z")).toBe(true);
    expect(isFactActiveAt(f, "2026-08-02T12:00:00Z")).toBe(false);
    expect(isFactActiveAt(f, "2026-08-02T09:00:00Z")).toBe(false);
  });

  it("expired facts remain historically queryable via event time", () => {
    const f: Fact = {
      ...baseFact,
      invalidAt: "2026-08-02T12:00:00Z",
      expiredAt: "2026-08-02T12:00:01Z",
    };
    expect(isFactActiveAt(f, "2026-08-02T11:00:00Z")).toBe(true);
    expect(isFactCurrent(f)).toBe(false);
  });

  it("only attested+digest episodes are primary-evidence eligible", () => {
    const ok: Episode = {
      id: "ep1",
      missionId: "m",
      groupId: "g",
      source: "attested",
      content: "{}",
      contentDigest: "sha256:abc",
      validAt: "2026-08-02T10:00:00Z",
      createdAt: "2026-08-02T10:00:01Z",
    };
    const llm: Episode = { ...ok, source: "text", contentDigest: undefined };
    expect(isPrimaryEvidenceEligible(ok)).toBe(true);
    expect(isPrimaryEvidenceEligible(llm)).toBe(false);
  });
});
