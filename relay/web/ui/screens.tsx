// Build-only static markup. React and shared components never enter served JavaScript.
import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Button } from "@ejm/ui/components/ui/button";
import { Input } from "@ejm/ui/components/ui/input";
import { NativeCheckbox } from "@ejm/ui/components/NativeCheckbox";
import { Select } from "@ejm/ui/components/ui/select";
import { TerminalWorkspace, TerminalHeader, TerminalLink, TerminalOutput, TerminalPairing, TerminalLog, TerminalEntry, TerminalStatus, TerminalComposer, TerminalTextarea, TerminalSelect } from "@ejm/ui/components/TerminalWorkspace";
import { Page } from "@ejm/ui/layout/Band";
import { DocumentContent } from "@ejm/ui/layout/DocumentContent";
import { ActionRows, ActionRow } from "@ejm/ui/layout/ActionRows";
import { Facts, Fact } from "@ejm/ui/layout/Facts";
import { SectionHead } from "@ejm/ui/layout/SectionHead";
import { Text } from "@ejm/ui/layout/Text";
import { Header } from "@ejm/ui/shell/Header";
import { Footer } from "@ejm/ui/shell/Footer";
import { Disclosure } from "@ejm/ui/shell/Disclosure";
import { SkipLink } from "@ejm/ui/shell/SkipLink";

const render = (node: ReactNode) => renderToStaticMarkup(node);
const actions = "flex flex-wrap items-center gap-3";
function Field({ name, label, values }: { name: string; label: string; values: string[] }) {
  return <label className="block space-y-2"><span>{label}</span><Select name={name}><option value="">Leave blank</option>{values.map(value => <option key={value}>{value}</option>)}</Select></label>;
}
const logo = <span className="inline-flex p-1"><img src="/common/relay-logo-ink.svg" width="798" height="256" alt="" className="h-8 w-auto dark:hidden" /><img src="/common/relay-logo-paper.svg" width="798" height="256" alt="" className="hidden h-8 w-auto dark:block" /></span>;
const siteHeader = <Header logo={logo} homeHref="/account" homeLabel="Relay by ejmlabs, account" themeControl={null} nav={[{ href: "/account", label: "Account" }, { href: "/control", label: "Controller" }, { href: "/privacy", label: "Privacy" }]} />;
const siteFooter = <Footer brand={<Text as="p" variant="muted">The relay carries encrypted traffic. Your host keeps control of local execution.</Text>} columns={[{ heading: "Relay", links: [{ href: "/terms", label: "Service terms" }, { href: "/privacy/rights", label: "Track a rights request" }, { href: "/do-not-sell-or-share", label: "Do not sell or share" }] }]} />;
const head = render(<><meta charSet="utf-8" /><meta name="viewport" content="width=device-width, initial-scale=1" /><meta name="referrer" content="same-origin" /><meta name="color-scheme" content="light dark" /><script src="/common/theme-boot.js"></script><link rel="icon" type="image/svg+xml" href="/common/relay-favicon.svg" /><link rel="stylesheet" href="/common/relay.css" /><link rel="preload" href="/common/manrope-latin.woff2" as="font" type="font/woff2" crossOrigin="anonymous" /><link rel="preload" href="/common/ibm-plex-mono-500-latin.woff2" as="font" type="font/woff2" crossOrigin="anonymous" /></>);

