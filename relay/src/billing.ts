import Stripe from "stripe";
import type { AccountSession, Env } from "./types.js";
import { boundedBytes, failure, jsonObject, onlyKeys } from "./http.js";

function stripe(env: Env): Stripe { return new Stripe(env.STRIPE_SECRET_KEY ?? "", { httpClient: Stripe.createFetchHttpClient(), maxNetworkRetries: 1, timeout: 10000 }); }
export function billingConfigured(env: Env): boolean { return Boolean(env.STRIPE_SECRET_KEY && env.STRIPE_WEBHOOK_SECRET && env.STRIPE_MONTHLY_PRICE && env.STRIPE_YEARLY_PRICE); }
function object(value: unknown): Record<string, unknown> { return value !== null && typeof value === "object" ? value as Record<string, unknown> : {}; }
function id(value: unknown): string | null { return typeof value === "string" ? value : typeof object(value).id === "string" ? object(value).id as string : null; }
export function subscriptionEntitlement(subscription: unknown, allowedPrices: string[], now: number): { active: boolean; until: number; status: string; cancelAtPeriodEnd: boolean; invoiceId: string | null } {
  const sub = object(subscription);
  const items = object(sub.items).data;
  const first = Array.isArray(items) && items.length === 1 ? object(items[0]) : {};
  const price = id(first.price);
  const until = Number(first.current_period_end ?? sub.current_period_end ?? 0) * 1000;
  const status = typeof sub.status === "string" ? sub.status : "unknown";
  return { active: price !== null && allowedPrices.includes(price) && status === "active" && until > now, until, status, cancelAtPeriodEnd: sub.cancel_at_period_end === true, invoiceId: id(sub.latest_invoice) };
}
function checkoutResponse(session: Stripe.Checkout.Session, accountId: string, customerId: string, planMatches: boolean, completedSubscriptionEnded = false): Response | null {
  if (id(session.customer) !== customerId || session.mode !== "subscription" || session.metadata?.relay_account_id !== accountId) return failure(503, "checkout_review_required");
  if (session.status === "complete") return completedSubscriptionEnded ? null : failure(409, "manage_existing_subscription_in_portal");
  if (session.status === "expired") return null;
  if (!planMatches) return failure(409, "finish_or_expire_pending_checkout");
  if (session.status === "open" && session.url && new URL(session.url).origin === "https://checkout.stripe.com") return Response.json({ url: session.url });
  return failure(503, "checkout_unavailable");
}
async function chargeInvoice(api: Stripe, charge: Stripe.Charge, currentInvoice: string | null): Promise<string | null> {
  const intent = id(charge.payment_intent);
  if (intent) {
    const payments = await api.invoicePayments.list({ payment: { type: "payment_intent", payment_intent: intent }, limit: 20 });
    const current = payments.data.find(payment => id(payment.invoice) === currentInvoice);
    return id(current?.invoice ?? payments.data[0]?.invoice);
  }
  if (currentInvoice) {
    const payments = await api.invoicePayments.list({ invoice: currentInvoice, limit: 100 });
    if (payments.data.some(payment => id(payment.payment.charge) === charge.id)) return currentInvoice;
  }
  // Unknown mapping is retained as a conservative hold until reconciled.
  return null;
}
export async function billingRoute(request: Request, env: Env, account: AccountSession | null): Promise<Response | null> {
  const path = new URL(request.url).pathname;
  if (path === "/v1/billing/webhook" && request.method === "POST") {
    if (!billingConfigured(env)) return failure(503, "billing_not_configured");
    const body = await boundedBytes(request, 256 * 1024);
    const signature = request.headers.get("stripe-signature");
    if (body === null || !signature || signature.length > 2048) return failure(400, "invalid_webhook");
    let event: Stripe.Event;
    try { event = await stripe(env).webhooks.constructEventAsync(body, signature, env.STRIPE_WEBHOOK_SECRET ?? "", undefined, Stripe.createSubtleCryptoProvider()); }
    catch { return failure(400, "invalid_webhook_signature"); }
    const watched = /^(customer\.subscription\.|checkout\.session\.completed|invoice\.payment_|charge\.refunded|charge\.dispute\.)/.test(event.type);
    if (!watched) return Response.json({ received: true });
    const data = object(event.data.object);
    let customerId = id(data.customer);
    if (!customerId && event.type.startsWith("charge.dispute.")) {
      const chargeId = id(data.charge);
      if (chargeId) customerId = id((await stripe(env).charges.retrieve(chargeId)).customer);
    }
    if (!customerId) return Response.json({ received: true });
    const row = await env.DB.prepare("SELECT account_id FROM relay_billing WHERE customer_id=?").bind(customerId).first<{ account_id: string }>();
    if (!row) return Response.json({ received: true });
    await env.DB.prepare("INSERT INTO relay_billing_events(id,account_id,status,created_at,expires_at) VALUES(?,?,'pending',?,?) ON CONFLICT(id) DO NOTHING").bind(event.id, row.account_id, Date.now(), Date.now() + 90 * 86_400_000).run();
    const response = await env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(row.account_id)).fetch("https://internal/billing/event", { method: "POST", body: JSON.stringify({ accountId: row.account_id, eventId: event.id, eventType: event.type, objectId: id(data), chargeId: id(data.charge) }) });
    await response.text();
    return response.ok ? Response.json({ received: true }) : failure(503, "billing_reconciliation_pending");
  }
  if (!["/v1/billing/checkout", "/v1/billing/portal"].includes(path)) return null;
  if (request.method !== "POST") return failure(405, "method_not_allowed");
  if (!account || account.termsVersion !== env.TERMS_VERSION) return failure(403, "admitted_account_required");
  if (!billingConfigured(env)) return failure(503, "billing_not_configured");
  const body = await jsonObject(request);
  if (!body || !onlyKeys(body, path.endsWith("checkout") ? ["plan"] : []) || (path.endsWith("checkout") && !["monthly", "yearly"].includes(String(body.plan)))) return failure(400, "invalid_billing_request");
  return env.ACCOUNT_QUOTA.get(env.ACCOUNT_QUOTA.idFromName(account.accountId)).fetch(`https://internal/billing/${path.endsWith("checkout") ? "checkout" : "portal"}`, { method: "POST", body: JSON.stringify({ accountId: account.accountId, plan: body.plan }) });
}

