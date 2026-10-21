import { env } from "cloudflare:workers";
import { applyD1Migrations, reset } from "cloudflare:test";
import { beforeEach, expect, it, vi } from "vitest";
import Stripe from "stripe";
import worker from "../src/index.js";
import { authority } from "../src/quota.js";
import { operationalCleanup } from "../src/cleanup.js";
import { createAuth, verifiedGithubIdentity } from "../src/auth.js";
import type { Env } from "../src/types.js";
const bindings = env as unknown as Env & { TEST_MIGRATIONS: { name: string; queries: string[] }[] };
const origin = bindings.RELAY_ORIGIN;
const sdk = new Stripe("sk_test_local_fixture");
let providerState = "active";
let providerExists = false;
let subscriptionsHaveMore = false;
let checkoutStatus = "open";
let checkoutSubscriptionId: string | null = "sub_local_fixture";
let checkoutListVisible = true;
let checkoutListHasMore = false;
let checkoutLists = 0;
let checkoutExpirations = 0;
let checkoutCompletionRace = false;
let subscriptionCancellations = 0;
let cancelAtPeriodEnd = false;
let providerUntil = Math.floor(Date.now() / 1000) + 3600;
let requestCount = 0;
let providerFailure = false;
let refunded = true;
let disputeState = "needs_response";
let ownerId = "";
let observedCheckout = "";
let observedCancellation = "";
let checkoutCreates = 0;
let checkoutKeys: (string | null)[] = [];
let checkoutParameters: string[] = [];
let checkoutAccepted = 0;
let checkoutCreationDelay = 2;
let checkoutFailure: { status: number; type: string; param: string } | "network" | null = null;
const customerId = "cus_local_fixture";
function checkoutSession(status: string): Record<string, unknown> {
  return { id: "cs_test_fixture", object: "checkout.session", customer: customerId, mode: "subscription", status, subscription: checkoutSubscriptionId, metadata: { relay_account_id: ownerId, relay_checkout_key: checkoutKeys.at(-1) }, url: status === "open" ? "https://checkout.stripe.com/c/pay/cs_test_fixture" : null };
}
function mockProvider(): void {
  vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
    const url = new URL(typeof input === "string" ? input : input instanceof Request ? input.url : input.toString());
    requestCount++;
    if (url.origin !== "https://api.stripe.com") throw new Error("unexpected_fixture_network");
    if (providerFailure) return Response.json({ error: { type: "api_error", message: "fixture temporarily unavailable" } }, { status: 503 });
    if (url.pathname === "/v1/customers") return Response.json({ id: customerId, object: "customer" });
    if (url.pathname === "/v1/checkout/sessions" && init?.method === "POST") {
      checkoutCreates++; checkoutKeys.push(new Headers(init.headers).get("idempotency-key")); observedCheckout = String(init.body); checkoutParameters.push(observedCheckout);
      if (checkoutFailure === "network") throw new TypeError("fixture connection lost");
      if (checkoutFailure) return Response.json({ error: { type: checkoutFailure.type, param: checkoutFailure.param, message: "Fixture provider rejection" } }, { status: checkoutFailure.status });
      const expiresAt = Number(new URLSearchParams(observedCheckout).get("expires_at"));
      const createdAt = Math.floor(Date.now() / 1000) + checkoutCreationDelay;
      if (expiresAt < createdAt + 1800 || expiresAt > createdAt + 86400) return Response.json({ error: { type: "invalid_request_error", param: "expires_at", message: "Fixture expiry outside the creation window" } }, { status: 400 });
      checkoutAccepted++; return Response.json(checkoutSession("open"));
    }
    if (url.pathname === "/v1/checkout/sessions") { checkoutLists++; expect(url.searchParams.get("limit")).toBe("20"); return Response.json({ object: "list", has_more: checkoutListHasMore, data: checkoutListVisible ? [checkoutSession(checkoutStatus)] : [] }); }
    if (url.pathname === "/v1/checkout/sessions/cs_test_fixture/expire") { if (checkoutCompletionRace) { checkoutStatus = "complete"; providerExists = true; checkoutCompletionRace = false; return Response.json({ error: { type: "invalid_request_error", message: "Fixture completed before expiration" } }, { status: 400 }); } checkoutStatus = "expired"; checkoutExpirations++; return Response.json(checkoutSession("expired")); }
    if (url.pathname === "/v1/checkout/sessions/cs_test_fixture") return Response.json(checkoutSession(checkoutStatus));
    if (url.pathname === "/v1/subscriptions/sub_local_fixture") { if (init?.method === "DELETE") { providerState = "canceled"; subscriptionCancellations++; } if (init?.method === "POST") { observedCancellation = String(init.body); cancelAtPeriodEnd = true; } return Response.json({ id: "sub_local_fixture", customer: customerId, status: providerState, cancel_at_period_end: cancelAtPeriodEnd }); }
    if (url.pathname === "/v1/billing_portal/sessions") return Response.json({ id: "bps_fixture", url: "https://billing.stripe.com/p/session/test_fixture" });
    if (url.pathname === "/v1/subscriptions") return Response.json({ object: "list", has_more: subscriptionsHaveMore, data: providerExists ? [{ id: "sub_local_fixture", object: "subscription", customer: customerId, metadata: { relay_account_id: ownerId }, created: 1, status: providerState, cancel_at_period_end: cancelAtPeriodEnd, latest_invoice: "in_current_fixture", items: { data: [{ id: "si_fixture", price: { id: "price_monthly_fixture" }, current_period_end: providerUntil }] } }] : [] });
    if (url.pathname === "/v1/charges/ch_fixture") return Response.json({ id: "ch_fixture", customer: customerId, refunded, payment_intent: "pi_fixture" });
    if (url.pathname === "/v1/invoice_payments") return Response.json({ object: "list", data: [{ id: "inpay_fixture", invoice: "in_current_fixture", payment: { type: "payment_intent", payment_intent: "pi_fixture" } }], has_more: false });
    if (url.pathname === "/v1/disputes/dp_fixture") return Response.json({ id: "dp_fixture", charge: "ch_fixture", status: disputeState });
    throw new Error("unexpected_fixture_endpoint");
  });
}
async function call(path: string, token: string, body: unknown): Promise<Response> { return worker.fetch(new Request(origin + path, { method: "POST", headers: { authorization: `Bearer ${token}`, "content-type": "application/json" }, body: JSON.stringify(body) }), bindings); }
async function account(): Promise<string> {
  const context = await createAuth(bindings).$context;
  const user = await context.internalAdapter.createUser({ email: "billing@example.invalid", name: "Fixture", emailVerified: true }, { method: "api" });
  const session = await context.internalAdapter.createSession(user.id); ownerId = user.id;
  expect((await call("/v1/account/terms", session.token, { version: bindings.TERMS_VERSION })).status).toBe(200);
  return session.token;
}
async function webhook(eventId: string, type = "customer.subscription.updated", object: Record<string, unknown> = { id: "sub_local_fixture", customer: customerId }): Promise<Response> {
  providerExists = true;
  const payload = JSON.stringify({ id: eventId, object: "event", type, created: 1, livemode: false, data: { object } });
  const signature = await sdk.webhooks.generateTestHeaderStringAsync({ payload, secret: bindings.STRIPE_WEBHOOK_SECRET ?? "" });
  return worker.fetch(new Request(origin + "/v1/billing/webhook", { method: "POST", headers: { "stripe-signature": signature }, body: payload }), bindings);
}
async function plan(): Promise<Record<string, unknown> | null> { return bindings.DB.prepare("SELECT plan,status,paid_until FROM relay_billing WHERE account_id=?").bind(ownerId).first(); }
beforeEach(async () => { vi.restoreAllMocks(); await reset(); await applyD1Migrations(bindings.DB, bindings.TEST_MIGRATIONS); providerState = "active"; providerExists = false; subscriptionsHaveMore = false; checkoutStatus = "open"; checkoutSubscriptionId = "sub_local_fixture"; checkoutListVisible = true; checkoutListHasMore = false; checkoutLists = 0; checkoutExpirations = 0; checkoutCompletionRace = false; subscriptionCancellations = 0; cancelAtPeriodEnd = false; providerUntil = Math.floor(Date.now() / 1000) + 3600; requestCount = 0; providerFailure = false; refunded = true; disputeState = "needs_response"; observedCheckout = ""; observedCancellation = ""; checkoutCreates = 0; checkoutKeys = []; checkoutParameters = []; checkoutAccepted = 0; checkoutCreationDelay = 2; checkoutFailure = null; });

