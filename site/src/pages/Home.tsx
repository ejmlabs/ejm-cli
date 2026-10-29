import { Static } from '@ejm/ssg';
import { ShowcaseFrame } from '@ejm/demos/ShowcaseFrame';
import { CodeBlockFrame } from '@ejm/ui/docs/CodeBlockFrame';
import { Band } from '@ejm/ui/layout/Band';
import { Hero } from '@ejm/ui/layout/Hero';
import {
  ContentFlow,
  ReadingColumn,
  SectionBody,
} from '@ejm/ui/layout/SiteContent';
import { LazyCliPreview } from '@ejm/demos/islands/cli-preview';

const sections =
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

export default function Home() {
  return (
    <>
      <Hero
        eyebrow="ejm-cli / open source / MIT"
        title="A Rust terminal coding agent."
        titleId="hero-title"
        actions={
          <Static
            className="contents"
            render={sections ? () => <sections.HeroActions /> : undefined}
          />
        }
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
        <Static
          className="contents"
          render={sections ? () => <sections.HeroIntro /> : undefined}
        />
      </Hero>
      <Static
        className="contents"
        render={sections ? () => <sections.Terminal /> : undefined}
      />
      <Band tone="mist" aria-labelledby="permissions-title">
        <Static
          className="contents"
          render={sections ? () => <sections.PermissionsHead /> : undefined}
        />
        <SectionBody>
          <ReadingColumn>
            <ContentFlow>
              <CodeBlockFrame
                lang="sh"
                label="Add access one flag at a time"
                code={permissions}
              >
                <Static
                  render={
                    CodeHighlight
                      ? () => (
                          <CodeHighlight
                            lang="sh"
                            label="Add access one flag at a time"
                            code={permissions}
                          />
                        )
                      : undefined
                  }
                />
              </CodeBlockFrame>
              <Static
                className="contents"
                render={
                  sections ? () => <sections.PermissionsNote /> : undefined
                }
              />
            </ContentFlow>
          </ReadingColumn>
        </SectionBody>
      </Band>
      <Static
        className="contents"
        render={sections ? () => <sections.RuntimeAndProviders /> : undefined}
      />
      <Band tone="sage" aria-labelledby="get-title">
        <Static
          className="contents"
          render={sections ? () => <sections.SourceHead /> : undefined}
        />
        <SectionBody>
          <ReadingColumn>
            <ContentFlow>
              <CodeBlockFrame
                lang="sh"
                label="Build from a compatible source checkout"
                code={build}
              >
                <Static
                  render={
                    CodeHighlight
                      ? () => (
                          <CodeHighlight
                            lang="sh"
                            label="Build from a compatible source checkout"
                            code={build}
                          />
                        )
                      : undefined
                  }
                />
              </CodeBlockFrame>
              <Static
                className="contents"
                render={sections ? () => <sections.SourceAction /> : undefined}
              />
            </ContentFlow>
          </ReadingColumn>
        </SectionBody>
      </Band>
      <Static
        className="contents"
        render={sections ? () => <sections.More /> : undefined}
      />
    </>
  );
}
