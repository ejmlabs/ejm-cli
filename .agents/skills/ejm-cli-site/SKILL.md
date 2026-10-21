---
name: ejm-cli-site
description: Change, preview or deploy the cli.ejmlabs.com marketing site under site/, including its design kit copy, headers, copy and Cloudflare Worker.
---

# ejm-cli marketing site

Read `site/dist/index.html`, `site/dist/_headers`, `site/wrangler.jsonc` and `scripts/sync-design-kit.sh`. The site is static files with no build step. It is the public page for "cli by ejmlabs", while the program, crate and binary are `ejm-cli`.

- The page uses the ejmlabs site kit. `scripts/sync-design-kit.sh` copies the kit files it needs from `../ejmlabs/design-kit` (or `DESIGN_KIT`) into `site/dist/design-kit`, finding motifs, icons and fonts by reading the kit CSS `url()` references. Never edit that copy. Change the kit, then rerun the script.
- Light and dark follow `prefers-color-scheme`, with the kit's `theme.js` toggle on top. Keep contrast and both modes working, and keep the page readable without scripts.
- Keep `_headers` as strict as the page allows. Scripts come from the kit's `theme.js`, `motion.js` and `analytics.js`. The policy allows `'self'` plus the Cloudflare Web Analytics hosts `static.cloudflareinsights.com` and `cloudflareinsights.com`. Add a source only for something the page needs, and write down why. Analytics is cookieless, skipped for Do Not Track and Global Privacy Control, and runs only on the hosts listed in `analytics.js`, so localhost and previews send nothing.
- Copy states only facts the repo can prove: [the CLI reference](../../../docs/ejm-cli.md) and [benchmark evidence](../../../docs/benchmarks.md). Use `$ejm-cli-docs` for claims. Do not show a repository URL, install command or download until `$ejm-cli-release` says one exists.
- Terminal examples must be real output, and condensed output must say so.

Preview with `python3 -m http.server 4831 --directory site/dist --bind 127.0.0.1`. Check desktop and a 375 px phone in light and dark, that `scrollWidth` equals `clientWidth`, keyboard focus, the skip link and an empty console.

Deploy only when the user asks to publish. Production goes live at once on `cli.ejmlabs.com` through the Worker `ejmlabs-cli`: `wrangler deploy --config site/wrangler.jsonc`. Leave every other Worker unchanged. Afterward, fetch the live page, check status and security headers with `curl -sI`, and look at it in a browser.
