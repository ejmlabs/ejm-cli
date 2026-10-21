// Server-only page copy. Shared Static keeps it out of the browser bundle.
import { PROJECTS, otherSites } from '@ejm/site/projects';
import { Callout } from '@ejm/ui/docs';
import {
  Band,
  Fact,
  Facts,
  Row,
  Rows,
  SectionHead,
  Stat,
} from '@ejm/ui/layout';
import { Button } from '@ejm/ui/components/ui/button';
import {
  Actions,
  Columns,
  ContentGroup,
  ContentHeading,
  InlineCode,
  MetricGrid,
  Note,
  ReadingColumn,
  SectionBody,
} from '@ejm/ui/layout/SiteContent';

const repo = PROJECTS.cli.repo;

// Project descriptions stay concrete and separate from registry taglines.
const about: Record<string, string> = {
  ejmlabs:
    'The open source projects, documentation and repositories at ejmlabs.',
  'ejm-ide': 'An open source desktop code editor with an agent interface.',
  'ejm-harness':
    'The open source Rust engine used by ejm-cli, with shared session scheduling and resource limits.',
};

export function HeroIntro() {
  return (
    <p>
      A Rust terminal app built on ejm-harness, open source under MIT.
    </p>
  );
}

export function HeroActions() {
  return (
    <>
      <Button asChild arrow>
        <a href="#try">Watch example</a>
      </Button>
      <Button asChild variant="link">
        <a href="/docs/user-guide/#build">Build from source</a>
      </Button>
      <Button asChild variant="link">
        <a href="#contribute">Source and contributions</a>
      </Button>
    </>
  );
}

export function Terminal() {
  return (
    <Band aria-labelledby="terminal-title">
      <SectionHead
        kicker="01 / Terminal"
        title="Terminal input and output."
        id="terminal-title"
      >
        <p>
          The screen follows your terminal colors and never paints a background.
          Configure a provider before launch, or try the offline replay.
        </p>
      </SectionHead>
      <SectionBody>
        <Columns
          primary={
            <ContentGroup>
              <ContentHeading as="h3">Main keys</ContentHeading>
              <Facts>
                <Fact label="Enter">
                  Send a turn, or queue it while a run is active.
                </Fact>
                <Fact label="Alt+Enter">Insert a newline.</Fact>
                <Fact label="Ctrl+K">
                  Steer the active run with your draft.
                </Fact>
                <Fact label="Ctrl+S">Choose a saved session.</Fact>
                <Fact label="Ctrl+C">
                  Stop the current run, or exit when idle.
                </Fact>
              </Facts>
              <Note>
                Type <code>/help</code> in the app for the full list of keys and
                commands. Use <code>--no-color</code> or <code>NO_COLOR</code> for
                monochrome, and <code>--plain</code> for ASCII symbols.
              </Note>
            </ContentGroup>
          }
          secondary={
            <Rows aria-label="Terminal features">
              <Row
                title="Composer"
                description="Unicode, multiline input, bracketed paste and sent-message history. Paste a stack trace without sending it by accident."
              />
              <Row
                title="Streaming output"
                description="Code, Markdown and diffs are styled as they arrive. Control sequences in model and tool output are removed from the display."
              />
              <Row
                title="Run controls"
                description="Queue the next turn, redirect the current one, or stop it. Run state stays visible."
              />
              <Row
                title="Saved sessions"
                description="Pick up a session by ID or unique prefix. Durable history lives in the harness, not in your scrollback."
              />
              <Row
                title="Pipes and scripts"
                description={
                  <>
                    <code>ejm-cli run</code> streams plain text, or one JSON
                    event per line with <InlineCode>--json</InlineCode>. Failed
                    or cancelled runs exit nonzero.
                  </>
                }
              />
              <Row
                title="Terminal restoration"
                description="Raw mode, mouse tracking, bracketed paste and the alternate screen are restored after errors, handled signals and panics."
              />
            </Rows>
          }
        />
      </SectionBody>
    </Band>
  );
}