it("uses only authenticated admitted account and configured prices with hosted payment URLs", async () => {
  const token = await account(); mockProvider();
  expect((await call("/v1/billing/checkout", token, { plan: "monthly", accountId: "attacker", price: "price_attacker" })).status).toBe(400);
  expect(requestCount).toBe(0);
  const checkout = await call("/v1/billing/checkout", token, { plan: "monthly" });
  expect(checkout.status).toBe(200); expect(await checkout.json()).toMatchObject({ url: "https://checkout.stripe.com/c/pay/cs_test_fixture" });
  const parameters = new URLSearchParams(observedCheckout);
  expect(parameters.get("line_items[0][price]")).toBe("price_monthly_fixture");
  expect(parameters.get("metadata[relay_account_id]")).toBe(ownerId);
  expect(await plan()).toMatchObject({ plan: "free", paid_until: 0 });
  expect((await call("/v1/billing/portal", token, {})).status).toBe(200);
});
it("verifies untouched signatures and reconciles duplicate and unordered delivery without additive grants", async () => {
  const token = await account(); mockProvider(); await call("/v1/billing/checkout", token, { plan: "monthly" });
  const forged = await worker.fetch(new Request(origin + "/v1/billing/webhook", { method: "POST", headers: { "stripe-signature": "t=1,v1=forged" }, body: "{}" }), bindings);
  expect(forged.status).toBe(400);
  expect((await webhook("evt_current")).status).toBe(200);
  expect(await plan()).toMatchObject({ plan: "paid", paid_until: providerUntil * 1000 });
  const count = requestCount;
  expect((await webhook("evt_current")).status).toBe(200); expect(requestCount).toBe(count);
  providerState = "canceled";
  expect((await webhook("evt_older_delivery", "invoice.payment_succeeded")).status).toBe(200);
  expect(await plan()).toMatchObject({ plan: "free", paid_until: 0 });
  providerState = "active";
  expect((await webhook("evt_actual_current")).status).toBe(200);
  expect(await plan()).toMatchObject({ plan: "paid", paid_until: providerUntil * 1000 });
});
it("retains pending events through interruption and grants only after authoritative retry", async () => {
  const token = await account(); mockProvider(); await call("/v1/billing/checkout", token, { plan: "monthly" });
  providerFailure = true;
  expect((await webhook("evt_retry")).status).toBe(503);
  expect(await plan()).toMatchObject({ plan: "free", paid_until: 0 });
  expect(await bindings.DB.prepare("SELECT status FROM relay_billing_events WHERE id='evt_retry'").first()).toEqual({ status: "pending" });
  providerFailure = false;
  expect((await webhook("evt_retry")).status).toBe(200);
  expect(await plan()).toMatchObject({ plan: "paid", paid_until: providerUntil * 1000 });
}, 10000);
it("enforces payment failure, full refund and authoritative dispute resolution", async () => {
  const token = await account(); mockProvider(); await call("/v1/billing/checkout", token, { plan: "monthly" });
  await webhook("evt_active");
  providerState = "past_due"; await webhook("evt_payment_failed", "invoice.payment_failed");
  expect(await plan()).toMatchObject({ plan: "free", paid_until: 0 });
  providerState = "active"; await webhook("evt_paid");
  await webhook("evt_refund", "charge.refunded", { id: "ch_fixture", customer: customerId });
  expect(await plan()).toMatchObject({ plan: "free", status: "payment_hold" });
  refunded = false; await webhook("evt_refund_current", "charge.refunded", { id: "ch_fixture", customer: customerId });
  expect(await plan()).toMatchObject({ plan: "paid" });
  await webhook("evt_dispute", "charge.dispute.created", { id: "dp_fixture", charge: "ch_fixture" });
  expect(await plan()).toMatchObject({ plan: "free", status: "payment_hold" });
  disputeState = "won"; await webhook("evt_dispute_resolved", "charge.dispute.closed", { id: "dp_fixture", charge: "ch_fixture" });
  expect(await plan()).toMatchObject({ plan: "paid" });
});
it("selects verified primary GitHub email and discards enrichment", async () => {
  vi.spyOn(globalThis, "fetch").mockImplementation(async input => String(input).endsWith("/emails")
    ? Response.json([{ email: "public@example.invalid", primary: false, verified: true }, { email: "primary@example.invalid", primary: true, verified: true }])
    : Response.json({ id: 1234, email: "public@example.invalid", company: "Forbidden enrichment", location: "Forbidden location", name: "Provider name", avatar_url: "https://example.invalid/avatar" }));
  const result = await verifiedGithubIdentity("fixture-secret-never-logged");
  expect(result?.user).toEqual({ name: "Relay account", email: "primary@example.invalid", emailVerified: true, image: "" });
  vi.restoreAllMocks();
  vi.spyOn(globalThis, "fetch").mockImplementation(async input => String(input).endsWith("/emails") ? Response.json([{ email: "unverified@example.invalid", primary: true, verified: false }]) : Response.json({ id: 1234 }));
  expect(await verifiedGithubIdentity("fixture-secret-never-logged")).toBeNull();
});

