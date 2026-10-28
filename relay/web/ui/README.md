# Shared relay screens

This source-only appearance layer consumes public `@ejm/ui` components and
`@ejm/tokens` styles, fonts and Relay artwork. It uses no private sibling imports.
React renders the shared components to static fragments during the build. Vite uses
automatic JSX and bundles public `@ejm/*` TSX in a temporary server entry, as
the shared SSG does. The routed, hydrating SSG renderer is not needed here.
The temporary React/Radix/server bundle is removed before output validation.

- `screens.tsx` composes public Header, Footer, Page, DocumentContent, Disclosure,
  NativeCheckbox, ActionRows and Terminal components with the existing shared
  buttons, inputs and selects. The account, device, terms, privacy and controller
  content remains product-owned. See the [public imports](../../../docs/relay-web.md#shared-appearance-preparation).
- `relay.css` imports the public base, static-page and terminal styles in that
  order, followed by the `screens.tsx` source directive. It defines no local
  visual primitives. NativeCheckbox uses `@ejm/ui/components/NativeCheckbox`.
- `build-screens.mjs` emits ignored `web/.ui/worker.ts`, controller HTML,
  common assets, complete font notices and a budget report.
- `src/ui.ts` composes the finite fragments. HTMLRewriter sets account and
  policy fields safely; privacy content retains explicit escaping. Account rows
  in `src/pages.ts` and transcript entries in `web/app.ts` clone build-rendered
  ActionRow and TerminalEntry templates, then fill text, state and handlers.
  The local preview clones the same templates with sample data. Existing vanilla
  scripts retain authentication, pairing, transport and run behavior; no React
  or other component framework runs in the Worker or browser.

The style entry is:

```css
@import "@ejm/ui/styles.css";
@import "@ejm/ui/styles-static.css";
@import "@ejm/ui/styles-terminal.css";
@source "./screens.tsx";
```

Static-page and terminal styles stay opt-in in the shared owner. Relay compiles
all three shared entries into its existing common stylesheet.

## Build and checks

Use Node 26.12.0 and npm 11.20.0. From `relay/`:

```sh
npm ci --ignore-scripts --install-links
npm run build:ui
npm run check:types
npm run check:schema
npm test
```

`--install-links` materializes the public local `file:` packages. Lifecycle
scripts remain disabled. The normal repository gate builds the fixture first;
`npm run check` also generates the shared Worker module before its checks.
`web/build.mjs` builds these screens before each production/fixture controller
build, including source-only frontend refreshes. Native/WASM behavior is unchanged.

`web/.ui/report.json` records raw/gzip bytes and hashes. The build rejects
unexpected files and inline scripts/styles. The 48 KiB raw total combines all
rendered fragments and controller HTML with the cooked literal parts in the
retained privacy adapter, document adapter and fixed authentication-error body.
Vite's public parser counts those adapter parts conservatively, including labels
and keys. Actual user/configuration data stays separate under existing bounds:
at most 32 named buyers, 64 consent rows and 65,536 JSON characters per stored
operational result. The report records both finite-template components. It also
rejects common CSS above 14 KiB gzip, the unchanged theme bootstrap
above 2 KiB gzip or fonts above 34,536 raw bytes. It emits no framework runtime
chunk. Measure the separate 32 KiB gzip incremental Worker budget from actual
before/after minified Wrangler bundles. Keep the existing 1.5 MiB WASM limit.
The earlier October 20 local build using the protected UI baseline recorded CSS at
14,103 bytes gzip within 14,336, theme bootstrap at 289 bytes gzip, templates at
41,657 raw bytes and fonts at 34,536 raw bytes, with no framework runtime chunks.
The current local base/static/terminal composition recorded CSS at 13,807 bytes
gzip within 14,336, theme bootstrap at 289 bytes gzip, finite templates at
48,345 raw bytes within 49,152 and fonts at 34,536 raw bytes. It emits zero
framework runtime chunks. Browser compilation, Worker type/schema checks,
118 Worker tests and eleven real Chrome cases passed in the
[current local stages](../../../docs/relay-publication.md#current-local-checks).
The comparable minified Worker presentation delta is 788 bytes gzip within
32,768. Account closure uses inline NativeCheckbox/Button confirmation;
unchecked requests are blocked and checked requests are covered in Chrome.
The earlier local light/dark viewport
gate found no horizontal overflow from 320px through 5120px for account and
controller pages; phone pairing screens were also checked. Native checkbox boxes
are smaller than their wrapping label targets. The browser review confirmed
16px pairing copy on phones, 14px desktop transcript text and 12px composer
insets. This offline sample disables actions. It does not qualify authentication,
WASM or native transport. Current local stages exercise those separately; normal
full and hosted gates remain required before adoption. Native source and the static UI
pin/context prerequisite are accepted through the protected merges recorded below.

The fixed public asset allowlist is common CSS, theme boot, two fonts and five
Relay SVGs: Ink/Paper outlined logos, Ink/Paper marks and the favicon. Font
notices ship beside the assets but are not another public
route. Controller HTML, app, protocol and WASM remain account protected.
Self-only CSP needs no inline exception. Theme boot loads before CSS, reads an
existing shared preference or system theme, and writes no product preference.
CSS follows the system theme without scripts. Privacy forms work without scripts.

## Relay identity

The build copies only the five selected files from the public
`@ejm/tokens/assets/projects/relay/` export. It never imports studio source,
editable logos or review pages. The exact output inventory and public-route
allowlist exclude those materials and keep controller assets authenticated.

Account, device and policy pages use the Ink/Paper outlined Relay logo. Its
798 × 256 view box keeps its proportions at 32 px high, with 12 px of clear
space around the image. The controller keeps its compact CLI layout and uses
a 24 px Relay mark with 6 px of clear space, beside the lowercase name. The
native host and transcript speaker remain `ejm-cli`. Links have accessible
Relay account names; decorative image alternatives stay empty. Every screen
uses the supplied Relay SVG favicon and a Relay document title.

The shared owner published the source baseline and Relay assets through protected
[ejm-ui PR #10](https://github.com/ejmlabs/ejm-ui/pull/10), merge
`24935aa85355ea99f49bdbecc4186a5e95049393`, reviewed tree
`2d8bde67c8b5fe851b70ae90a3a2787091397780`. Static component support is now
accepted through protected [ejm-ui PR #11](https://github.com/ejmlabs/ejm-ui/pull/11)
merge `79a1aa8b1075f834717bc0c0299dc2c41a22b966`, tree
`c18c74fd336dbad9515bff8ecf22ca980b889a51`. Protected CLI PR #22 merge
`78810826cd376588d3303f13bc237309fb5170a8` adopted that source pin and readable
source/install context. Native fixes were accepted through protected CLI PR #21
merge `71d0f47f8b05c6314eca4dc0750ffaad833d6afb`. This following Worker source
still needs fresh admission, hosted product CI, deployment and live asset
verification; see [publication prerequisites](../../../docs/relay-publication.md#protected-shared-source-and-inert-prerequisite).

## Proof history and release boundary

`build-proof.mjs`, `proof.tsx` and `proof.css` retain the initial small static
control proof. Root executed its V3 source successfully before authorizing the
screen composition. Its `web/.ui-proof/` output is not served. See
[UI build notices](../../UI_BUILD_NOTICES.md) for licenses.

The earlier composition selected the protected artwork/source baseline. The
current shared primitives are accepted in UI PR #11, and CLI PR #22 adopted
their protected source pin and readable source/install context. This prerequisite
does not establish the following Worker's runtime or installed-package acceptance.
The current local checks bind installed packages and runtime stages separately.
The release workflow owns normal gates, actual
bundle measurements, the design detector and the bounded desktop/mobile,
light/dark, keyboard, zoom, forced-colors and reduced-motion visual review.
Nothing here publishes screens, activates settings or completes a release task.

## Local visual review

The controller follows the native CLI: two compact header lines, a continuous
monospace transcript, one status line and a rounded composer. Pairing details
open while the host is approving this tab and remain available under Connection.
The composer appears after pairing starts, leaving the initial trust notice and
acknowledgement in view on phones.
The account page leads with the controller, sessions and devices. Current terms
open when acceptance is required; accepted terms and account settings use native
disclosures. Protocol, access roles and outcome recovery keep their existing
contracts.

Build an offline visual sample from the actual production templates and assets:

```sh
npm run build:ui
RELAY_PREVIEW_OUT=/absolute/new/preview-folder node web/ui/build-preview.mjs
```

The output directory must not exist. Serve only that directory on loopback using
an existing local static server. Open `/account/`, `/control/` and `/pairing/`.
`/account/?terms=required` shows the required-terms state. The preview has sample
data, disables actions, and loads no account script, controller script, WebSocket
or encryption engine. It is for local layout review and does not prove sign-in,
pairing or live execution. Its fixture script and output are outside the
production build and public asset allowlist. The optional `/logo/` link is for
a separately copied local artwork gallery. This candidate consumes the published
Relay artwork through the public shared asset package and accepted shared
components through public UI imports. Consumer pin/context and native source
adoption are accepted as recorded above. Fresh Worker admission, required full gates,
protected Worker adoption and application publication remain pending.
