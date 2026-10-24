import type { HTMLAttributes } from 'react';
import { cn } from '../lib/utils';

/** Readable documents and native forms, without a second typography stylesheet. */
export function DocumentContent({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn('document-content', className)} {...props} />;
}