it("reuses pending checkout and recovers an uncertain provider call with one persisted request", async () => {
  const token = await account(); mockProvider();
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  expect((await call("/v1/billing/checkout", token, { plan: "yearly" })).status).toBe(409);
  expect(checkoutCreates).toBe(1);
  const pending = await bindings.DB.prepare("SELECT checkout_key,checkout_expires_at FROM relay_billing WHERE account_id=?").bind(ownerId).first<{ checkout_key: string; checkout_expires_at: number }>();
  expect(pending!.checkout_expires_at - Date.now()).toBeGreaterThan(30 * 60_000);
  expect(pending!.checkout_expires_at - Date.now()).toBeLessThanOrEqual(60 * 60_000);
  await bindings.DB.prepare("UPDATE relay_billing SET checkout_session_id=NULL WHERE account_id=?").bind(ownerId).run();
  const clock = vi.spyOn(Date, "now").mockReturnValue(Date.now() + 10 * 60_000);
  try { expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200); }
  finally { clock.mockRestore(); }
  expect(checkoutCreates).toBe(2); expect(checkoutKeys[0]).toBeTruthy(); expect(checkoutKeys[1]).toBe(checkoutKeys[0]);
  expect(checkoutParameters[1]).toBe(checkoutParameters[0]);
  expect(await bindings.DB.prepare("SELECT checkout_key,checkout_expires_at FROM relay_billing WHERE account_id=?").bind(ownerId).first()).toEqual(pending);
});
it("keeps Checkout expiry valid at provider creation after request delay", async () => {
  const token = await account(); mockProvider(); checkoutCreationDelay = 30;
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  expect(checkoutAccepted).toBe(1);
  expect(Number(new URLSearchParams(observedCheckout).get("expires_at"))).toBeGreaterThanOrEqual(Math.floor(Date.now() / 1000) + checkoutCreationDelay + 1800);
});
it("releases only a fresh attempt after typed expiry validation rejects creation", async () => {
  const token = await account(); mockProvider(); checkoutCreationDelay = 31 * 60;
  const rejected = await call("/v1/billing/checkout", token, { plan: "monthly" });
  expect(rejected.status).toBe(503);
  expect(checkoutAccepted).toBe(0);
  expect(await bindings.DB.prepare("SELECT checkout_key,checkout_session_id,checkout_plan,checkout_expires_at FROM relay_billing WHERE account_id=?").bind(ownerId).first()).toEqual({ checkout_key: null, checkout_session_id: null, checkout_plan: null, checkout_expires_at: null });
  expect(await rejected.json()).toEqual({ error: { code: "checkout_unavailable" } });
  checkoutCreationDelay = 2;
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  expect(checkoutCreates).toBe(2); expect(checkoutAccepted).toBe(1); expect(new Set(checkoutKeys).size).toBe(2);
});
it.each([
  { name: "network failure", failure: "network" as const },
  { name: "server failure", failure: { status: 503, type: "api_error", param: "expires_at" } },
  { name: "conflict", failure: { status: 409, type: "invalid_request_error", param: "expires_at" } },
  { name: "idempotency failure", failure: { status: 400, type: "idempotency_error", param: "expires_at" } },
  { name: "other validation", failure: { status: 400, type: "invalid_request_error", param: "customer" } },
  { name: "non-validation expiry error", failure: { status: 400, type: "api_error", param: "expires_at" } },
])("retains the exact Checkout attempt after $name", async ({ failure }) => {
  const token = await account(); mockProvider(); checkoutFailure = failure;
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(503);
  const pending = await bindings.DB.prepare("SELECT checkout_key,checkout_session_id,checkout_plan,checkout_expires_at FROM relay_billing WHERE account_id=?").bind(ownerId).first<{ checkout_key: string; checkout_session_id: string | null; checkout_plan: string; checkout_expires_at: number }>();
  expect(pending).toMatchObject({ checkout_key: expect.any(String), checkout_session_id: null, checkout_plan: "monthly" });
  expect(checkoutAccepted).toBe(0);
  checkoutFailure = null;
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  expect(checkoutAccepted).toBe(1); expect(new Set(checkoutKeys).size).toBe(1); expect(new Set(checkoutParameters).size).toBe(1);
  expect(await bindings.DB.prepare("SELECT checkout_key,checkout_expires_at FROM relay_billing WHERE account_id=?").bind(ownerId).first()).toEqual({ checkout_key: pending!.checkout_key, checkout_expires_at: pending!.checkout_expires_at });
}, 10000);
it("preserves an earlier uncertain attempt through validation and bounded deadline reconciliation", async () => {
  const token = await account(); mockProvider(); checkoutFailure = "network";
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(503);
  const pending = await bindings.DB.prepare("SELECT checkout_key,checkout_expires_at FROM relay_billing WHERE account_id=?").bind(ownerId).first<{ checkout_key: string; checkout_expires_at: number }>();
  checkoutFailure = { status: 400, type: "invalid_request_error", param: "expires_at" };
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(503);
  expect(await bindings.DB.prepare("SELECT checkout_key,checkout_expires_at FROM relay_billing WHERE account_id=?").bind(ownerId).first()).toEqual(pending);
  const posts = checkoutCreates;
  const clock = vi.spyOn(Date, "now").mockReturnValue(pending!.checkout_expires_at - 31 * 60_000 + 1);
  checkoutFailure = null; checkoutListVisible = false;
  try {
    const unavailable = await call("/v1/billing/checkout", token, { plan: "monthly" });
    expect(unavailable.status).toBe(503); expect(await unavailable.json()).toEqual({ error: { code: "checkout_reconciliation_pending" } });
    checkoutListHasMore = true;
    const incomplete = await call("/v1/billing/checkout", token, { plan: "monthly" });
    expect(incomplete.status).toBe(503); expect(await incomplete.json()).toEqual({ error: { code: "checkout_review_required" } });
    checkoutListHasMore = false; clock.mockReturnValue(pending!.checkout_expires_at + 1000);
    expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(503);
    expect(checkoutLists).toBe(3); expect(checkoutCreates).toBe(posts); expect(checkoutAccepted).toBe(0);
    expect(await bindings.DB.prepare("SELECT checkout_key,checkout_expires_at FROM relay_billing WHERE account_id=?").bind(ownerId).first()).toEqual(pending);
  } finally { clock.mockRestore(); }
}, 10000);
it("checks completed Checkout after local expiry even while the completion webhook is delayed", async () => {
  const token = await account(); mockProvider();
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  checkoutStatus = "complete";
  await bindings.DB.prepare("UPDATE relay_billing SET checkout_expires_at=? WHERE account_id=?").bind(Date.now() - 1000, ownerId).run();
  const response = await call("/v1/billing/checkout", token, { plan: "monthly" });
  expect(response.status).toBe(409); expect(await response.json()).toEqual({ error: { code: "manage_existing_subscription_in_portal" } });
  expect(await plan()).toMatchObject({ plan: "free", paid_until: 0 }); expect(checkoutCreates).toBe(1);
  expect((await call("/v1/billing/portal", token, {})).status).toBe(200);
});
it("uses current nonterminal subscriptions instead of free local entitlement to prevent duplicate subscriptions", async () => {
  const token = await account(); mockProvider();
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  providerExists = true;
  for (const status of ["active", "past_due", "unpaid", "paused", "trialing", "incomplete", "unknown_future_status"]) {
    providerState = status;
    const response = await call("/v1/billing/checkout", token, { plan: "yearly" });
    expect(response.status).toBe(409); expect(await response.json()).toEqual({ error: { code: "manage_existing_subscription_in_portal" } });
  }
  expect(await plan()).toMatchObject({ plan: "free", paid_until: 0 }); expect(checkoutCreates).toBe(1);
  expect((await call("/v1/billing/portal", token, {})).status).toBe(200);
});
it("replaces only a provider-confirmed expired Checkout and refuses an incomplete subscription listing", async () => {
  const token = await account(); mockProvider();
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  subscriptionsHaveMore = true;
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(503); expect(checkoutCreates).toBe(1);
  subscriptionsHaveMore = false; checkoutStatus = "expired"; providerExists = true;
  for (const status of ["canceled", "incomplete_expired"]) {
    providerState = status;
    expect((await call("/v1/billing/checkout", token, { plan: "yearly" })).status).toBe(200);
  }
  expect(checkoutCreates).toBe(3); expect(new Set(checkoutKeys).size).toBe(3);
});
it("allows resubscription only after the completed Checkout's exact subscription is confirmed terminal", async () => {
  const token = await account(); mockProvider();
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  checkoutStatus = "complete"; providerExists = true; providerState = "canceled";
  expect((await call("/v1/billing/checkout", token, { plan: "yearly" })).status).toBe(200);
  expect(checkoutCreates).toBe(2); expect(checkoutKeys[1]).not.toBe(checkoutKeys[0]);
  checkoutSubscriptionId = "sub_unresolved_fixture";
  expect((await call("/v1/billing/checkout", token, { plan: "yearly" })).status).toBe(409);
  checkoutSubscriptionId = null;
  expect((await call("/v1/billing/checkout", token, { plan: "yearly" })).status).toBe(409);
  expect(checkoutCreates).toBe(2);
});
it("retains an expired uncertain request until its provider session can be recovered", async () => {
  const token = await account(); mockProvider();
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  await bindings.DB.prepare("UPDATE relay_billing SET checkout_session_id=NULL,checkout_expires_at=? WHERE account_id=?").bind(Date.now() - 1000, ownerId).run();
  checkoutListVisible = false;
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(503);
  checkoutListVisible = true; checkoutStatus = "complete";
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(409);
  expect(checkoutCreates).toBe(1);
  expect(await bindings.DB.prepare("SELECT checkout_key,checkout_session_id FROM relay_billing WHERE account_id=?").bind(ownerId).first()).toEqual({ checkout_key: checkoutKeys[0], checkout_session_id: "cs_test_fixture" });
});
it("expires paid authority absolutely while leaving free eligibility and commercial refusal unchanged", async () => {
  const token = await account(); mockProvider(); await call("/v1/billing/checkout", token, { plan: "monthly" }); await webhook("evt_paid_authority");
  const registration = await call("/v1/devices", token, { label: "Fixture paid device" }); const device = await registration.json<{ id: string }>();
  const session = await bindings.DB.prepare('SELECT id FROM "session" WHERE token=?').bind(token).first<{ id: string }>();
  expect((await authority(bindings, ownerId, device.id, session!.id))?.plan).toBe("paid");
  const clock = vi.spyOn(Date, "now").mockReturnValue(providerUntil * 1000);
  try { expect((await authority(bindings, ownerId, device.id, session!.id))?.plan).toBe("free"); }
  finally { clock.mockRestore(); }
  expect(await bindings.DB.prepare("SELECT status FROM relay_accounts WHERE id=?").bind(ownerId).first()).toEqual({ status: "active" });
});
it("revokes an account before acknowledging closure and retries failed subscription cancellation", async () => {
  const token = await account(); mockProvider(); await call("/v1/billing/checkout", token, { plan: "monthly" }); await webhook("evt_before_close");
  providerFailure = true;
  const response = await call("/v1/account/close", token, { confirm: "close" });
  expect(response.status).toBe(200); expect(await response.json()).toMatchObject({ closed: true, billingCancellation: { state: "pending" } });
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(401);
  providerFailure = false; await operationalCleanup(bindings);
  providerState = "canceled"; expect((await webhook("evt_closed_cancellation")).status).toBe(200);
  expect(observedCancellation).toContain("cancel_at_period_end=true");
  expect(await bindings.DB.prepare("SELECT state FROM relay_billing_cancellations WHERE account_id=?").bind(ownerId).first()).toEqual({ state: "completed" });
}, 10000);