// Called only by the same per-account serialized authority that allocates leases.
export async function serializedBilling(env: Env, path: string, data: Record<string, unknown>): Promise<Response> {
  if (!billingConfigured(env)) return failure(503, "billing_not_configured");
  const accountId = String(data.accountId);
  const account = await env.DB.prepare("SELECT status,terms_version FROM relay_accounts WHERE id=?").bind(accountId).first<{ status: string; terms_version: string }>();
  if (!account || (!["/billing/cancel", "/billing/event"].includes(path) && (account.status !== "active" || account.terms_version !== env.TERMS_VERSION))) return failure(403, "admitted_account_required");
  const api = stripe(env);
  let billing = await env.DB.prepare("SELECT customer_id,subscription_id,plan,status,paid_until,checkout_key,checkout_session_id,checkout_plan,checkout_expires_at FROM relay_billing WHERE account_id=?").bind(accountId).first<{ customer_id: string; subscription_id: string | null; plan: string; status: string; paid_until: number; checkout_key: string | null; checkout_session_id: string | null; checkout_plan: string | null; checkout_expires_at: number | null }>();
  if (path === "/billing/cancel") {
    if (billing) {
      // A first Checkout can finish after close's earlier D1 snapshot found no billing row.
      // Persist recovery inside this authority before any fallible provider I/O.
      await env.DB.prepare("INSERT INTO relay_billing_cancellations(account_id,state,created_at,updated_at) VALUES(?,'pending',?,?) ON CONFLICT(account_id) DO UPDATE SET state='pending',updated_at=excluded.updated_at").bind(accountId, Date.now(), Date.now()).run();
      // Closing the account also closes payment paths whose webhook has not arrived.
      const listed = await api.checkout.sessions.list({ customer: billing.customer_id, limit: 20 });
      if (listed.has_more) return failure(503, "checkout_review_required");
      const sessions = listed.data.filter(session => session.mode === "subscription");
      const checkoutSessionId = billing.checkout_session_id;
      const checkoutKey = billing.checkout_key;
      if (checkoutSessionId && !sessions.some(session => session.id === checkoutSessionId)) sessions.push(await api.checkout.sessions.retrieve(checkoutSessionId));
      const unresolved = checkoutKey && !sessions.some(session => session.id === checkoutSessionId || session.metadata?.relay_checkout_key === checkoutKey);
      for (const session of sessions) {
        if (id(session.customer) !== billing.customer_id || session.mode !== "subscription") return failure(503, "checkout_review_required");
        if (session.status === "open") {
          const expired = await api.checkout.sessions.expire(session.id, {}, { idempotencyKey: `relay-close-checkout-${session.id}` });
          if (expired.status !== "expired") return failure(503, "checkout_reconciliation_pending");
        } else if (!["complete", "expired"].includes(String(session.status))) return failure(503, "checkout_reconciliation_pending");
      }
      // List after expiring open sessions: a completion race leaves this task pending.
      const subscriptions = await api.subscriptions.list({ customer: billing.customer_id, status: "all", limit: 20 });
      if (subscriptions.has_more) return failure(503, "subscription_review_required");
      for (const subscription of subscriptions.data) {
        if (id(subscription.customer) !== billing.customer_id) return failure(503, "subscription_review_required");
        if (["canceled", "incomplete_expired"].includes(subscription.status) || subscription.cancel_at_period_end) continue;
        const canceled = subscription.status === "incomplete"
          ? await api.subscriptions.cancel(subscription.id, {}, { idempotencyKey: `relay-close-subscription-${subscription.id}` })
          : await api.subscriptions.update(subscription.id, { cancel_at_period_end: true }, { idempotencyKey: `relay-close-subscription-${subscription.id}` });
        if (!["canceled", "incomplete_expired"].includes(canceled.status) && !canceled.cancel_at_period_end) return failure(503, "billing_cancellation_pending");
      }
      if (unresolved || sessions.some(session => session.status === "complete" && !subscriptions.data.some(subscription => subscription.id === id(session.subscription)))) return failure(503, "billing_reconciliation_pending");
    }
    await env.DB.prepare("UPDATE relay_billing_cancellations SET state='completed',updated_at=? WHERE account_id=?").bind(Date.now(), accountId).run();
    return Response.json({ cancellation: "completed" });
  }
  if (path === "/billing/checkout") {
    if (!billing) {
      const customer = await api.customers.create({ metadata: { relay_account_id: accountId } }, { idempotencyKey: `relay-customer-${accountId}` });
      await env.DB.prepare("INSERT INTO relay_billing(account_id,customer_id,plan,status,paid_until,cancel_at_period_end,updated_at) VALUES(?,?,'free','free',0,0,?) ON CONFLICT(account_id) DO NOTHING").bind(accountId, customer.id, Date.now()).run();
      billing = { customer_id: customer.id, subscription_id: null, plan: "free", status: "free", paid_until: 0, checkout_key: null, checkout_session_id: null, checkout_plan: null, checkout_expires_at: null };
    }
    const price = data.plan === "monthly" ? env.STRIPE_MONTHLY_PRICE : data.plan === "yearly" ? env.STRIPE_YEARLY_PRICE : undefined;
    if (!price) return failure(400, "invalid_plan");
    // Local entitlement can lag a completed Checkout or suspended subscription.
    // Any nonterminal subscription for this customer must be managed in the portal.
    const subscriptions = await api.subscriptions.list({ customer: billing.customer_id, status: "all", limit: 20 });
    if (subscriptions.has_more) return failure(503, "subscription_review_required");
    if (subscriptions.data.some(subscription => !["canceled", "incomplete_expired"].includes(subscription.status))) return failure(409, "manage_existing_subscription_in_portal");
    const now = Date.now();
    let pending: Stripe.Checkout.Session | null = null;
    if (billing.checkout_session_id) pending = await api.checkout.sessions.retrieve(billing.checkout_session_id);
    else if (billing.checkout_key && (billing.checkout_expires_at ?? 0) <= now) {
      // Recover a lost response without replacing an uncertain provider request.
      const sessions = await api.checkout.sessions.list({ customer: billing.customer_id, limit: 20 });
      const requestKey = billing.checkout_key;
      pending = sessions.data.find(session => session.metadata?.relay_checkout_key === requestKey) ?? null;
      if (!pending) return failure(503, "checkout_reconciliation_pending");
      await env.DB.prepare("UPDATE relay_billing SET checkout_session_id=? WHERE account_id=? AND checkout_key=?").bind(pending.id, accountId, billing.checkout_key).run();
    }
    if (pending) {
      const completedSubscriptionEnded = pending.status === "complete" && subscriptions.data.some(subscription => subscription.id === id(pending.subscription) && ["canceled", "incomplete_expired"].includes(subscription.status));
      const response = checkoutResponse(pending, accountId, billing.customer_id, billing.checkout_plan === data.plan, completedSubscriptionEnded);
      if (response) return response;
      // Replace only after provider-confirmed expiry or a terminal prior subscription.
      billing.checkout_key = null;
    } else if (billing.checkout_key && billing.checkout_plan !== data.plan) return failure(409, "finish_or_expire_pending_checkout");
    if (!billing.checkout_key) {
      billing.checkout_key = crypto.randomUUID(); billing.checkout_expires_at = (Math.floor(now / 1000) + 1800) * 1000;
      await env.DB.prepare("UPDATE relay_billing SET checkout_key=?,checkout_session_id=NULL,checkout_plan=?,checkout_expires_at=? WHERE account_id=?").bind(billing.checkout_key, data.plan, billing.checkout_expires_at, accountId).run();
    }
    // Persist request identity before provider I/O, so an uncertain retry recovers one session.
    const checkout = await api.checkout.sessions.create({ mode: "subscription", customer: billing.customer_id, line_items: [{ price, quantity: 1 }], expires_at: Math.floor((billing.checkout_expires_at ?? 0) / 1000), success_url: `${env.RELAY_ORIGIN}/account?checkout=returned`, cancel_url: `${env.RELAY_ORIGIN}/account`, subscription_data: { metadata: { relay_account_id: accountId } }, metadata: { relay_account_id: accountId, relay_checkout_key: billing.checkout_key } }, { idempotencyKey: billing.checkout_key });
    await env.DB.prepare("UPDATE relay_billing SET checkout_session_id=? WHERE account_id=? AND checkout_key=?").bind(checkout.id, accountId, billing.checkout_key).run();
    return checkoutResponse(checkout, accountId, billing.customer_id, true) ?? failure(503, "checkout_unavailable");
  }
  if (path === "/billing/portal") {
    if (!billing) return failure(404, "billing_account_not_found");
    const portal = await api.billingPortal.sessions.create({ customer: billing.customer_id, return_url: `${env.RELAY_ORIGIN}/account` });
    if (new URL(portal.url).origin !== "https://billing.stripe.com") return failure(503, "portal_unavailable");
    return Response.json({ url: portal.url });
  }
  if (path !== "/billing/event" || !billing) return failure(404, "billing_account_not_found");
  const event = await env.DB.prepare("SELECT status FROM relay_billing_events WHERE id=? AND account_id=?").bind(data.eventId, accountId).first<{ status: string }>();
  if (!event) return failure(400, "unverified_event");
  if (event.status === "processed") return Response.json({ processed: true });
  // Fetch current state on every retry. Delivery order and redirect parameters confer no authority.
  const subscriptions = await api.subscriptions.list({ customer: billing.customer_id, status: "all", limit: 20 });
  if (subscriptions.has_more) return failure(503, "subscription_review_required");
  if (account.status === "closed") await env.DB.prepare("INSERT INTO relay_billing_cancellations(account_id,state,created_at,updated_at) VALUES(?,'pending',?,?) ON CONFLICT(account_id) DO UPDATE SET state='pending',updated_at=excluded.updated_at").bind(accountId, Date.now(), Date.now()).run();
  const allowedPrices = [env.STRIPE_MONTHLY_PRICE ?? "", env.STRIPE_YEARLY_PRICE ?? ""];
  const candidates = subscriptions.data.filter(sub => sub.metadata.relay_account_id === accountId).sort((a, b) => b.created - a.created);
  const subscription = candidates[0];
  const state = subscriptionEntitlement(subscription, allowedPrices, Date.now());
  if (typeof data.eventType === "string" && data.eventType === "charge.refunded" && typeof data.objectId === "string") {
    const charge = await api.charges.retrieve(data.objectId);
    if (id(charge.customer) !== billing.customer_id) return failure(400, "customer_mismatch");
    const invoiceId = await chargeInvoice(api, charge, state.invoiceId);
    await env.DB.prepare("INSERT INTO relay_payment_holds(account_id,reference_id,invoice_id,kind,active,updated_at) VALUES(?,?,?,'refund',?,?) ON CONFLICT(reference_id) DO UPDATE SET active=excluded.active,updated_at=excluded.updated_at").bind(accountId, charge.id, invoiceId, charge.refunded ? 1 : 0, Date.now()).run();
  }
  if (typeof data.eventType === "string" && data.eventType.startsWith("charge.dispute.") && typeof data.objectId === "string") {
    const dispute = await api.disputes.retrieve(data.objectId);
    const charge = await api.charges.retrieve(id(dispute.charge) ?? "");
    if (id(charge.customer) !== billing.customer_id) return failure(400, "customer_mismatch");
    const active = !["won", "warning_closed"].includes(dispute.status);
    await env.DB.prepare("INSERT INTO relay_payment_holds(account_id,reference_id,invoice_id,kind,active,updated_at) VALUES(?,?,?,'dispute',?,?) ON CONFLICT(reference_id) DO UPDATE SET active=excluded.active,updated_at=excluded.updated_at").bind(accountId, dispute.id, await chargeInvoice(api, charge, state.invoiceId), active ? 1 : 0, Date.now()).run();
  }
  const holds = await env.DB.prepare("SELECT COUNT(*) AS n FROM relay_payment_holds WHERE account_id=? AND active=1 AND (kind='dispute' OR invoice_id=? OR invoice_id IS NULL)").bind(accountId, state.invoiceId).first<{ n: number }>();
  const paid = state.active && (holds?.n ?? 1) === 0;
  await env.DB.batch([
    env.DB.prepare("UPDATE relay_billing SET subscription_id=?,plan=?,status=?,paid_until=?,cancel_at_period_end=?,updated_at=? WHERE account_id=?").bind(subscription?.id ?? null, paid ? "paid" : "free", paid ? "active" : state.active ? "payment_hold" : state.status, paid ? state.until : 0, state.cancelAtPeriodEnd ? 1 : 0, Date.now(), accountId),
    env.DB.prepare("UPDATE relay_billing_events SET status='processed',processed_at=? WHERE id=? AND account_id=?").bind(Date.now(), data.eventId, accountId),
  ]);
  if (account.status === "closed") {
    const cancellation = await serializedBilling(env, "/billing/cancel", { accountId });
    await cancellation.text();
    if (!cancellation.ok) return failure(503, "billing_cancellation_pending");
  }
  return Response.json({ processed: true });
}
