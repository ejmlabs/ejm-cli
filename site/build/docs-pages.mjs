// The documentation pages: one entry per route. `file` is a Markdown source, relative to the repository root. The guides live in
// site/content (authored for this site); the references are the repository's own docs/*.md, which stay the source of truth.
// Titles, leads and descriptions are written here once, so the Markdown files stay plain and read the same on GitHub.

export const GUIDES = 'Guides';
export const REFERENCE = 'Reference';

/** @type {import('./docs-pages.d.mts').DocEntry[]} */
export const docPages = [
  {
    path: '/docs/',
    file: 'site/content/docs.md',
    group: GUIDES,
    label: 'Overview',
    title: 'CLI documentation',
    lead: 'Build the terminal app, connect a model, or contribute a change.',
    description: 'User and developer documentation for ejm-cli: source setup, providers, terminal controls, native remote sessions, credential recovery, and contributing.',
  },
  {
    path: '/docs/user-guide/',
    file: 'site/content/user-guide.md',
    group: GUIDES,
    title: 'User guide',
    lead: 'Build the CLI, connect your model, and work with clear grants and saved sessions.',
    description: 'Practical ejm-cli user guide covering source builds, provider access, permissions, terminal controls, saved sessions, native remote control, package trust, and troubleshooting.',
  },
  {
    path: '/docs/developer-guide/',
    file: 'site/content/developer-guide.md',
    group: GUIDES,
    title: 'Developer guide',
    lead: 'Understand the host boundary, make a focused change, and verify the terminal app and its site.',
    description: 'ejm-cli developer guide: source setup, Rust module boundaries, quality checks, offline fixture testing, native remote transport, contribution gates, and documentation maintenance.',
  },
  {
    path: '/docs/relay-notice/',
    file: 'docs/relay-notice.md',
    group: 'Relay policies',
    label: 'Signup and privacy notice',
    title: 'Relay signup and privacy notice',
    lead: 'Information, retention, privacy choices and service limits for the bounded Relay beta.',
    description: 'Relay signup and privacy notice: approved for publication, unpublished, with its effective date pending actual publication.',
  },
  {
    path: '/docs/relay-service-terms/',
    file: 'docs/relay-service-terms.md',
    group: 'Relay policies',
    label: 'Service terms',
    title: 'Relay beta service terms',
    lead: 'Access, allowance, pairing, reliability and recovery terms for the Relay beta.',
    description: 'Relay beta service terms: approved for publication, unpublished, with its effective date pending actual publication.',
  },
  {
    path: '/docs/reference/cli/',
    file: 'docs/ejm-cli.md',
    group: REFERENCE,
    label: 'CLI reference',
    title: 'CLI reference',
    lead: 'Arguments, provider accounts, interaction, packages, sessions and rendering bounds.',
    description: 'The ejm-cli command contract: arguments, environment, provider accounts and models, terminal interaction, sessions, packages and trust, and rendering bounds.',
  },
  {
    path: '/docs/reference/remote-control/',
    file: 'docs/remote-control.md',
    group: REFERENCE,
    label: 'Remote control',
    title: 'Remote control',
    lead: 'Share one selected session through an account-gated relay: pairing, encryption limits, accounts and recovery.',
    description: 'The ejm-cli remote control reference: relay accounts, host and controller pairing, encryption, view and control access, and recovery.',
  },
  {
    path: '/docs/reference/relay-ci/',
    file: 'docs/ci-bootstrap.md',
    group: REFERENCE,
    label: 'CI setup',
    title: 'CI setup',
    lead: 'Prepare the tools and source dependencies for the complete CLI gate.',
    description: 'ejm-cli CI setup: required tools, the project setup action, pinned source dependencies and shared workflow integration.',
  },
  {
    path: '/docs/reference/contributing/',
    file: 'docs/contributing.md',
    group: REFERENCE,
    label: 'Development reference',
    title: 'Development reference',
    lead: 'Tool pins, module ownership, lint policy, the required gate and docs maintenance.',
    description: 'The ejm-cli development reference: pinned tools, the required quality gate, module ownership, Rust design rules and public docs maintenance.',
  },
  {
    path: '/docs/reference/platforms/',
    file: 'docs/native-remote-publication.md',
    group: REFERENCE,
    label: 'Native verification',
    title: 'Native source verification and release limits',
    lead: 'Accepted native source, Linux and macOS checks, and remaining publication gates.',
    description: 'ejm-cli native source acceptance: exact commits, dependency contracts, checked platforms and remaining Worker, installer and live verification gates.',
  },
  {
    path: '/docs/reference/benchmarks/',
    file: 'docs/benchmarks.md',
    group: REFERENCE,
    label: 'Benchmarks',
    title: 'Performance benchmarks',
    lead: 'Measurements, method and limits.',
    description: 'Reproduce ejm-cli benchmarks and read measured startup, binary size, hardware, workloads and limitations.',
  },
];