it("cancels a completed Checkout subscription on closure before its webhook reaches the relay", async () => {
  const token = await account(); mockProvider();
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  checkoutStatus = "complete"; providerExists = true;
  expect(await bindings.DB.prepare("SELECT subscription_id FROM relay_billing WHERE account_id=?").bind(ownerId).first()).toEqual({ subscription_id: null });
  const response = await call("/v1/account/close", token, { confirm: "close" });
  expect(response.status).toBe(200); expect(await response.json()).toMatchObject({ closed: true, billingCancellation: { state: "completed" } });
  expect(observedCancellation).toContain("cancel_at_period_end=true"); expect(checkoutCreates).toBe(1);
});
it("expires an open Checkout on closure so it cannot create a later subscription", async () => {
  const token = await account(); mockProvider();
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  const response = await call("/v1/account/close", token, { confirm: "close" });
  expect(response.status).toBe(200); expect(await response.json()).toMatchObject({ closed: true, billingCancellation: { state: "completed" } });
  expect(checkoutExpirations).toBe(1); expect(checkoutStatus).toBe("expired"); expect(checkoutCreates).toBe(1);
});
it("keeps uncertain checkout cancellation pending until provider history resolves it", async () => {
  const token = await account(); mockProvider();
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  await bindings.DB.prepare("UPDATE relay_billing SET checkout_session_id=NULL WHERE account_id=?").bind(ownerId).run(); checkoutListVisible = false;
  const response = await call("/v1/account/close", token, { confirm: "close" });
  expect(response.status).toBe(200); expect(await response.json()).toMatchObject({ closed: true, billingCancellation: { state: "pending" } });
  checkoutListVisible = true; checkoutStatus = "complete"; providerExists = true; providerState = "incomplete";
  await operationalCleanup(bindings);
  expect(subscriptionCancellations).toBe(1); expect(providerState).toBe("canceled");
  expect(await bindings.DB.prepare("SELECT state FROM relay_billing_cancellations WHERE account_id=?").bind(ownerId).first()).toEqual({ state: "completed" });
});

