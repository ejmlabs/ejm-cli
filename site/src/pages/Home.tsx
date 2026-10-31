import { Static } from '@ejm/ssg';
import { ShowcaseFrame } from '@ejm/demos/ShowcaseFrame';
import { CodeBlockFrame } from '@ejm/ui/docs/CodeBlockFrame';
import { Hero } from '@ejm/ui/layout/Hero';
import { Section } from '@ejm/ui/layout/Section';
import { ContentFlow } from '@ejm/ui/layout/SiteContent';
import { LazyCliPreview } from '@ejm/demos/islands/cli-preview';

const copy =
  import.meta.env.SSR || import.meta.env.DEV
    ? await import('./home.server')
    : undefined;
const CodeHighlight =
  import.meta.env.SSR || import.meta.env.DEV
    ? (await import('@ejm/ui/docs/CodeHighlight')).CodeHighlight
    : undefined;

const permissions = `# read only
ejm-cli

# let it edit files
ejm-cli --allow-write

# and run commands
ejm-cli --allow-write --allow-shell`;
const build = `cargo build --release --locked
./target/release/ejm-cli --replay`;

/** Fixed copy from home.server.tsx: in the built page, not in the browser bundle. */
function Copy({ part }: { part: keyof typeof import('./home.server') }) {
  const Part = copy?.[part];
  return (
    <Static
      className="contents"
      render={Part ? () => <Part /> : undefined}
    />
  );
}

function Snippet({ label, code }: { label: string; code: string }) {
  return (
    <CodeBlockFrame lang="sh" label={label} code={code}>
      <Static
        render={
          CodeHighlight
            ? () => <CodeHighlight lang="sh" label={label} code={code} />
            : undefined
        }
      />
    </CodeBlockFrame>
  );
}

export default function Home() {
  return (
    <>
      <Hero
        eyebrow="ejm-cli / open source / MIT"
        title="A Rust terminal coding agent."
        titleId="hero-title"
        actions={<Copy part="HeroActions" />}
        aside={
          <ShowcaseFrame
            id="try"
            label="ejm-cli example"
            note="Recorded local example. No model calls run in the browser."
          >
            <LazyCliPreview />
          </ShowcaseFrame>
        }
      >
        <Copy part="HeroIntro" />
      </Hero>
      <Section
        kicker="01 / Terminal"
        title="Terminal input and output."
        intro={<Copy part="TerminalIntro" />}
      >
        <Copy part="Terminal" />
      </Section>
      <Section
        kicker="02 / Permissions"
        title="File and shell access."
        intro={<Copy part="PermissionsIntro" />}
      >
        <ContentFlow>
          <Snippet label="Add access one flag at a time" code={permissions} />
          <Copy part="PermissionsNote" />
        </ContentFlow>
      </Section>
      <Section
        kicker="03 / Runtime"
        title="Runtime and default limits."
        intro={<Copy part="RuntimeIntro" />}
      >
        <Copy part="Runtime" />
      </Section>
      <Section
        kicker="04 / Providers"
        title="Model providers."
        intro={<Copy part="ProvidersIntro" />}
      >
        <Copy part="Providers" />
      </Section>
      <Section
        emphasis
        kicker="05 / Source"
        title="Build and run locally."
        intro={<Copy part="SourceIntro" />}
        actions={<Copy part="SourceAction" />}
      >
        <ContentFlow>
          <Snippet
            label="Build from a compatible source checkout"
            code={build}
          />
        </ContentFlow>
      </Section>
      <Section kicker="06 / Documentation" title="Documentation.">
        <Copy part="Documentation" />
      </Section>
      <Section
        id="contribute"
        kicker="07 / Contributions"
        title="Source and contributions."
        intro={<Copy part="ContributionsIntro" />}
      >
        <Copy part="Contributions" />
      </Section>
    </>
  );
}
