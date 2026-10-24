# Adopting ejm-ui

How the ejmlabs.com site, the cli, ide and harness sites and the IDE app move onto ejm-ui. Use `examples/starter` as the reference consumer and check each source revision before adopting it. Sibling links and local tarballs do not require registry publication. Creating a remote, publishing a package and deploying each need the owner's approval.

## What you depend on

| Package | What it gives a consumer |
| --- | --- |
| `@ejm/tokens` | Palette, fonts, brand files (favicon set, logos, social card), generated Tailwind theme, Monaco and terminal themes. |
| `@ejm/ui` | Components, layouts, hooks, `styles.css` (the Tailwind v4 theme), `theme-boot.js`. |
| `@ejm/core` | Framework-neutral logic that `@ejm/ui` uses. Installed with it. |
| `@ejm/ssg` | Prerender plugin, `hydrateSite`, `Link`, the budget gate (`ejm-ssg`). Sites only. |
| `@ejm/site` | Project registry, analytics, shared head, theme boot and headers plugin (`ejm-site-headers`). Sites only. |
| `@ejm/demos` | The simulated product screens (static views, lazy interactive demos, the product hero). Only a site that shows a product demo. |

The packages ship TypeScript source for what the consumer's Vite bundles (components, hooks, the `@ejm/ssg` client and server render code). What Node itself loads, the `@ejm/ssg/vite` plugin, `ejm-ssg`, `@ejm/site/vite` and `ejm-site-headers`, is plain `.mjs` with `.d.mts` types, because Node refuses to strip types from TypeScript under `node_modules`. Use Node 22.12 or later.

## 1. Get the packages before a registry exists

