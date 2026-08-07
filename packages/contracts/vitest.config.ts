import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
    // Constrain workers — avoids tinypool min/max conflicts in sandbox/CI.
    fileParallelism: false,
    maxWorkers: 1,
    minWorkers: 1,
  },
});
