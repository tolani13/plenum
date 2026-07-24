import { defineConfig, devices } from "@playwright/test";

// The responsive tripwire (gate P2-2, automated half). Chromium only; the dev
// server is started/reused on 127.0.0.1:5177. The API must be running
// separately on 127.0.0.1:5777 (the dev server only proxies to it) — the spec
// says so in its failure message if /api is unreachable.
export default defineConfig({
  testDir: ".",
  testMatch: /(tripwire|dragproof)\.spec\.ts/,
  fullyParallel: false,
  workers: 1,
  reporter: "list",
  timeout: 90_000,
  use: {
    baseURL: "http://127.0.0.1:5177",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "npm run dev",
    url: "http://127.0.0.1:5177",
    reuseExistingServer: true,
    timeout: 60_000,
  },
});
