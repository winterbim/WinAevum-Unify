import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: false,
    pool: "threads",
    fileParallelism: false,
    maxWorkers: 1,
    minWorkers: 1,
  },
});
