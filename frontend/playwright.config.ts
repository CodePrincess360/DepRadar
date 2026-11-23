import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "**/*.pw.ts",
  reporter: "html",
  use: { baseURL: "http://localhost:4173" },
  webServer: {
    command: "npm run dev -- --host localhost --port 4173 --strictPort",
    port: 4173,
    reuseExistingServer: !process.env.CI,
  },
});
