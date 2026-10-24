# Relay dependency review prerequisite

This inert slice includes [the exact lockfile](package-lock.json) at its future
application path. Runnable `relay/package.json`, Worker configuration and the
Relay application are absent. The complete readable 51-line
[planned manifest](../review/dependency-sources/relay-npm/sha256-878dbbec27fdc01a889ee2e470e9f343fb9bbfb7d8f040c9441b9b589d8ad8e3/package.json)
is ordinary nonexecutable review data outside the runnable package.

The [provenance manifest](../review/dependency-sources/relay-npm/manifest.json)
binds both files' paths, modes, sizes and digests. Its `sha256-878dbbec...` source
identity addresses the exact path-and-content pair described there. It is a
prepared candidate snapshot, not a committed CLI revision; computed Git blob
identities make no claim that these bytes belong to a commit.

## Planned install inputs

The manifest records Node `26.12.0` and npm `11.20.0`. Its direct dependency
fields match the version-3 lock root. The three local public packages,
`@ejm/core`, `@ejm/tokens` and `@ejm/ui`, bind to protected UI source through
the separate [UI review context](../review/dependency-sources/ejm-ui/README.md),
not their `0.1.0` versions or local paths alone.

The lock contains 304 package records:

| Record kind | Count |
| --- | ---: |
| npm registry tarballs | 294 |
| Local `file:` packages | 3 |
| Bundled child packages | 6 |
| Root package | 1 |

The 294 registry records use HTTPS `registry.npmjs.org` URLs and syntactically
valid SHA-512 integrity. The six bundled children are inside
`@tailwindcss/oxide-wasm32-wasi` version `4.3.3`. Its parent tarball record has
SHA-512 integrity; the children have `inBundle` records without separate
tarball URLs or integrity values. Parent integrity does not claim independent
child verification. No tarball was downloaded or verified during preparation.

The provenance manifest retains install-hook records for `esbuild`, `fsevents`
and `workerd`. These are recorded inputs, not executed hooks. Review fixtures
retain complete scripts for inspection; do not run them as an application.
The later consumer install must use the existing `npm ci --ignore-scripts`
recipe and verify the installed tools separately.
Package licenses and notices remain governed by their source licenses.

## Use and acceptance limits

This prerequisite starts on protected native merge
`71d0f47f8b05c6314eca4dc0750ffaad833d6afb`, tree
`3adf3701aec18883b2aabd710e72fa3c513c7fdf`. It preserves that parent's
patched native dependency graph, gates, harness and workflows.
The later complete Worker PR must inherit the merged UI pin selected from
protected BASE and verify its full source archive and installation separately.

These files do not activate the Relay install step: its runnable manifest is
absent. Static input verification does not establish a Relay installation,
build, test or Cloudflare result. This is not an accepted prerequisite PR, an advisory or license audit,
browser/native acceptance, deployment or live release. Admission, complete
reviews, exact approval, protected adoption and final product gates remain.
