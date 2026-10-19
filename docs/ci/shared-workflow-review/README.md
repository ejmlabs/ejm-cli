# Shared workflow review source

## Historical ae903/C4 pair

This retained bundle records the earlier CLI PR #4 comparison on 2026-10-19.
The current selector adoption uses the complete canonical C4/b2 pair in
[`.github/review-context/manifest.json`](../../../.github/review-context/manifest.json).
That manifest identifies both exact revisions and all 16 source paths per
revision. These older copies remain to keep the adoption patch below the
1,000,000-byte admission limit without relying on rename detection. Their
later removal is separate cleanup; they are not the current selector evidence.

This directory supplied the external source needed to review the paired PR
and draft-release caller update. It contains documentation data, with `.txt`
suffixes and no executable modes. The workflow callers, product gate, runtime
and dependency pins do not read or execute these copies. The
[copied MIT notice](LICENSE.txt) also covers the current canonical central
source export.

## Source revisions

- Repository: `ejmlabs/.github`.
- Historical installed caller: `ae903129564926a14287f75d2012610c59cccb65`.
- Historical proposed caller: `c4f2873ef0eef5bc3c7d75cf71fa3489a911d134`.

CLI main `189170a37b3efab4e842266cb09d8aeeeb4d5d1b` already adopted the
installed caller. Shared PR #14 adopted the proposed combined implementation
as protected main `d00d8bbc7231485eb918008c562d835f394e82ea`. The manifest
records both source trees and exact `workflow-pins/FULL_SHA` retention refs.
Check those refs and their protection before human admission. A shared adoption
receipt does not establish completed review or hosted execution in this CLI.

## Contents and integrity

[The manifest](manifest.json) maps each source revision and original path to
its exact snapshot, source mode, Git blob SHA-1, SHA-256 digest, byte count and
line count. It also records each implementation's Git tree.
Preparation compared full-revision source bytes and recomputed their Git blob
IDs with Git. Matching blobs have one shared copy; an existing snapshot path
may serve both revisions. The 34 source-path records use 21 plaintext files
containing 462588 bytes. Copies retain safe, nonexecutable file modes even when
the manifest records an executable original source mode.
The digests identify content; they are not digital signatures or independent
proof of maintainer approval. [The copied MIT notice](LICENSE.txt) covers the
central source.

The generated workflow copies preserve the actual called implementation.
The generator, libraries and templates expose the readable source embedded as
base64 in those workflows. Both release and PR behavior belong to the pin
comparison. Four original paths differ between these revisions: the generated
PR workflow, `review/prompt.md`, `review/schema.json`, and
`scripts/review_result.py`. The CLI setup hook and release behavior are retained.

| Original central path                        | Previous source                                               | Proposed source                                               |
| -------------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------- |
| `.github/workflows/governance.yml`           | [text](old/workflows/governance.yml.txt)                      | [text](new/workflows/governance.yml.txt)                      |
| `.github/workflows/draft-release.yml`        | [text](new/workflows/draft-release.yml.txt)                   | [text](new/workflows/draft-release.yml.txt)                   |
| `scripts/build_workflows.py`                 | [text](new/scripts/build_workflows.py.txt)                    | [text](new/scripts/build_workflows.py.txt)                    |
| `scripts/governance.py`                      | [text](common/scripts/governance.py.txt)                      | [text](common/scripts/governance.py.txt)                      |
| `scripts/prepare.py`                         | [text](common/scripts/prepare.py.txt)                         | [text](common/scripts/prepare.py.txt)                         |
| `scripts/release.py`                         | [text](common/scripts/release.py.txt)                         | [text](common/scripts/release.py.txt)                         |
| `scripts/review_result.py`                   | [text](common/scripts/review_result.py.txt)                   | [text](new/scripts/review_result.py.txt)                      |
| `scripts/standards.py`                       | [text](new/scripts/standards.py.txt)                          | [text](new/scripts/standards.py.txt)                          |
| `review/prompt.md`                           | [text](common/review/prompt.md.txt)                           | [text](new/review/prompt.md.txt)                              |
| `review/schema.json`                         | [text](common/review/schema.json.txt)                         | [text](new/review/schema.json.txt)                            |
| `LICENSE`                                    | [text](LICENSE.txt)                                           | [text](LICENSE.txt)                                           |
| `templates/community/CODE_OF_CONDUCT.md`     | [text](common/templates/community/CODE_OF_CONDUCT.md.txt)     | [text](common/templates/community/CODE_OF_CONDUCT.md.txt)     |
| `templates/community/CONTRIBUTING.md`        | [text](common/templates/community/CONTRIBUTING.md.txt)        | [text](common/templates/community/CONTRIBUTING.md.txt)        |
| `templates/community/RELEASING.md`           | [text](common/templates/community/RELEASING.md.txt)           | [text](common/templates/community/RELEASING.md.txt)           |
| `templates/community/SECURITY.md`            | [text](common/templates/community/SECURITY.md.txt)            | [text](common/templates/community/SECURITY.md.txt)            |
| `templates/community/SUPPORT.md`             | [text](common/templates/community/SUPPORT.md.txt)             | [text](common/templates/community/SUPPORT.md.txt)             |
| `templates/community/THIRD_PARTY_NOTICES.md` | [text](common/templates/community/THIRD_PARTY_NOTICES.md.txt) | [text](common/templates/community/THIRD_PARTY_NOTICES.md.txt) |

## Review transport and limits

The existing trusted preparation job archives committed consumer files into
`product.tar`. Each independent reviewer receives that archive under
`review-data/product/`, along with `context.json` and `change.patch`. It does
not receive a central checkout. PR bodies, comments and remote URLs are not
source evidence in that archive. The trusted reviewer uses text-only,
read-only inspection without network access; a binary Git bundle alone would
not supply readable implementation context.

These copies remain untrusted review material. They change no trusted prompt,
review authority, permissions, action policy, human admission or exact-head
approval. They are source context, not proof that a review completed. A new
consumer head still needs its normal gate, fresh admission, two complete
reviews and its own explicit approval before protected adoption.

The earlier caller-head review reports explicitly said the external-source
comparison was incomplete despite successful wrapper jobs and zero findings.
Those reports do not establish acceptance. Source publication, hosted quality
and completed review remain separate checks.

That historical adoption ran through installed `ae903129...`, which lacks
the completion field. Inspect both actual reports and transcripts for completed
scope before merging. After adoption, a fresh admitted PR must test the installed
guard, which requires JSON boolean `complete: true` in each review result.
This retained bundle establishes neither hosted result. See the current
[selector adoption order](../../ci-bootstrap.md#protected-harness-selector-adoption)
for the C4/b2 transition and its remaining checks.