export function RuntimeAndProviders() {
  return (
    <>
      <Band aria-labelledby="footprint-title">
        <SectionHead
          kicker="03 / Runtime"
          title="Runtime and default limits."
          id="footprint-title"
        >
          <p>
            No Node or Python runtime for the core binary. Agents share one
            resource governor instead of opening a shell each, and the visible
            transcript is capped at 256 KiB.
          </p>
        </SectionHead>
        <SectionBody>
          <MetricGrid>
            <Stat figure="100" label="managed sessions by default" />
            <Stat figure="8" label="model requests at once by default" />
            <Stat figure="4" label="local jobs by default" />
            <Stat figure="1" label="heavy job by default" />
          </MetricGrid>
        </SectionBody>
        <SectionBody>
          <ReadingColumn>
            <Callout kind="note">
              In an offline benchmark on 18 October 2026, a Mac16,8 painted the
              first composer in 24.197&nbsp;ms with fresh state and
              41.312&nbsp;ms at the warm p95 across 20 samples. Results depend
              on hardware and workload. See{' '}
              <a href="/docs/reference/benchmarks/">
                benchmark methods and results
              </a>
              .
            </Callout>
          </ReadingColumn>
        </SectionBody>
      </Band>
      <Band tone="mist" aria-labelledby="models-title">
        <SectionHead
          kicker="04 / Providers"
          title="Model providers."
          id="models-title"
        >
          <p>
            Choose a provider with <InlineCode>--provider</InlineCode>. Use API
            credentials or ChatGPT sign-in. Your provider's terms and charges apply.
          </p>
        </SectionHead>
        <SectionBody>
          <Rows aria-label="Providers">
            <Row
              title="OpenAI compatible"
              meta="--provider openai"
              href="/docs/user-guide/#compatible"
              description="Chat Completions-compatible APIs. Provide the key through EJM_CLI_API_KEY and set the base URL and model."
            />
            <Row
              title="OpenAI Responses"
              meta="--provider responses"
              href="/docs/user-guide/#native"
              description="Native Responses access with your API key."
            />
            <Row
              title="ChatGPT"
              meta="--provider chatgpt"
              href="/docs/user-guide/#chatgpt"
              description="ejm-cli login chatgpt signs in through your browser. Tokens never touch arguments, logs or the screen."
            />
            <Row
              title="Anthropic"
              meta="--provider anthropic"
              href="/docs/user-guide/#native"
              description="Native Anthropic API access with an explicit model."
            />
          </Rows>
        </SectionBody>
      </Band>
    </>
  );
}

export function More() {
  return (
    <>
      <Band aria-labelledby="docs-title">
        <SectionHead
          kicker="06 / Documentation"
          title="Documentation."
          id="docs-title"
        />
        <SectionBody>
          <Rows aria-label="Documentation">
            <Row
              title="User guide"
              href="/docs/user-guide/"
              description="Build the CLI, connect your model, choose grants, save sessions, and use scripts."
              meta="Guide"
            />
            <Row
              title="Developer guide"
              href="/docs/developer-guide/"
              description="Set up sibling checkouts, find module ownership, and run the CLI and site checks."
              meta="Guide"
            />
            <Row
              title="CLI reference"
              href="/docs/reference/cli/"
              description="Arguments, provider accounts, interaction, packages, sessions and rendering bounds."
              meta="Reference"
            />
            <Row
              title="Native remote access"
              href="/docs/reference/remote-control/"
              description="Relay accounts, endpoint pairing, grants, encrypted transport and recovery limits."
              meta="Reference"
            />
          </Rows>
        </SectionBody>
      </Band>
      <Band id="contribute" aria-labelledby="family-title">
        <SectionHead
          kicker="07 / Contributions"
          title="Source and contributions."
          id="family-title"
        >
          <p>
            Read the code, report a reproducible issue, or propose a change.
          </p>
        </SectionHead>
        <SectionBody>
          <Rows aria-label="Source, contributions and related projects">
            <Row
              title="ejm-cli on GitHub"
              href={repo}
              external
              description="Source code, tests and development history."
              meta="MIT"
            />
            <Row
              title="Issue tracker"
              href={`${repo}/issues`}
              external
              description="Report a problem with steps to reproduce it, or discuss a proposed change."
            />
            <Row
              title="Contribution guide"
              href={`${repo}/blob/main/CONTRIBUTING.md`}
              external
              description="Project conventions, review policy and required checks."
            />
            {otherSites('cli').map((s) => (
              <Row
                key={s.href}
                title={s.label}
                href={s.href}
                external
                description={about[s.label]}
                meta={new URL(s.href).host}
              />
            ))}
          </Rows>
        </SectionBody>
      </Band>
    </>
  );
}

export function PermissionsHead() {
  return (
    <SectionHead
      kicker="02 / Permissions"
      title="File and shell access."
      id="permissions-title"
    >
      <p>
        File tools start read only. Writing and shell access are flags you pass
        on purpose, and the grants stay visible in the header for the life of
        the process.
      </p>
    </SectionHead>
  );
}

export function PermissionsNote() {
  return (
    <Callout kind="warning">
      Native execution runs as you and is not a sandbox.{' '}
      <code>ejm-cli doctor</code> reports what the executor actually enforces.
    </Callout>
  );
}

export function SourceHead() {
  return (
    <SectionHead
      kicker="05 / Source"
      title="Build and run locally."
      id="get-title"
    >
      <p>
        Build the public source with Rust. There is no supported binary
        installer or package-manager distribution.
      </p>
    </SectionHead>
  );
}

export function SourceAction() {
  return (
    <Actions>
      <Button asChild arrow>
        <a href="/docs/user-guide/#build">Read setup</a>
      </Button>
    </Actions>
  );
}
