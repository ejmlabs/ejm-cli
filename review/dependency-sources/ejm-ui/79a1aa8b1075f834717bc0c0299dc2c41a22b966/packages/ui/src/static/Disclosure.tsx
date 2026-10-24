import type { ComponentProps, ReactNode } from 'react';
import type { Density } from '../lib/density';
import { cn } from '../lib/utils';

export interface DisclosureProps extends ComponentProps<'details'> {
  /** The summary text. Keep controls in the disclosed content. */
  label: ReactNode;
  /** A section stays in the flow; a menu opens below its summary. Both remain collapsible at every width. */
  presentation?: 'section' | 'menu';
  /** Menus default to compact. Sections default to comfortable. Targets stay 44px on touch. */
  density?: Density;
  summaryClassName?: string;
  contentClassName?: string;
}

/** A native disclosure with no script requirement. Unlike Collapse, it never forces content open at a breakpoint. */
export function Disclosure({ label, presentation = 'section', density, className, summaryClassName, contentClassName, children, ...props }: DisclosureProps) {
  const menu = presentation === 'menu';
  const compact = (density ?? (menu ? 'compact' : 'comfortable')) === 'compact';
  return (
    <details {...props} className={cn(menu ? 'relative' : 'hairline hairline-t', className)}>
      <summary
        className={cn(
          'disclosure-summary',
          menu ? 'rounded-control px-3' : 'font-semibold',
          compact ? 'compact-target py-1 text-[0.8125rem]' : 'min-h-11 py-2',
          summaryClassName,
        )}
      >
        {label}
      </summary>
      <div
        className={cn(
          'disclosure-content',
          menu ? 'disclosure-menu' : 'space-y-4 pb-4 pt-2',
          contentClassName,
        )}
      >
        {children}
      </div>
    </details>
  );
}
