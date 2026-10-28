import { billingConfigured } from "./billing.js";
import { parseConfig } from "./privacy/config.js";
import { retentionPolicy } from "./retention.js";
import { templates, uiPage, pageValues, withCurrentNav } from "./ui.js";
import type { AccountSession, Env } from "./types.js";

export function page(title: string, body: string, current?: "/account" | "/control" | "/privacy"): Response {
  const response = uiPage(title, body);
  return current ? withCurrentNav(response, current) : response;
}
export function authFailurePage(): Response { return withCurrentNav(uiPage("Sign-in unavailable", "<p>Sign-in could not finish. Return to Account and try again.</p>", 500), "/account"); }
export function accountPage(account: AccountSession | null, env: Env): Response {
  if (!account) return page("Relay account", templates.signedOut, "/account");
  return pageValues(page("Relay account", templates.account, "/account"), { email: account.email, termsVersion: env.TERMS_VERSION, termsState: account.termsVersion === env.TERMS_VERSION ? "Service terms accepted" : "Service terms required" },
    { plan: billingConfigured(env) ? templates.paymentActions : templates.freePlan },
    { "#accept-terms": { "data-version": env.TERMS_VERSION }, "#close-account-form": { method: "post", action: "/v1/account/close" }, ...(account.termsVersion === env.TERMS_VERSION ? {} : { "#account-terms": { open: "" } }) });
}
export function devicePage(account: AccountSession | null): Response { return page("Approve a CLI device", account ? templates.device : templates.deviceSignedOut, "/account"); }
export function termsPage(env: Env): Response {
  const policy = retentionPolicy(env);
  const body = (policy ? templates.operatorNotice : templates.previewNotice) + templates.terms;
  return pageValues(page("Relay service terms", body), {
    termsVersion: env.TERMS_VERSION, policyVersion: policy?.version ?? "", approvedBy: policy?.approvedBy ?? "",
  }, {
    payments: billingConfigured(env) ? templates.paidTerms : templates.freePlan,
    commercial: parseConfig(env.COMMERCIAL_CONFIG) ? templates.commercialEnabled : templates.commercialDisabled,
  }, policy ? { '[data-field="noticeLink"]': { href: policy.externalNotice } } : {});
}
export const accountScript = String.raw`const status=document.getElementById('status');
const signInError=new URL(location.href).searchParams.get('error');
if(signInError){const messages=new Map([['signup_disabled','New relay accounts are not being accepted. Existing accounts can still sign in.'],['unable_to_get_user_info','GitHub identity could not be verified. Try signing in again.'],['access_denied','GitHub sign-in was not completed. You can try again.']]);status.textContent=messages.get(signInError)??'Sign-in could not finish. Start sign-in again.';const clean=new URL(location.href);clean.searchParams.delete('error');clean.searchParams.delete('error_description');history.replaceState(null,'',clean.pathname+clean.search+clean.hash);}
async function api(path,body,method='POST'){const response=await fetch(path,{method,credentials:'same-origin',headers:{'content-type':'application/json'},...(method==='GET'?{}:{body:JSON.stringify(body??{})})});const result=await response.json();if(!response.ok)throw new Error(result.error?.code??'Request failed');return result;}
function report(error){status.textContent=error instanceof Error?error.message:'Request failed';}
function on(id,task){document.getElementById(id)?.addEventListener('click',()=>task().catch(report));}
on('sign-in',async()=>{const result=await api('/api/auth/sign-in/social',{provider:'github',callbackURL:location.origin+'/account'});const target=new URL(result.url);if(target.origin!=='https://github.com')throw new Error('Unexpected sign-in origin');location.assign(target.href);});
on('accept-terms',async()=>{await api('/v1/account/terms',{version:document.getElementById('accept-terms').dataset.version});const label=document.querySelector('[data-field="termsState"]');if(label)label.textContent='Service terms accepted';status.textContent='Service terms accepted. Commercial choices remain off.';});
on('sign-out',async()=>{await api('/v1/logout',{});location.assign('/account');});
async function openBillingPortal(){const result=await api('/v1/billing/portal',{});const target=new URL(result.url);if(target.origin!=='https://billing.stripe.com')throw new Error('Unexpected payment origin');location.assign(target.href);}
on('billing-portal',openBillingPortal);
document.querySelectorAll('[data-plan]').forEach(button=>button.addEventListener('click',async()=>{try{const result=await api('/v1/billing/checkout',{plan:button.dataset.plan});const target=new URL(result.url);if(target.origin!=='https://checkout.stripe.com')throw new Error('Unexpected payment origin');location.assign(target.href);}catch(error){if(error instanceof Error&&error.message==='manage_existing_subscription_in_portal')await openBillingPortal().catch(report);else report(error);}}));
function row(label,action,task){const template=document.getElementById('account-row-template');if(!(template instanceof HTMLTemplateElement))throw new Error('Account page is incomplete');const item=template.content.firstElementChild.cloneNode(true);item.querySelector('[data-row-title]').textContent=label;const button=item.querySelector('[data-row-action]');if(action){button.textContent=action;button.addEventListener('click',()=>task().catch(report));}else button.parentElement.remove();return item;}
if(document.getElementById('devices'))api('/v1/devices',undefined,'GET').then(result=>{const list=document.getElementById('devices');list.replaceChildren();if(!result.devices.length)list.append(row('No registered devices.'));for(const device of result.devices)list.append(row(device.label+(device.revoked_at?' (revoked)':''),device.revoked_at?null:'Revoke',()=>api('/v1/devices/'+encodeURIComponent(device.id)+'/revoke',{}).then(()=>location.reload())));}).catch(report);
if(document.getElementById('rooms'))api('/v1/rooms',undefined,'GET').then(result=>{const list=document.getElementById('rooms');list.replaceChildren();if(!result.rooms.length)list.append(row('No active relay session.'));for(const room of result.rooms)list.append(row('Routing ID: '+room.id+' · expires '+new Date(room.expiresAt).toLocaleString(),'Stop sharing',()=>api('/v1/rooms/'+encodeURIComponent(room.id)+'/stop',{}).then(()=>location.reload())));}).catch(report);
let checkedCode=null;
document.getElementById('device-code')?.addEventListener('submit',event=>{event.preventDefault();const code=new FormData(event.target).get('code');api('/api/auth/device?user_code='+encodeURIComponent(code),undefined,'GET').then(result=>{if(result.client_id!=='ejm-cli-relay'||result.scope!=='relay')throw new Error('Unexpected or expired device request');checkedCode=code;document.getElementById('device-details').textContent='Client: cli by ejmlabs. Access: relay. Code: '+code;document.getElementById('device-decision').hidden=false;}).catch(report);});
for(const action of ['approve','deny'])on(action+'-device',async()=>{if(!checkedCode)throw new Error('Check the device code first');await api('/api/auth/device/'+action,{userCode:checkedCode});checkedCode=null;document.getElementById('device-decision').hidden=true;status.textContent=action==='approve'?'Device approved. Return to the CLI.':'Device denied.';});`;