const signedOut = render(<>
  <p>Sign in to connect your own CLI devices or open the browser controller. Pairing grants access to one shared CLI session.</p>
  <Button id="sign-in">Sign in with GitHub</Button>
  <p className="text-sm text-muted">Sign-in uses your verified primary GitHub email for account operations. No data-sale choice is enabled by sign-in.</p>
  <noscript>Account sign-in needs JavaScript. Public opt-out and rights tracking forms work without it.</noscript>
</>);
const account = render(<>
  <div className="space-y-6">
    <ActionRows><ActionRow title={<><span className="text-muted">Signed in as </span><span data-field="email"></span></>} actions={<Button id="sign-out" variant="ghost" density="compact">Sign out</Button>} /></ActionRows>
    <div className={actions}><Button asChild density="compact"><a href="/control">Open controller</a></Button><Text as="p" variant="muted">Connect to a session shared from your CLI.</Text></div>
    <Disclosure id="account-terms" label={<span data-field="termsState">Service terms</span>}><p>Read <a href="/terms" data-field="termsVersion"></a> before requesting relay access.</p><Button id="accept-terms" data-version="" density="compact">Accept service terms</Button></Disclosure>
    <section><h2>Sessions</h2><ActionRows id="rooms"><ActionRow title="Loading relay sessions…" /></ActionRows></section>
    <section><h2>Devices</h2><ActionRows id="devices"><ActionRow title="Loading devices…" /></ActionRows><Text as="p" variant="muted">Revoking a device ends its relay access.</Text></section>
    <div>
      <Disclosure label="Plan and allowance"><div data-field="plan"></div><Text as="p" variant="muted">Data-sale refusal never changes your plan, price or allowance.</Text></Disclosure>
      <Disclosure label="Privacy and browser trust"><p>Session content and pairing keys stay at the endpoints. The relay sees account data, routing IDs, timing and traffic sizes.</p><p>The operator serves the browser code and could change it to read a pairing secret or session. An installed native controller provides a stronger boundary against a malicious operator.</p><nav aria-label="Privacy choices" className={actions}><a href="/privacy">Privacy choices</a><a href="/privacy/rights">Track a rights request</a><a href="/do-not-sell-or-share">Do not sell or share</a></nav></Disclosure>
      <Disclosure label="Close account"><p>This revokes relay access and suppresses new commercial transfers. Records legally needed for payments and rights may be retained.</p><form id="close-account-form" className="space-y-3"><NativeCheckbox name="confirm" value="close" required label="I want to close my relay account and revoke its devices." /><Button id="close-account" type="submit" variant="danger" density="compact">Close account</Button></form></Disclosure>
    </div>
  </div>
  <template id="account-row-template"><ActionRow title={<span data-row-title=""></span>} actions={<Button type="button" variant="secondary" density="compact" data-row-action="">Action</Button>} /></template>
  <noscript>Account and device actions need JavaScript. Privacy forms remain available without it.</noscript>
</>);
const paymentActions = render(<><p>Choose a configured paid plan or manage an existing subscription. Hosted payment fees are separate from hosting.</p><div className={actions}><Button data-plan="monthly">Monthly plan</Button><Button data-plan="yearly" variant="secondary">Yearly plan</Button><Button id="billing-portal" variant="secondary">Manage subscription</Button></div></>);
const freePlan = render(<p>Free beta. Paid plans are unavailable while payment configuration is disabled. Capacity and the published free allowances still apply.</p>);
const device = render(<>
  <p>Approve only a device in your possession. Do not approve unexpected requests or codes sent by someone else. Account approval grants relay access; it cannot grant access to an existing encrypted session.</p>
  <form id="device-code" className="space-y-5"><label className="block space-y-2"><span>Code shown by your CLI</span><Input name="code" autoComplete="off" maxLength={32} required /></label><Button type="submit">Check code</Button></form>
  <section id="device-decision" hidden><h2>Check this request</h2><p id="device-details" className="font-mono break-all"></p><p>Confirm that the code and requested relay access match your CLI.</p><div className={actions}><Button id="approve-device">Approve this device</Button><Button id="deny-device" variant="secondary">Deny</Button></div></section>
</>);
const deviceSignedOut = render(<><p>First sign in to your relay account. Then return here and enter the code displayed by the CLI.</p><Button id="sign-in">Sign in with GitHub</Button></>);
const terms = render(<>
  <p>Service terms version <span data-field="termsVersion" className="font-mono"></span>. This is a bounded beta with no availability guarantee. Local CLI operation can continue when the relay is unavailable.</p>
  <section><h2>Session privacy</h2><p>The service requires a verified relay account and registered devices. Only paired endpoints hold stream keys. Installed native controllers provide a stronger boundary against a malicious operator. The operator serves browser code and could change it to capture browser pairing secrets or plaintext.</p><p>The operator sees operational account and device data, routing identifiers, timing and traffic sizes. End-to-end encryption does not conceal these metadata.</p></section>
  <section><h2>Free beta limits</h2><p>At most one room per account, 50,000 combined incoming frames and 64 MiB daily, 16 KiB maximum frames, 100 frames per second and connection leases up to fifteen minutes. Capacity is shared and may be exhausted.</p></section>
  <section><h2>Payments</h2><div data-field="payments"></div></section>
  <section><h2>Optional choices and retention</h2><div data-field="commercial"></div><p>No relay payload is stored. Required operational records follow category retention policies; provider logs and backups follow their own disclosed policies.</p></section>
</>);
const operatorNotice = render(<><h2>Operator notice</h2><p><a data-field="noticeLink">Read the operator's terms and privacy notice</a>.</p><p>Operational policy version <span data-field="policyVersion" className="font-mono"></span>. Approval recorded by <span data-field="approvedBy"></span>.</p></>);
const previewNotice = render(<><h2>Preview notice</h2><p>The operator must publish its legal identity, contact, jurisdiction, retention and dispute terms before public admission. This preview is not an approved launch notice.</p></>);
const paidTerms = render(<p>Payment details stay with Stripe. Subscription cancellation keeps purchased access until the effective period ends. Failed payment, full refund of the current paid period, or a confirmed payment dispute suspends paid allocation. Free eligibility remains subject to admission. Hosted payment fees are separate from hosting.</p>);
const commercialEnabled = render(<p>Optional commercial choices are separate, off by default and never required for a plan. Read the configured privacy notice and named-recipient terms before opting in.</p>);
const commercialDisabled = render(<p>Commercial collection is disabled. No optional profile or usage sharing is enabled by signing in or accepting service terms. Opt-out, withdrawal and data rights remain available.</p>);
const adult = render(<form method="post" action="/api/privacy/adult"><NativeCheckbox name="adult" value="true" required label="I affirm that I am an adult eligible for this optional program." /><Button type="submit">Save eligibility</Button></form>);
const rejoin = render(<form method="post" action="/api/privacy/rejoin"><input type="hidden" name="step" value="request" /><Button type="submit" variant="secondary">Ask to rejoin</Button></form>);
const profile = render(<section><h2>Direct professional profile</h2><p>Enter your own choices. Do not copy provider data. The program accepts no names, email, company name or contact list.</p><form method="post" action="/api/privacy/profile"><input type="hidden" name="provenance" value="direct-user" /><Field name="role" label="Role" values={["engineering", "design", "product", "research", "operations", "education", "other"]} /><Field name="industry" label="Industry" values={["software", "finance", "health", "education", "manufacturing", "public-sector", "nonprofit", "other"]} /><Field name="organizationSize" label="Organization size" values={["solo", "2-10", "11-50", "51-250", "251-1000", "1001+"]} /><label className="block space-y-2"><span>Country code</span><Input name="country" maxLength={2} pattern="[A-Z]{2}" autoComplete="country" placeholder="Two uppercase letters" /></label><Button type="submit">Save or correct my profile</Button></form></section>);
const withdrawal = render(<section><h2>Withdraw a choice</h2><form method="post" action="/api/privacy/withdraw"><Field name="category" label="Category" values={["profile", "usage"]} /><Button type="submit" variant="secondary" className="max-w-full whitespace-normal text-wrap">Withdraw this category for all buyers</Button></form></section>);
const rights = render(<section><h2>Data rights</h2><p>Requests include operational stores and recorded recipients. We track deadlines, exceptions and recipient follow-up. We cannot promise that every buyer copy or provider backup is erased at once.</p><form method="post" action="/api/privacy/rights/request"><Field name="kind" label="Request kind" values={["access", "export", "correction", "delete", "close"]} /><label className="block space-y-2"><span>Jurisdiction</span><Input name="jurisdiction" required maxLength={64} placeholder="Your country or state" /></label><Button type="submit">Submit rights request</Button></form></section>);
const rightsStatus = render(<><p>Use your saved request ID and private status key. This works after closure and grants no relay access. Keep the key private. Leave the part blank for status, or enter an available export part number.</p><form method="post" action="/api/privacy/rights/status"><label className="block space-y-2"><span>Request ID</span><Input name="requestId" required autoComplete="off" /></label><label className="block space-y-2"><span>Private status key</span><Input name="statusKey" type="password" required autoComplete="off" minLength={64} maxLength={64} /></label><label className="block space-y-2"><span>Export part (optional)</span><Input name="part" type="number" min={0} step={1} /></label><Button type="submit">View this request</Button></form></>);
const optOut = render(<form method="post"><Button type="submit">Save my opt-out</Button></form>);
const consent = render(<><NativeCheckbox name="accepted" value="true" required label="I permit these fields for this named buyer and purpose." /><Button type="submit">Save this choice</Button></>);
const confirmRejoin = render(<><NativeCheckbox name="confirmed" value="true" required label="I confirm my request to rejoin after reviewing the current notices." /><Button type="submit">Confirm rejoining</Button></>);
const privacySignIn = render(<><p>Sign in to view your account's privacy data and choices. Public opt-out and private rights-request tracking remain available without a session.</p><Button asChild><a href="/account">Go to sign-in</a></Button></>);
const controllerSignIn = render(<><p>Sign in to your relay account before opening the controller. You will still need a private invitation and approval from your host.</p><Button asChild><a href="/account">Go to sign-in</a></Button></>);

// Split one shared, rendered document at fixed internal markers. Runtime values remain escaped.
const markers = ["EJM_RELAY_DOCUMENT_TITLE", "EJM_RELAY_PAGE_TITLE", "EJM_RELAY_CONTENTS", "EJM_RELAY_ACCOUNT_SCRIPT"];
const frame = ("<!doctype html>" + render(<html lang="en"><head><title>{markers[0] + " | Relay by ejmlabs"}</title></head><body className="bg-ground text-fg font-sans"><SkipLink />{siteHeader}<main id="main"><Page className="py-8"><DocumentContent><SectionHead as="h1" size="title" title={markers[1]} />{markers[2]}</DocumentContent></Page></main>{siteFooter}{markers[3]}</body></html>)).replace("</head>", head + "</head>");
let rest = frame;
const frameParts = markers.map(marker => {
  const parts = rest.split(marker);
  if (parts.length !== 2) throw new Error("Relay frame marker must occur once");
  rest = parts[1]!;
  return parts[0]!;
});
const [pageStart, pageHeading, pageContents, pageEnd] = frameParts as [string, string, string, string];
const pageClose = rest;
const accountStatus = render(<p id="status" role="status" aria-live="polite"></p>);
const accountScript = '<script src="/account.js" defer></script>';
const [privacyRecordStart, privacyRecordEnd] = render(<Disclosure label="View my privacy record"><pre>EJM_RELAY_PRIVACY_RECORD</pre></Disclosure>).split("EJM_RELAY_PRIVACY_RECORD") as [string, string];
export const templates = { pageStart, pageHeading, pageContents, pageEnd, pageClose, accountStatus, accountScript, signedOut, account, paymentActions, freePlan, device, deviceSignedOut, terms, operatorNotice, previewNotice, paidTerms, commercialEnabled, commercialDisabled, adult, rejoin, profile, withdrawal, rights, rightsStatus, optOut, consent, confirmRejoin, privacySignIn, controllerSignIn, privacyRecordStart, privacyRecordEnd };

const controller = <>
  <TerminalHeader homeHref="/account" homeLabel="Relay account" brand={<><span className="p-1.5"><img src="/common/relay-mark-ink.svg" width="24" height="24" alt="" className="dark:hidden" /><img src="/common/relay-mark-paper.svg" width="24" height="24" alt="" className="hidden dark:block" /></span><span className="font-sans">relay</span></>} title={<span id="session-heading">Shared session</span>}
    navigation={<nav aria-label="Session" className="flex items-center gap-2 sm:gap-4"><TerminalLink href="/account" hideOnSmall>Account</TerminalLink><Disclosure id="connection-details" label="Connection" presentation="menu">
      <section id="peer-panel" hidden className="space-y-4" aria-labelledby="peer-heading"><h2 id="peer-heading" className="font-semibold">Pairing details</h2><Facts><Fact label="Session"><span id="peer-session-id">Awaiting host approval</span></Fact><Fact label="Access"><span id="access">Waiting for host</span></Fact><Fact label="Host fingerprint"><span id="host-fingerprint"></span></Fact><Fact label="This tab's fingerprint"><span id="controller-fingerprint"></span></Fact><Fact label="Sharing ends"><span id="share-expiry"></span></Fact></Facts><Text as="p" variant="muted">Compare this tab's full fingerprint on the host and approve it there. A new tab or reload needs a new pairing.</Text><div className={actions}><Button id="reconnect" variant="secondary" density="compact" disabled>Reconnect</Button><Button id="disconnect" variant="ghost" density="compact">Disconnect and clear</Button></div></section>
      <Text as="p" variant="muted">Keep the host running while connected. This controller cannot change host grants or approve execution. The host's local policy stays in force.</Text>
    </Disclosure></nav>}
    context={<><span id="role-note">Waiting for a session</span><span aria-hidden="true"> · </span><span id="session-context">Pair a host to view its session.</span><span id="session-id" hidden></span></>}
    contextActions={<NativeCheckbox id="follow" defaultChecked density="compact" label="Follow output" />}
  />
  <TerminalOutput aria-label="Session output">
    <TerminalPairing id="pair-panel" aria-labelledby="pair-heading"><h2 id="pair-heading">Pair with your host</h2><p>Open a session shared from your CLI.</p><Text as="p" id="trust" variant="muted">This page encrypts traffic between your browser and the host. The relay operator serves this page and could change its code to read your session. Use the native controller for stronger protection from the operator.</Text>
      <form id="pair-form" autoComplete="off" className="space-y-4"><label htmlFor="invitation" className="block">Private invitation</label><Input id="invitation" type="password" autoComplete="off" autoCapitalize="off" spellCheck={false} maxLength={2048} aria-describedby="invite-help trust" required /><Text as="p" id="invite-help" variant="muted">Paste the invitation from the host's protected screen. It stays in this tab. Do not put it in a link.</Text><NativeCheckbox id="trust-accepted" required label="I understand the browser trust limit." /><Button id="pair" type="submit" density="compact" disabled>Pair session</Button></form>
    </TerminalPairing>
    <Text as="p" id="gap" variant="muted" className="hairline-t py-4" hidden>Some earlier output is unavailable. This view contains the host's retained events.</Text>
    <TerminalLog id="transcript" aria-label="Session output"><Text as="p" variant="muted">The host's prompts, responses, and tool results will appear here.</Text></TerminalLog>
  </TerminalOutput>
  <TerminalStatus connection={<span id="connection-state">Loading encryption</span>}><p id="status" role="status" aria-live="polite">Loading the local encryption engine.</p></TerminalStatus>
  <TerminalComposer aria-label="Send to host" legend="message" formProps={{ id: "action-form" }}
    prompt={<label htmlFor="prompt"><span aria-hidden="true">›</span><span className="sr-only">Prompt or instruction</span></label>}
    input={<TerminalTextarea id="prompt" rows={2} maxLength={16384} autoComplete="off" spellCheck={false} disabled placeholder="Describe a task…" />}
    actions={<><label><span className="sr-only">Action</span><TerminalSelect id="action" disabled><option value="submit">Submit a new prompt</option><option value="steer">Steer the active run</option><option value="queue">Queue after the active run</option></TerminalSelect></label><Button id="send" type="submit" density="compact" disabled>Send</Button><Button id="cancel" variant="ghost" density="compact" disabled>Cancel run</Button></>}
    hints={<><span>Enter adds a line</span><span aria-hidden="true">·</span><p id="action-status" role="status">View access keeps these actions disabled.</p></>}>
    <DocumentContent id="pending-panel" className="hairline-t py-4 text-sm" hidden><p id="pending-status" role="status"></p><Disclosure id="pending-draft" label="Original instruction"><pre id="pending-text"></pre></Disclosure><Button id="check-outcome" variant="secondary" density="compact" disabled>Check outcome</Button></DocumentContent>
  </TerminalComposer>
  <template id="transcript-entry-template"><TerminalEntry label="">{""}</TerminalEntry></template>
</>;
export const controllerHtml = ("<!doctype html>" + render(<html lang="en"><head><title>Remote session | Relay by ejmlabs</title></head><body className="bg-ground text-fg font-mono"><SkipLink /><TerminalWorkspace id="main">{controller}</TerminalWorkspace><noscript><Page className="py-6"><p>This controller needs JavaScript and WebAssembly. Use the native CLI controller if you prefer.</p></Page></noscript><script type="module" src="/control/assets/app.js"></script></body></html>)).replace("</head>", head + "</head>");
