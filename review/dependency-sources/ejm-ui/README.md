# UI source and install review context

The [manifest](manifest.json) binds the complete npm install metadata boundary to
protected `ejmlabs/ejm-ui` commit
[`79a1aa8b1075f834717bc0c0299dc2c41a22b966`](https://github.com/ejmlabs/ejm-ui/commit/79a1aa8b1075f834717bc0c0299dc2c41a22b966),
tree `c18c74fd336dbad9515bff8ecf22ca980b889a51`, the accepted UI PR #11 merge.
Shared source acceptance is separate from consumer pin adoption and release.

## Complete install metadata

All thirteen files remain complete and readable: the license, root manifest,
root lock and ten workspace manifests for `apps/gallery`, `examples/starter`
and `packages/{core,demos,ink,shots,site,ssg,tokens,ui}`.

The manifest records fresh current-source mode, blob, size, SHA-256 and fixture
byte-identity checks. Twelve unchanged files reuse retained fixture paths under
`24935aa85355ea99f49bdbecc4186a5e95049393/`, with explicit reuse provenance.
Within this metadata boundary, only the changed
[UI manifest](79a1aa8b1075f834717bc0c0299dc2c41a22b966/packages/ui/package.json)
has a new fixture under the current commit. Every source and fixture mode is
ordinary nonexecutable `100644`. Fixture paths identify readable copies; the
current commit and tree identify the selected owner source.

The twelve retained files are present from protected native parent
`71d0f47f8b05c6314eca4dc0750ffaad833d6afb`. All thirteen files are
present in this checkout. The superseded M UI manifest remains historical data;
it is outside the thirteen selected bindings. No selected file is omitted or
physically duplicated.

## Complete changed-owner source supplement

The unchanged trusted workflow selects the UI archive from the protected product
BASE pin. For this prerequisite, it supplies the full 932-file source at
`24935aa85355ea99f49bdbecc4186a5e95049393`, tree
`2d8bde67c8b5fe851b70ae90a3a2787091397780`. That archive supplies every
unchanged file in the current owner tree, including npm configuration scope,
workspace manifests, lock, licenses, notices and assets.

The separate [source supplement](source-supplement.json) binds every changed
owner path from that base to current `79a1aa8b1075f834717bc0c0299dc2c41a22b966`.
It contains complete target files for all 19 modified paths and nine additions,
with no deletion, rename or mode change. The 28 files total 175,936 bytes and
2,941 lines. The already selected UI manifest is reused; the other 27 files
add 172,326 bytes and 2,841 lines. All copies are complete UTF-8 text with
ordinary nonexecutable mode `100644`.

Each entry records the owner path, base mode/blob/size/SHA-256 or absence,
target mode/blob/size/SHA-256 and readable fixture path. Retain the full base
archive's unchanged files, replace its 19 modified paths with their complete
target copies and include the nine additions. This exposes the exact 941-file
target tree `c18c74fd336dbad9515bff8ecf22ca980b889a51` for static inspection,
without executing source or changing the workflow selector. The original owner
documents retain their relative links; resolve them against that composed
owner tree, not this sparse fixture directory.

The thirteen-file install metadata boundary remains unchanged. Source inspection
does not establish that CI supplied a full target archive or that the target
was installed or compiled. The following complete Worker PR must inherit the
merged UI pin from protected BASE, verify that full archive and pass its
lifecycle-disabled installation and product gates.

## Public export changes

The complete UI manifest retains these seven public exports:

| Public `@ejm/ui` suffix | Owner source target |
| --- | --- |
| `styles-terminal.css` | `src/styles-terminal.css` |
| `components/TerminalWorkspace` | `src/terminal/TerminalWorkspace.tsx` |
| `styles-static.css` | `src/styles-static.css` |
| `components/NativeCheckbox` | `src/static/NativeCheckbox.tsx` |
| `shell/Disclosure` | `src/static/Disclosure.tsx` |
| `layout/ActionRows` | `src/static/ActionRows.tsx` |
| `layout/DocumentContent` | `src/static/DocumentContent.tsx` |

The source supplement supplies the complete current TSX/CSS targets alongside
all other changed owner files. Metadata and source fixtures are read-only review
data. Never import, install, execute or package them as an implementation.
Production consumes public owner exports. Canonical `.github/review-context`
and the trusted workflow selector stay separate and unchanged.

## License and acceptance limits

The retained [license](24935aa85355ea99f49bdbecc4186a5e95049393/LICENSE)
covers software and documentation under MIT. Marks, brand artwork, design
assets, templates and visual identity retain their owners' rights unless
separately licensed. Font rights and notices remain separate. A consumer license
does not relicense these files or assets.

This inert prerequisite starts on that actual protected native parent. Static
input verification establishes no accepted prerequisite PR, consumer
installation, build, Cloudflare action or live release. The later consumer still
needs the protected pin/archive binding, normal gates, complete reviews, exact
approval and protected adoption.
