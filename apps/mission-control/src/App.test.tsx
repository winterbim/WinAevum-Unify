import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import App from "./App";

describe("Mission Control", () => {
  it("renders sidebar brand", () => {
    render(<App />);
    expect(screen.getByText(/Aevum Unify/i)).toBeDefined();
  });
});
