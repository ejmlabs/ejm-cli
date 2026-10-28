// Build-only entry bundled by Vite SSR. Never import this TSX directly in Node.
import { renderToStaticMarkup } from "react-dom/server";
import { Button, buttonVariants } from "@ejm/ui/components/ui/button";
import { Input, fieldClass } from "@ejm/ui/components/ui/input";
import { Textarea } from "@ejm/ui/components/ui/textarea";
import { Select } from "@ejm/ui/components/ui/select";
import { Badge } from "@ejm/ui/components/ui/badge";
import { cn } from "@ejm/ui/lib/utils";

const checkboxClass = "size-5 shrink-0 accent-primary focus-ring forced-colors:appearance-auto";
export const html = "<!doctype html>" + renderToStaticMarkup(
  <html lang="en">
    <head>
      <meta charSet="utf-8" />
      <meta name="viewport" content="width=device-width, initial-scale=1" />
      <meta name="referrer" content="no-referrer" />
      <meta httpEquiv="Content-Security-Policy" content="default-src 'self'; script-src 'self'; style-src 'self'; font-src 'self'; img-src 'self'; connect-src 'none'; form-action 'none'; base-uri 'none'; object-src 'none'" />
      <title>Shared relay controls build proof | cli by ejmlabs</title>
      <script src="/common/theme-boot.js"></script>
      <link rel="stylesheet" href="/common/relay.css" />
      <link rel="preload" href="/common/manrope-latin.woff2" as="font" type="font/woff2" crossOrigin="anonymous" />
      <link rel="preload" href="/common/ibm-plex-mono-500-latin.woff2" as="font" type="font/woff2" crossOrigin="anonymous" />
    </head>
    <body className="bg-ground text-fg font-sans">
      <a href="#main" className="sr-only focus:not-sr-only focus-ring">Skip to controls</a>
      <header className="page flex items-center gap-3 py-5 hairline">
        <img src="/common/cli-mark-ink.svg" width="32" height="32" alt="" className="dark:hidden" />
        <img src="/common/cli-mark-paper.svg" width="32" height="32" alt="" className="hidden dark:block" />
        <span className="font-semibold">cli by ejmlabs</span>
      </header>
      <main id="main" className="page py-8">
        <div className="max-w-measure space-y-8">
          <div className="space-y-3">
            <h1 className="text-2xl font-semibold tracking-tight">Shared relay controls</h1>
            <Badge tone="warn">Build proof. No relay connection.</Badge>
            <p>This page checks the shared controls and assets before relay pages adopt them.</p>
          </div>
          <form className="space-y-5" autoComplete="off">
            <div className="space-y-2">
              <label htmlFor="proof-invitation" className="block font-semibold">Private invitation</label>
              <Input id="proof-invitation" type="password" autoComplete="off" autoCapitalize="off" spellCheck={false} maxLength={2048} aria-describedby="proof-trust" />
              <p id="proof-trust" className="text-sm text-muted">The relay operator serves the browser code and could change it to read your session. Use the native controller for stronger protection from the operator.</p>
            </div>
            <label className="flex items-start gap-3">
              <input type="checkbox" className={checkboxClass} required />
              <span>I understand the browser trust limit.</span>
            </label>
            <div className="space-y-2">
              <label htmlFor="proof-action" className="block font-semibold">Action</label>
              <Select id="proof-action" defaultValue="submit">
                <option value="submit">Submit a new prompt</option>
                <option value="steer">Steer the active run</option>
                <option value="queue">Queue after the active run</option>
              </Select>
            </div>
            <div className="space-y-2">
              <label htmlFor="proof-prompt" className="block font-semibold">Prompt or instruction</label>
              <Textarea id="proof-prompt" rows={3} maxLength={16384} aria-describedby="proof-limit" />
              <p id="proof-limit" className="text-sm text-muted">The controller separately checks the 16 KiB UTF-8 limit before sending.</p>
            </div>
            <div className="flex flex-wrap gap-3">
              <Button type="button" disabled>Pair session</Button>
              <Button type="button" variant="secondary">Disconnect and clear</Button>
              <button type="button" className={cn(buttonVariants({ variant: "danger", density: "compact" }))}>Revoke device</button>
            </div>
          </form>
          <p className="font-mono text-sm break-all">Full fingerprint display: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef</p>
          <noscript>The native controls remain visible without JavaScript. This build proof does not connect to a host.</noscript>
        </div>
      </main>
    </body>
  </html>,
);

export const classes = {
  revokeButton: cn(buttonVariants({ variant: "danger", density: "compact" })),
  stopButton: cn(buttonVariants({ variant: "secondary", density: "compact" })),
  field: fieldClass,
  checkbox: checkboxClass,
};
