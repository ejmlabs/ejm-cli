import type { ReactNode } from 'react';
import { cn } from '../lib/utils';
import { Collapse } from './Collapse';
import { LinkProvider, useLinkComponent, type LinkComponent } from './link';
import { SkipLink } from './SkipLink';
import { ThemeToggle } from './ThemeToggle';

export interface AppShellProps {
  /** The product mark or name, in the top bar. */
  brand: ReactNode;
  homeHref?: string;
  homeLabel?: string;
  /** Controls that sit in the top bar before the theme toggle (a search field, an account menu). */
  actions?: ReactNode;
  /** Replaces the last theme control. Omit to use ThemeToggle; pass null to show none. */
  themeControl?: ReactNode;
  /** The side navigation list. Below 768px it becomes a labelled disclosure above the content. */
  nav: ReactNode;
  navLabel?: string;
  linkComponent?: LinkComponent;
  className?: string;
  children: ReactNode;
}

/**
 * The frame for an application: a top bar with the theme toggle last, a side navigation and the content. No boxes: the bar and
 * the nav are the page ground, separated by one hairline each (one under the bar from 768px, one beside the nav). The page scrolls as a whole (the bar and nav stay put), which
 * keeps find, zoom and print working. Collapses to a single column below 768px.
 */
export function AppShell({ brand, homeHref = '/', homeLabel = 'Home', actions, themeControl, nav, navLabel = 'Navigation', linkComponent, className, children }: AppShellProps) {
  const ambient = useLinkComponent();
  const Link = linkComponent ?? ambient;
  return (
    <LinkProvider value={Link}>
      <div className={cn('flex min-h-svh flex-col', className)}>
        <SkipLink />
        <header className="band sticky top-0 z-40 pt-[env(safe-area-inset-top)] md:hairline">
          <div className="flex h-14 items-center justify-between gap-4 px-(--ejm-page-gutter)">
            <Link href={homeHref} aria-label={homeLabel} className="touch-target -ms-2 inline-flex items-center rounded-control px-2 py-2 text-fg">
              {brand}
            </Link>
            <div className="flex items-center gap-2">
              {actions}
              {themeControl === undefined ? <ThemeToggle /> : themeControl}
            </div>
          </div>
        </header>
        <div className="grid flex-1 md:grid-cols-[15rem_minmax(0,1fr)]">
          <div className="md:border-e md:border-line">
            <Collapse
              from="md"
              label={navLabel}
              className="px-(--ejm-page-gutter) md:sticky md:top-[calc(3.5rem+env(safe-area-inset-top,0px))] md:max-h-[calc(100svh-3.5rem-env(safe-area-inset-top,0px))] md:overflow-y-auto md:overscroll-contain md:py-6 md:ps-[max(var(--ejm-page-gutter),env(safe-area-inset-left,0px))] md:pe-4"
            >
              <nav aria-label={navLabel} className="pb-4 md:pb-0">
                {nav}
              </nav>
            </Collapse>
          </div>
          <main id="main" tabIndex={-1} className="min-w-0 px-(--ejm-page-gutter) py-8 focus:outline-none md:px-8 lg:px-12">
            {children}
          </main>
        </div>
      </div>
    </LinkProvider>
  );
}