it("retries current provider state when Checkout completion races account-close expiration", async () => {
  const token = await account(); mockProvider();
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  checkoutCompletionRace = true;
  const response = await call("/v1/account/close", token, { confirm: "close" });
  expect(response.status).toBe(200); expect(await response.json()).toMatchObject({ closed: true, billingCancellation: { state: "pending" } });
  expect(checkoutStatus).toBe("complete"); expect(observedCancellation).toBe("");
  await operationalCleanup(bindings);
  expect(observedCancellation).toContain("cancel_at_period_end=true"); expect(checkoutCreates).toBe(1);
  expect(await bindings.DB.prepare("SELECT state FROM relay_billing_cancellations WHERE account_id=?").bind(ownerId).first()).toEqual({ state: "completed" });
});

it("persists cancellation recovery before provider I/O when closure raced the first billing row", async () => {
  const token = await account(); mockProvider();
  expect((await call("/v1/billing/checkout", token, { plan: "monthly" })).status).toBe(200);
  // The first Checkout has now published its billing row after close's prior empty snapshot.
  await bindings.DB.prepare("UPDATE relay_accounts SET status='closed' WHERE id=?").bind(ownerId).run();
  expect(await bindings.DB.prepare("SELECT state FROM relay_billing_cancellations WHERE account_id=?").bind(ownerId).first()).toBeNull();
  providerFailure = true;
  const cancellation = await bindings.ACCOUNT_QUOTA.get(bindings.ACCOUNT_QUOTA.idFromName(ownerId)).fetch("https://internal/billing/cancel", { method: "POST", body: JSON.stringify({ accountId: ownerId }) });
  await cancellation.text(); expect(cancellation.status).toBe(503);
  expect(await bindings.DB.prepare("SELECT state FROM relay_billing_cancellations WHERE account_id=?").bind(ownerId).first()).toEqual({ state: "pending" });
  providerFailure = false; await operationalCleanup(bindings);
  expect(checkoutStatus).toBe("expired");
  expect(await bindings.DB.prepare("SELECT state FROM relay_billing_cancellations WHERE account_id=?").bind(ownerId).first()).toEqual({ state: "completed" });
}, 10000);
