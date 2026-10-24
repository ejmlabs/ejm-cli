import type { ComponentProps, ReactNode } from 'react';
import type { Density } from '../lib/density';
import { cn } from '../lib/utils';

export interface NativeCheckboxProps extends Omit<ComponentProps<'input'>, 'type'> {
  /** Visible label. Without it, give the input an `aria-label`. */
  label?: ReactNode;
  /** `compact`: a 16px box with a 32px target on a fine pointer and 44px on touch. Default: a 24px box and 44px target. */
  density?: Density;
}

/** A native form checkbox that works before hydration. The label supplies the target; the input keeps focus and form behavior. */
export function NativeCheckbox({ className, label, density = 'comfortable', ...props }: NativeCheckboxProps) {
  const compact = density === 'compact';
  return (
    <label className={cn('native-check-label', compact ? 'compact-target text-[0.8125rem]' : 'min-h-11 min-w-11')}>
      <input
        {...props}
        type="checkbox"
        className={cn('native-check', compact ? 'size-4' : 'size-6', className)}
      />
      {label != null && <span className="min-w-0 [overflow-wrap:anywhere]">{label}</span>}
    </label>
  );
}
