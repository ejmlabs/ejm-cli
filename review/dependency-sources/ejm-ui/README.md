# UI install review context

These are exact committed source copies from `ejmlabs/ejm-ui` at
[`24935aa85355ea99f49bdbecc4186a5e95049393`](https://github.com/ejmlabs/ejm-ui/commit/24935aa85355ea99f49bdbecc4186a5e95049393),
tree `2d8bde67c8b5fe851b70ae90a3a2787091397780`.
This is the actual protected squash merge of [Relay asset PR #10](https://github.com/ejmlabs/ejm-ui/pull/10),
which follows the shared UI change in PR #7. Its full tree matches reviewed head
`059e8b62b6961602a07bd08b8f72dbfc6c0266a3`; its sole parent is
`593bbf1e03c084985fd1b0ea4de54183e256aadb`. The CLI selects this source in
[the UI pin](../../../.github/ui-source.json). This prerequisite remains subject
to its own human admission, hosted checks, reviews and protected adoption.

The [manifest](manifest.json) binds all 13 copied files, totaling
189,724 bytes and 5,581 lines, to repository, commit, tree,
source path, Git mode, blob, byte count, SHA-256 and fixture path/mode.
All source files and fixtures are ordinary nonexecutable `100644` files. Source
bytes were read by Git blob ID and their Git blob hashes were checked; written
bytes, SHA-256 values and nonexecutable modes were checked afterward.

The copied [license](24935aa85355ea99f49bdbecc4186a5e95049393/LICENSE) retains UI's license scope:
MIT covers software and documentation; marks, brand artwork, design assets,
templates and visual identity retain their owners' rights unless separately
licensed. Font rights and notices remain separate. A consumer's license does
not relicense these snapshots.

## Complete install boundary

The complete [root manifest](24935aa85355ea99f49bdbecc4186a5e95049393/package.json),
[root lockfile](24935aa85355ea99f49bdbecc4186a5e95049393/package-lock.json) and all ten root
workspace manifests cover `packages/*`, `apps/*` and `examples/*`:
`apps/gallery`, `examples/starter`, and
`packages/{core,demos,ink,shots,site,ssg,tokens,ui}`.
The lock's ten workspace links match those exact directories and package names.
Compared version, dependency and platform/engine metadata matches source.
Eight `packages/*` manifests declare `SEE LICENSE IN LICENSE`, while their
workspace lock records omit `license`; the manifest records each omission.
Keep the complete source license and manifests. The tracked nested
`packages/shots/fixtures/acme-api/package.json` is neither a root workspace nor
a root lock workspace entry and is outside this 13-file install closure.

The version-3 lock has 300 external resolved records. Every record
uses HTTPS `registry.npmjs.org` and has syntactically valid SHA-512 integrity.
No dependency tarball was downloaded or checked. The lock flags install scripts
for `fsevents` 2.3.3 and `node-pty` 1.1.0; their complete lock records are in the
manifest. The existing `npm ci --ignore-scripts` recipe suppresses lifecycle
hooks. No installation or rebuild ran while creating these snapshots. The owner CI result
is separate from a consumer install against the protected CLI pin.

The root requires Node `>=22`. Root and workspace manifests have no install
lifecycle scripts (`preinstall`, `install`, `postinstall`, `prepublish`,
`preprepare`, `prepare`, `postprepare`). They have no `config`, `overrides`,
`packageManager`, `publishConfig`, `resolutions`, `installConfig`, OS/CPU/libc
restrictions, optional dependency map, peer-dependency metadata or bundled
dependency map. Their complete manifests retain all build scripts, dependencies,
dev dependencies, peer dependencies, exports and other fields unchanged.
The manifest records which checked install fields are present in each file.

At this exact commit no tracked `.npmrc`, `npm-shrinkwrap.json`, `.nvmrc`,
`.node-version`, `.yarnrc`, `.yarnrc.yml`, `.yarnrc.yaml`,
`pnpm-workspace.yaml`, `.pnpmfile.cjs`, `.pnpmfile.mjs`, `pnpm-lock.yaml`,
`yarn.lock`, `.pnp.cjs`, `.pnp.loader.mjs`, `bun.lock`, `bun.lockb` or
`lerna.json` exists anywhere in the owner tree. These checks cover tracked owner
files, not runner or user configuration. No absent configuration was invented.
All 13 source modes, blobs, sizes and SHA-256 bindings also match the accepted
Studio install context at `7cf86428d0266297901ee52d0e137df100f8c23d`.

## Use and acceptance limits

Treat these copies as read-only review data. Never import, execute or package
them as an implementation. The canonical `.github/review-context` stays
separate. UI owns active packages; consumers use qualified public source.

This set is install metadata, not a runnable UI checkout, advisory audit,
installed-run proof, package publication or deployment. The first pin-installing
PR's old trusted base cannot supply the proposed UI archive. A fresh admitted
PR after the prelude merges must verify the installed pin, exact UI archive and
context, lifecycle-disabled install, both complete reviews and all protected
gates. Existing exact-revision readiness, separate final approval, closed
rollout and OpenSpec acceptance requirements remain in force.
