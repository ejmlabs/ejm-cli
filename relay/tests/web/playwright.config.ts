import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  testMatch: ["browser.spec.mjs", "oauth.spec.mjs"],
  globalSetup: "./setup.mjs",
  workers: 1,
  retries: 0,
  timeout: 75000,
  expect: { timeout: 12000 },
  reporter: "line",
  outputDir: "../../web/test-results",
  use: { channel: "chrome", headless: true, trace: "off", screenshot: "off", video: "off" },
});
