---
name: ejm-cli-site
description: Change, preview or deploy the cli.ejmlabs.com marketing site under site/, including its design kit copy, headers, copy and Cloudflare Worker.
---

# ejm-cli marketing site

Read `site/dist/index.html`, `site/dist/_headers`, `site/wrangler.jsonc` and `scripts/sync-design-kit.sh`. The site is static files with no build step. It is the public page for "cli by ejmlabs", while the program, crate and binary are `ejm-cli`.

- The page uses the ejmlabs site kit. `scripts/sync-design-kit.sh` copies the kit files it needs from `../ejmlabs/design-kit` (or `DESIGN_KIT`) into `site/dist/design-kit`. Never edit that copy. Change the kit, then rerun the script.
- Light and dark follow `prefers-color-scheme`. Keep the logo swap, contrast and both modes working, and keep the page readable without scripts.
- Keep `_headers` as strict as the page allows. Add a source to the Content Security Policy only for a file the site actually ships.
- Copy states only facts the repo can prove: [the CLI reference](../../../docs/ejm-cli.md) and [benchmark evidence](../../../docs/benchmarks.md). Use `$ejm-cli-docs` for claims. Do not show a repository URL, install command or download until `$ejm-cli-release` says one exists.
- Terminal examples must be real output, and condensed output must say so.

Preview with `python3 -m http.server 4831 --directory site/dist --bind 127.0.0.1`. Check desktop and a 375 px phone in light and dark, that `scrollWidth` equals `clientWidth`, keyboard focus, the skip link and an empty console.

Deploy only when the user asks to publish. Production goes live at once on `cli.ejmlabs.com` through the Worker `ejmlabs-cli`: `wrangler deploy --config site/wrangler.jsonc`. Never touch other Workers on the account, including `gerry-*`, `ejmlabs`, `ejmlabs-ide` and `ejmlabs-harness`. Afterward, fetch the live page, check status and security headers with `curl -sI`, and look at it in a browser.
