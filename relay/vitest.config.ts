import { cloudflareTest, readD1Migrations } from "@cloudflare/vitest-plugin";
import { defineConfig } from "vitest/config";
export default defineConfig(async () => ({
  plugins: [cloudflareTest({ wrangler: { configPath: "./wrangler.jsonc" }, miniflare: { bindings: {
    RETENTION_CONFIG: JSON.stringify({ approved: true, version: "fixture-v1", approvedBy: "fixture-operator", accountActivePurpose: "Authenticate active relay accounts and enforce access, billing and user rights.", unadmittedDays: 7, accountClosedDays: 30, providerIdentityDays: 30, paymentDays: 365, externalNotice: "https://relay.example.invalid/terms", legalHoldPolicy: "Fixture holds require reason, evidence and expiry." }),
    TEST_MIGRATIONS: await readD1Migrations("./migrations"),
    RELAY_ORIGIN: "https://relay.example.invalid", TERMS_VERSION: "fixture-v1", ADMISSION_ENABLED: "true",
    BETTER_AUTH_SECRET: "local-fixture-only-at-least-thirty-two-characters-long", GITHUB_CLIENT_ID: "local-fixture", GITHUB_CLIENT_SECRET: "local-fixture",
    STRIPE_SECRET_KEY: "sk_test_local_fixture", STRIPE_WEBHOOK_SECRET: "whsec_local_fixture", STRIPE_MONTHLY_PRICE: "price_monthly_fixture", STRIPE_YEARLY_PRICE: "price_yearly_fixture",
  } } })],
  test: { include: ["tests/**/*.test.ts"], fileParallelism: false },
}));
