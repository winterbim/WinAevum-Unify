import { describe, it, expect } from "vitest";
import { isIdOf, IdPrefix } from "./ids.js";

describe("ids", () => {
  it("rejects strings without a prefix", () => {
    expect(isIdOf<`mis_${string}`>("abc", IdPrefix.Mission)).toBe(false);
  });

  it("rejects empty payload after the prefix", () => {
    expect(isIdOf<`mis_${string}`>("mis_", IdPrefix.Mission)).toBe(false);
  });

  it("accepts a non-empty ULID-like body", () => {
    expect(isIdOf<`mis_${string}`>("mis_01Jabc", IdPrefix.Mission)).toBe(true);
  });

  it("rejects non-string input", () => {
    expect(isIdOf<`mis_${string}`>(42, IdPrefix.Mission)).toBe(false);
    expect(isIdOf<`mis_${string}`>(null, IdPrefix.Mission)).toBe(false);
  });
});