Before building or using linked packages, run `npm ci --ignore-scripts` inside `ejm-ui` to install its committed lockfile. Linked packages resolve their own dependencies from there. Keep dependency lifecycle scripts disabled for this setup. Linux terminal fixture tests and capture need a separate, explicit `node-pty` rebuild; see [Development](README.md#development). The site build does not need that native rebuild.

### A. Link from a sibling checkout (works today)

Keep `ejm-ui` next to the product repo (`../ejm-ui`) and point at the package folders. In the consumer `package.json`:

```json
{
  "dependencies": {
    "@ejm/core": "file:../ejm-ui/packages/core",
    "@ejm/demos": "file:../ejm-ui/packages/demos",
    "@ejm/site": "file:../ejm-ui/packages/site",
    "@ejm/ssg": "file:../ejm-ui/packages/ssg",
    "@ejm/tokens": "file:../ejm-ui/packages/tokens",
    "@ejm/ui": "file:../ejm-ui/packages/ui",
    "react": "^19.3.1",
    "react-dom": "^19.3.1"
  },
  "devDependencies": {
    "@tailwindcss/vite": "^4.3.3",
    "@types/react": "^19.3.0",
    "@types/react-dom": "^19.3.0",
    "@vitejs/plugin-react": "^6.1.2",
    "tailwindcss": "^4.3.3",
    "typescript": "^5.9.0",
    "vite": "^8.3.4"
  }
}
```

`npm install` makes symlinks, so an edit in `ejm-ui` shows up at once, and the Node type-stripping limit does not apply (the real path is outside `node_modules`). Leave `@ejm/demos` out of the list if the site has no product demo. Two extra settings are required, because the linked packages would otherwise load a second copy of React (from `ejm-ui/node_modules`) at build time and a second copy of `@types/react` at type check time. Both were run in a throwaway consumer linked with `file:`:

```ts
// vite.config.ts: one React at build time, or hooks break
export default defineConfig({ resolve: { dedupe: ['react', 'react-dom'] }, plugins: [/* ... */] });
```

```jsonc
// tsconfig.json: one copy of the React types. Without it tsc fails inside @ejm/ui with
// "Two different types with this name exist, but they are unrelated" (Ref, VoidOrUndefinedOnly).
{
  "compilerOptions": {
    "target": "ES2022", "module": "ESNext", "moduleResolution": "bundler", "lib": ["ES2023", "DOM", "DOM.Iterable"],
    "jsx": "react-jsx", "strict": true, "skipLibCheck": true, "isolatedModules": true, "noEmit": true,
    "types": ["node", "vite/client"],
    "paths": {
      "react": ["./node_modules/@types/react"],
      "react/*": ["./node_modules/@types/react/*"],
      "react-dom": ["./node_modules/@types/react-dom"],
      "react-dom/*": ["./node_modules/@types/react-dom/*"]
    }
  },
  "include": ["src", "vite.config.ts"]
}
```

`paths` is relative to the tsconfig (no `baseUrl` needed). If the site lives in a subfolder of a repo that has the `node_modules`, point the four lines at that folder (`../node_modules/@types/react`). Route B (tarballs) needs neither line: the packages install their own dependencies next to yours.

Listing `@ejm/core` and `@ejm/tokens` yourself matters: `@ejm/ui` depends on them at exactly `0.1.0`, and with no registry npm can only satisfy that from your own entries.

### B. Tarballs with `npm pack`

From `ejm-ui`:

```sh
mkdir -p ../tarballs
for p in tokens core ui ssg site demos; do npm pack -w packages/$p --pack-destination ../tarballs; done
```

In the consumer, use `"@ejm/core": "file:../tarballs/ejm-core-0.1.0.tgz"` and the same for the others (leave out `demos` if the site has no product demo), with the same React and tooling dependencies as above (no `dedupe` needed). To update, repack, delete `node_modules/@ejm` and `package-lock.json`, and install again (npm will not refresh a tarball of the same version otherwise). Commit the lockfile.

Use `node scripts/pack-proof.mjs` to check the tarball route end to end. It packs the six packages, installs only the tarballs into a throwaway copy of `examples/starter` outside this repo (React, Vite, Tailwind and TypeScript come from the registry as the consumer's own dependencies), runs `ejm-ssg build`, `ejm-ssg budget`, `ejm-site-headers dist/_headers --check` and a typecheck, and diffs the HTML against the workspace build. **It needs network access to the npm registry, so it is not part of `npm run check`.** Run it after changing a `package.json`, a `files` or `exports` field, or one of the Vite plugins. Without `--dir`, the script creates a new temporary folder. To choose a folder, pass `--dir <folder>` outside the repo and reserve it for this tool: the script recursively deletes and recreates its `workspace-dist`, `tarballs` and `consumer` subfolders. It leaves the output folder in place for inspection. Results apply to the source and environment checked.

What the tarballs hold (the `files` field of each package):

| Package | Tarball | Contents |
| --- | --- | --- |
| `@ejm/tokens` | about 1.7 MB | `theme.json`, `src`, `generated`, `rust`, and `assets` without ZIP archives, `templates/`, `editable/` sources, TTF font sources and the large PNG renders (motifs; per-project logos, marks and icons). Additional retained source assets stay in this repo. |
| `@ejm/core` | about 9 KB | `src` and `contract.json`, no tests. |
| `@ejm/ui` | about 170 KB | `src` without tests, `public/theme-boot.js`, `THEME.md`, `COMPONENTS.md`. |
| `@ejm/ssg` | about 15 KB | `bin`, `src` without tests and perf tests, README. No fixtures. |
| `@ejm/site` | about 9 KB | `bin`, `src` without tests, README. |
| `@ejm/demos` | about 45 KB | `src` without tests, README. |

`@ejm/ui` depends at runtime on `clsx`, `tailwind-merge` and `class-variance-authority` (installed with it) and has `react`, `react-dom` and `tailwindcss` as peer dependencies, so list those in the consumer. Deep imports that resolve and typecheck for an outside consumer: `@ejm/ui/font-preloads`, `@ejm/ui/{app,docs,hooks,layout,motion,shell}` (the folder's index) and `@ejm/ui/{app,docs,hooks,layout,motion,shell,components}/<File>` (for example `@ejm/ui/layout/Hero`, `@ejm/ui/hooks/useTheme`, `@ejm/ui/docs/highlight` (the core tokenizer) and `@ejm/ui/docs/highlight-all` (with Python, PowerShell and TOML), `@ejm/ui/components/ui/button`), `@ejm/ui/lib/utils`, `@ejm/ui/art/*` and `@ejm/ui/fonts/*` (with the extension, for example `@ejm/ui/art/inline/divider-ripple.svg?raw`), `@ejm/ui/catalog.json`. Anything not listed in the `exports` of `packages/ui/package.json` is private.

## 2. Set up the site

Copy `examples/starter` and rename it, or add the same files:

```
index.html            the two ssg markers, charset, viewport with viewport-fit=cover
vite.config.ts        tailwindcss(), react(), ejmSite({ project }), ejmSsg({ site: 'src/site.tsx' })
tsconfig.json         strict, jsx react-jsx, moduleResolution bundler, types ["vite/client"]
.gitignore            node_modules, dist, .ejm-ssg
src/styles.css        @import "@ejm/ui/styles.css";
src/main.tsx          void hydrateSite(site)
src/site.tsx          defineSite({ ...siteDefaults('cli'), App, routes, notFound })
src/shell.tsx         App: SiteLayout (Header, Footer, linkComponent={Link}) and <Analytics />
```

`package.json` scripts: `"dev": "ejm-ssg dev"`, `"build": "ejm-ssg build"`, `"budget": "ejm-ssg budget"`.

Notes that cost time when missed:

- **Tailwind sources.** `@ejm/ui/styles.css` scans `components`, `layout`, `shell`, `hooks` and `motion`. Add each required partial after that import:

  - `@ejm/ui/styles-docs.css` for `DocsLayout`, `DocsSidebar`, `CodeBlock`, `ParamTable`, `Callout`, `Pager` and `Contents`.
  - `@ejm/ui/styles-app.css` for `Dashboard`, charts and `ActivityList`.
  - `@ejm/ui/styles-static.css` for `NativeCheckbox`, `Disclosure`, `ActionRows` and `DocumentContent`.
  - `@ejm/ui/styles-terminal.css` for the `Terminal*` views.

  A product demo site also adds `@import "@ejm/demos/styles.css";`. Measure the actual entries used by the consumer. Tailwind scans your project folder automatically. A source folder outside the project needs its own `@source "../other/src";` line in `src/styles.css`.

- **Keep `dist` and `.ejm-ssg` in `.gitignore`.** Tailwind skips ignored paths. Without that line it reads the previous build's HTML, and the CSS grew from 13.1 KB to 14.2 KB in the proof, enough to breach the 14 KB budget on the second build.
- **`tsconfig.json`.** Types resolve through the package `exports`, so with installed packages (route B) no `paths` are needed. With linked packages (route A) the four `paths` for `react` and `react-dom` shown in route A are required. Inside the `ejm-ui` repo the starter extends `tsconfig.base.json`; outside it, copy the options listed above.
- **Import deep or by name.** `@ejm/ui` has `sideEffects: ["*.css"]`, so `import { Band } from '@ejm/ui'` tree-shakes to the same bytes as `@ejm/ui/layout/Band` (`packages/ui/scripts/check-treeshake.mjs` checks it). A bare `@ejm/ui` in a module that runs in the first load is still worth avoiding when you only need one thing (`@ejm/ui/font-preloads`). `@ejm/site` and `@ejm/demos` never import the barrel.
- **Vite config import order.** The ejm-ide site reported TypeScript 6.0.3 crashing with "parameter should have errors" when the `@ejm/*` imports in `vite.config.ts` came before the `vite` imports. A throwaway consumer with the ejm-ide `tsconfig` options and `@ejm/*` first typechecked cleanly on 6.0.3 and 5.9, and the plugin typings are plain `Plugin` returns, so this looks specific to that project's file set. If it happens, put the `vite`, `@tailwindcss/vite` and `@vitejs/plugin-react` imports first.
- **Routes** are written the way Cloudflare serves them: `/about/` with the slash, so there is no redirect. One `h1` per route, and `head: { title, description }` on each.
- **Links.** Internal links use `Link` from `@ejm/ssg` (give it to `SiteLayout` as `linkComponent`). Other sites use `otherSites('cli')` from `@ejm/site/projects` in the nav or footer, which renders `ExternalLink`, or `<Row external>`: both set `rel="external"`.
- **Header.** `Header` puts the nav and then the theme control last in the tools group, at the far right. `themeControl` omitted or `undefined` keeps `ThemeToggle`; `null` suppresses it. A replacement stays in that final slot.
- **Heavy features are lazy.** A route with a demo or code highlighting uses `load: () => import('./pages/Demo')`, so the initial JavaScript stays under 95 KB.
- **A product demo.** Add `@ejm/demos` and `@import "@ejm/demos/styles.css"` (or its `@ejm/demos/ide.css` entry for the current IDE views), and import the entry for the product you show, so you do not ship the views of the other two: `LazyCliDemo` from `@ejm/demos/islands/cli-lazy` (also `ide-lazy`, `harness-lazy`), or `ProductHeroStage` with `CliHeroWindow` from `@ejm/demos/hero`. `LazyDemo` and `ProductHero` take any product but carry all three views (about 5 KB gzip more). Details and measured sizes: `packages/demos/README.md`.
- **Prose that needs no script.** A docs page keeps its text in the HTML with `<Static>` from `@ejm/ssg` (see `packages/ssg/README.md`), which keeps the guide text out of the JavaScript budget.
- **Static pages that are not routes** (copied brand assets) are measured as routes by the budget gate. Skip them with `ignore: ['brand/assets/**']` in `defineSite`, or `ejm-ssg budget --ignore <glob>` and `node scripts/perf.mjs <dist> --ignore <glob>`.

### Native product pages

A product can prerender `NativeCheckbox`, `Disclosure`, `ActionRows`,
`DocumentContent` and the `Terminal*` view components through public `@ejm/ui`
exports. Import `@ejm/ui/styles.css` first. Add `@ejm/ui/styles-static.css`
for the static controls and document layouts. Add
`@ejm/ui/styles-terminal.css` for `Terminal*` views. A page that uses both
groups needs both partials. The partials share the same theme. Read
[Native static pages](packages/ui/COMPONENTS.md#native-static-pages)
for the exact form, disclosure and transcript-template contracts.

Keep authentication, transport, pairing verification, content validation and
command logic in the product. An external script can bind native DOM handlers
without shipping React in the browser. Use shared component output as the
template, fill user text with `textContent`, and keep source pins and consumer
checks current. `NativeCheckbox` uses `@ejm/ui/components/NativeCheckbox`.
The other static parts keep their public shell and layout deep paths. The
changed source still needs its installed consumer and pack proof. Source
documentation does not establish deployment or browser acceptance.

### Theme boot, head, analytics, headers (all from `@ejm/site`)

`ejmSite({ project: 'cli' })` does the head and the files, `siteDefaults('cli')` does the route head, and `<Analytics />` goes once in `App`. After a build, `dist` holds `theme-boot.js` (an external file, so the policy keeps `script-src 'self'`), the favicon set, `site.webmanifest`, `social-card.png` and `_headers`. Details and every option are in `packages/site/README.md`. The shared rules to keep:

- The theme choice is the `ejm-theme` cookie on `.ejmlabs.com`, so a visitor who picks dark on one site sees dark on the others on first paint. `?theme=dark` is a test override that is never saved.
- Analytics is Cloudflare Web Analytics only, off on localhost and previews, off for Do Not Track and Global Privacy Control, no cookies. Say so in the footer `privacy` slot, as the starter does.
- The headers are the one shared policy. A site must not hand-edit `_headers` or keep its own copy in `public/`. To extend the policy (an image host), pass `headers: { extra: { 'img-src': [...] } }`.
- `ejm-site-headers dist/_headers --check` verifies a built or committed file against the policy.

## 3. Deploy configuration (Cloudflare Workers static assets)

`wrangler.jsonc` next to `package.json`. The account and the routes are the ones the sites already use; change `name` and the pattern per site:

```jsonc
{
  "name": "ejmlabs-cli",
  "account_id": "478a0db3a110d8503c16abbc1db27af4",
  "compatibility_date": "2026-10-18",
  "assets": {
    "directory": "./dist",
    "not_found_handling": "404-page"
  },
  "routes": [{ "pattern": "cli.ejmlabs.com", "custom_domain": true }]
}
```

`not_found_handling: "404-page"` serves the nearest `404.html`, which `ejm-ssg` writes at `dist/404.html` with `noindex`. `_headers` in `dist` is read by the asset server. Do not run `wrangler deploy` until the owner approves a deploy.

## 4. Per-site checklist

For every site: delete the `sync-design-kit.sh` script and the copied `design-kit/` folder, the `<link>` and `<script>` tags that pointed at it and `analytics.js`; move static files that are not part of the design system (`robots.txt`, `sitemap.xml`, `license.txt`, product images) into `public/`; list the routes; set the budget; check the links; run the starter's checks against the new site (skip link first, one `h1` per route, toggle last in the header tools, no inline script, external links marked, `_headers` current).

| Site | Registry id | Route list to build | Notes |
| --- | --- | --- | --- |
| ejmlabs.com | `ejmlabs` | `/`; include any routes the home page links to | Keep studio content in the studio repository and compose it with shared components. |
| cli | `cli` | the pages in `ejm-cli/site/dist` today | Product windows and demos load with `load: () => import(...)`, from `@ejm/demos`. |
| ide | `ide` | `/`, `/docs/`, `/docs/user-guide/`, `/docs/developer-guide/` | Docs use `DocsLayout`. Keep `public/` files (`editor.webp`, `license.txt`, `notices.txt`, `robots.txt`, `sitemap.xml`). The `/api/` and `/downloads/` dev proxy moves into `server.proxy` in `vite.config.ts`. `website/scripts/check-docs.mjs` stays. |
| harness | `harness` | the pages in `ejm-harness/site/public` today | Same shape as cli. |

Set `budget` in `defineSite` only to raise a limit for a measured reason (defaults: JS 95, CSS 14, fonts 70, HTML 30 KB gzip). The starter measures 86.8 JS, 11.9 CSS and 33.7 fonts (it was 89.6, 13.1 and 33.7 before the slimmer `cn` and the opt-in docs and app CSS): write pages from the shared components and do not add CSS. Product demos add 1.7 KB of CSS, the docs partial about 1 KB.

Adopt and verify each consumer in its own repository.

## 5. The IDE app (not a prerendered site)

The Tauri app in `ejm-ide` is a single Vite and React 19 package with Tailwind 4.3. It adopts the design system with no adapter, and it does not use `@ejm/ssg` or `@ejm/site`:

1. Depend on `@ejm/tokens`, `@ejm/ui` and `@ejm/core` (route A or B above, plus the same `dedupe` line for route A).
2. Replace the app's theme CSS: its entry stylesheet starts with `@import "@ejm/ui/styles.css";` and any `@source` line for folders outside the app root. Colors come only from the semantic names in `packages/ui/THEME.md`; remove literals as screens are touched.
3. Copy `@ejm/ui/theme-boot.js` to the app's `public/` and load it synchronously in `index.html` before the stylesheet, so the first frame has `data-theme` (the Tauri webview is not an ejmlabs host, so the choice is kept in `localStorage["ejm-theme"]`, which `useTheme` and the boot script already handle).
4. Use `AppShell`, `ThemeToggle`, the `components/ui` primitives and the hooks directly. The editor keeps Monaco; load the generated themes from `@ejm/tokens/generated/monaco`. The editor font stays a choice of the app, the interface fonts are Manrope and IBM Plex Mono from `@ejm/ui`.
5. The app's own `Content-Security-Policy` (in `tauri.conf.json`) keeps `script-src 'self'` for the same reason: the boot script is a file.

The IDE's marketing site (`ide/website`) is a normal site (section 4).

## 6. Keeping product repos in step

- Linked (route A): nothing to copy. Pull `ejm-ui`, run `npm install` there if its dependencies changed, then rebuild the product. There is no sync script to run and no copy to keep.
- Tarballs (route B): run the `npm pack` loop above, delete `node_modules/@ejm` and the lockfile in the product, install, rebuild, and commit the lockfile. Do it when `ejm-ui` changes something the product needs, not on a schedule.
- All packages currently declare `0.1.0` and remain private. Pin source by its full commit, and regenerate the consumer lockfile when that source changes. Standard source releases need the shared UI version validator and release workflow before a version tag can serve as a release reference. A future registry publication requires the owner's approval and a versioned package release.
- After any update, the product's own check must pass: its budgets (`ejm-ssg budget`), its route list and the shared rules in section 4. Brand files come from `@ejm/tokens` only, so never edit a copy of them in a product repo.

## Package notices and the IDE stylesheet entry

Every owned npm package carries `LICENSE` and `THIRD_PARTY_NOTICES.md` in its
package output. `@ejm/tokens` keeps its original OFL notices in `assets/fonts/`;
`@ejm/ui` carries the same original notices beside its subsets in `src/fonts/`.
Preserve these files when assembling source or tarball distributions. Software
uses the MIT terms, brand materials retain their owners' rights, and fonts keep
their own license.

`@ejm/demos/ide.css` is a narrow compatibility entry owned by `ejm-ui`. Studio
and IDE source consumers already name that public path. It imports the existing
shared demo stylesheet for the current simulated IDE views. The upstream target
is the same public path. Replace this forwarding file when an accepted UI
change supplies a distinct IDE stylesheet together with its matching view
markup and visual checks. Check package resolution, the target file, and the
matching consumer build whenever that contract changes. This entry does not
bring other local view or stylesheet changes into the source snapshot.
