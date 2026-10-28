import cliLogoInk from '@ejm/tokens/assets/projects/cli/mark-ink.svg?url&no-inline';
import cliLogoPaper from '@ejm/tokens/assets/projects/cli/mark-paper.svg?url&no-inline';
import { Link, usePath } from '@ejm/ssg';
import { Analytics } from '@ejm/site/analytics';
import { otherSites, PROJECTS, sourceLinks } from '@ejm/site/projects';
import { SiteLayout, type FooterProps, type NavItem } from '@ejm/ui/shell';
import type { ReactNode } from 'react';

const here = PROJECTS.cli;

const identity = {
  name: here.name,
  description: 'Open source Rust terminal coding agent.',
  artwork: { src: cliLogoInk, darkSrc: cliLogoPaper, width: 40, height: 40 },
};

const nav = [
  { label: 'Watch', href: '/#try' },
  { label: 'Docs', href: '/docs/' },
  { label: 'Source', href: '/#contribute' },
] satisfies NavItem[];

const footer: FooterProps = {
  identity,
  columns: [
    {
      heading: 'Documentation',
      links: [
        { label: 'Overview', href: '/docs/' },
        { label: 'User guide', href: '/docs/user-guide/' },
        { label: 'Developer guide', href: '/docs/developer-guide/' },
        { label: 'CLI reference', href: '/docs/reference/cli/' },
        { label: 'Native remote access', href: '/docs/reference/remote-control/' },
        { label: 'Relay privacy notice', href: '/docs/relay-notice/' },
        { label: 'Relay service terms', href: '/docs/relay-service-terms/' },
        {
          label: 'Website privacy',
          href: 'https://ejmlabs.com/privacy/',
          external: true,
        },
      ],
    },
    { heading: 'Sites', links: otherSites('cli') },
    {
      heading: 'Source and contributions',
      links: [
        ...sourceLinks('cli'),
        {
          label: 'Contribute',
          href: `${here.repo}/blob/main/CONTRIBUTING.md`,
          external: true,
        },
      ],
    },
  ],
  privacy:
    'Visits are counted with Cloudflare Web Analytics, which sets no cookies and is off when your browser sends Do Not Track or Global Privacy Control. The only cookie is your theme choice.',
  legal: 'ejm-cli is an open source ejmlabs project under the MIT license.',
};

/**
 * The page frame for every route: skip link, header with the theme toggle last, the page, the footer, and analytics.
 * Shared links load prerendered documents when their route contains static prose.
 */
export function App({ children }: { children: ReactNode }) {
  const path = usePath();
  const items: NavItem[] = nav.map((n) => ({
    ...n,
    current: n.href === '/docs/' && path.startsWith('/docs'),
  }));
  return (
    <>
      <SiteLayout
        header={{
          identity,
          homeLabel: `${here.name} home`,
          nav: items,
        }}
        footer={footer}
        linkComponent={Link}
      >
        {children}
      </SiteLayout>
      <Analytics />
    </>
  );
}
