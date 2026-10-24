import type { ComponentProps, ReactNode } from 'react';
import { cn } from '../lib/utils';

export type ActionRowsProps = ComponentProps<'ul'>;

/** A compact semantic list for records with separate actions. */
export function ActionRows({ className, children, ...props }: ActionRowsProps) {
  return <ul {...props} className={cn('hairline-t m-0 min-w-0 list-none p-0', className)}>{children}</ul>;
}

export interface ActionRowProps extends Omit<ComponentProps<'li'>, 'title'> {
  title: ReactNode;
  description?: ReactNode;
  /** Short metadata, such as a time or an identifier. */
  meta?: ReactNode;
  /** Separate links, buttons or forms. The row itself is never a stretched link. */
  actions?: ReactNode;
}

/** Text and actions wrap independently. Long identifiers stay readable instead of widening the list. */
export function ActionRow({ title, description, meta, actions, className, children, ...props }: ActionRowProps) {
  return (
    <li {...props} className={cn('hairline flex min-w-0 flex-wrap items-start gap-x-6 gap-y-3 py-3', className)}>
      <div className="min-w-0 flex-[1_1_16rem] [overflow-wrap:anywhere]">
        <div className="font-semibold text-fg">{title}</div>
        {description != null && <div className="mt-1 text-sm text-muted">{description}</div>}
        {meta != null && <div className="text-meta mt-1 text-muted">{meta}</div>}
      </div>
      {actions != null && <div className="flex min-w-0 max-w-full flex-wrap items-center gap-2">{actions}</div>}
      {children != null && <div className="min-w-0 basis-full [overflow-wrap:anywhere]">{children}</div>}
    </li>
  );
}
