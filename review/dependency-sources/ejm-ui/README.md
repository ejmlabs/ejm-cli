# UI install review context

The [manifest](manifest.json) binds the complete npm install metadata boundary to
protected `ejmlabs/ejm-ui` commit
[`79a1aa8b1075f834717bc0c0299dc2c41a22b966`](https://github.com/ejmlabs/ejm-ui/commit/79a1aa8b1075f834717bc0c0299dc2c41a22b966),
tree `c18c74fd336dbad9515bff8ecf22ca980b889a51`, the accepted UI PR #11 merge.
Shared source acceptance is separate from consumer pin adoption and release.

## Complete readable files

All thirteen files remain complete and readable: the license, root manifest,
root lock and ten workspace manifests for `apps/gallery`, `examples/starter`
and `packages/{core,demos,ink,shots,site,ssg,tokens,ui}`.

The manifest records fresh current-source mode, blob, size, SHA-256 and fixture
byte-identity checks. Twelve unchanged files reuse retained fixture paths under
`24935aa85355ea99f49bdbecc4186a5e95049393/`, with explicit reuse provenance.
Only the changed [UI manifest](79a1aa8b1075f834717bc0c0299dc2c41a22b966/packages/ui/package.json)
has a new fixture under the current commit. Every source and fixture mode is
ordinary nonexecutable `100644`. Fixture paths identify readable copies; the
current commit and tree identify the selected owner source.

The twelve retained files are present from protected native parent
`71d0f47f8b05c6314eca4dc0750ffaad833d6afb`. All thirteen files are
present in this checkout. The superseded M UI manifest remains historical data;
it is outside the thirteen selected bindings. No selected file is omitted or
physically duplicated.

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

The full protected source archive supplies those TSX/CSS targets separately.
These metadata fixtures are read-only review data, never private source imports
or implementation files to execute or package. Canonical `.github/review-context`
stays separate and unchanged.

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
