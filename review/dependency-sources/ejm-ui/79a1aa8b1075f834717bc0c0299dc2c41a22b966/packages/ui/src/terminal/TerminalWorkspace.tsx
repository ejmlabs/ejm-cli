import type { ComponentProps, ReactNode } from 'react';
import { cn } from '../lib/utils';
import { Select } from '../components/ui/select';
import { Textarea } from '../components/ui/textarea';

export type TerminalWorkspaceProps = Omit<ComponentProps<'main'>, 'style'>;

/** A full-height terminal surface. A visible pairing section replaces the log and composer. */
export function TerminalWorkspace({ className, children, ...props }: TerminalWorkspaceProps) {
  return (
    <main
      {...props}
      data-terminal-workspace=""
      className={cn('terminal-workspace', className)}
    >
      {children}
    </main>
  );
}

export interface TerminalHeaderProps extends Omit<ComponentProps<'header'>, 'title' | 'children' | 'style'> {
  homeHref: string;
  homeLabel: string;
  /** Brand artwork and text. */
  brand: ReactNode;
  title: ReactNode;
  navigation?: ReactNode;
  context?: ReactNode;
  contextActions?: ReactNode;
}

/** Two quiet lines: brand, title and navigation, then context and optional controls. */
export function TerminalHeader({ homeHref, homeLabel, brand, title, navigation, context, contextActions, className, ...props }: TerminalHeaderProps) {
  return (
    <header {...props} className={cn('terminal-header', className)}>
      <div className="terminal-header-line">
        <a href={homeHref} aria-label={homeLabel} className="terminal-brand">{brand}</a>
        <h1 className="terminal-title">{title}</h1>
        {navigation && <div className="terminal-nav">{navigation}</div>}
      </div>
      {(context || contextActions) && (
        <div className="terminal-context-line">
          <div className="terminal-context">{context}</div>
          {contextActions && <div className="ms-auto shrink-0">{contextActions}</div>}
        </div>
      )}
    </header>
  );
}

export interface TerminalLinkProps extends Omit<ComponentProps<'a'>, 'style'> {
  /** Keep secondary navigation out of the narrow header. */
  hideOnSmall?: boolean;
}

/** A compact native navigation link with the shared focus and touch targets. */
export function TerminalLink({ hideOnSmall = false, className, children, ...props }: TerminalLinkProps) {
  return <a {...props} className={cn('terminal-link', hideOnSmall ? 'hidden sm:inline-flex' : 'inline-flex', className)}>{children}</a>;
}

export type TerminalOutputProps = Omit<ComponentProps<'section'>, 'style'>;

/** Space shared by the initial pairing view and the scrollable transcript. */
export function TerminalOutput({ className, children, ...props }: TerminalOutputProps) {
  return <section {...props} className={cn('terminal-output', className)}>{children}</section>;
}

export type TerminalPairingProps = Omit<ComponentProps<'section'>, 'style'>;

/** Introductory connection content. Set hidden after the host has been paired. */
export function TerminalPairing({ className, children, ...props }: TerminalPairingProps) {
  return <section {...props} data-terminal-pairing="" className={cn('terminal-pairing', className)}>{children}</section>;
}

export type TerminalLogProps = Omit<ComponentProps<'div'>, 'role' | 'style'>;

/** Keyboard-scrollable output. Live announcements are off so streamed text does not interrupt input. */
export function TerminalLog({ className, children, tabIndex = 0, 'aria-live': live = 'off', ...props }: TerminalLogProps) {
  return <div {...props} role="log" aria-live={live} tabIndex={tabIndex} data-terminal-log="" className={cn('terminal-log', className)}>{children}</div>;
}

export type TerminalEntryKind = 'prompt' | 'assistant' | 'output' | 'error';

export interface TerminalEntryProps extends Omit<ComponentProps<'div'>, 'children' | 'style'> {
  label: string;
  /** Presentation follows the kind, never the translated label. */
  kind?: TerminalEntryKind;
  /** Plain text only. For native DOM updates, fill the label and text hooks with textContent. */
  children: string;
}

/** A transcript entry. An empty entry can be rendered in a template and cloned without React at runtime. */
export function TerminalEntry({ label, kind = 'output', className, children, ...props }: TerminalEntryProps) {
  return (
    <div {...props} data-terminal-entry="" data-kind={kind} className={cn('terminal-entry', className)}>
      <div data-terminal-label="" className="terminal-entry-label">{label}</div>
      <pre data-terminal-text="" className="terminal-entry-text">{children}</pre>
    </div>
  );
}

export interface TerminalStatusProps extends Omit<ComponentProps<'div'>, 'children' | 'style'> {
  connection: ReactNode;
  /** Supply a role=status node when these words change during a session. */
  children: ReactNode;
}

/** Connection and status share a line. State words remain visible when colors are unavailable. */
export function TerminalStatus({ connection, className, children, ...props }: TerminalStatusProps) {
  return (
    <div {...props} className={cn('terminal-status', className)}>
      <span className="shrink-0">{connection}</span>
      <span aria-hidden="true">·</span>
      <div className="min-w-0 break-words">{children}</div>
    </div>
  );
}

export interface TerminalComposerProps extends Omit<ComponentProps<'section'>, 'children' | 'style'> {
  legend: string;
  /** A label linked to the input, with the prompt glyph and an accessible name. */
  prompt: ReactNode;
  input: ReactNode;
  actions: ReactNode;
  hints?: ReactNode;
  /** Outcome or recovery content, after the form and before the hints. */
  children?: ReactNode;
  formProps?: Omit<ComponentProps<'form'>, 'children' | 'style'>;
}

/** One native form, with a fieldset around the prompt and actions. */
export function TerminalComposer({ legend, prompt, input, actions, hints, children, formProps, className, ...props }: TerminalComposerProps) {
  return (
    <section {...props} data-terminal-composer="" className={cn('terminal-composer', className)}>
      <form {...formProps}>
        <fieldset className="terminal-input">
          <legend className="terminal-legend">{legend}</legend>
          <div className="terminal-input-body">
            <div className="terminal-prompt">{prompt}</div>
            {input}
          </div>
          <div className="terminal-actions">{actions}</div>
        </fieldset>
      </form>
      {children}
      {hints && <div className="terminal-hints">{hints}</div>}
    </section>
  );
}

export type TerminalTextareaProps = Omit<ComponentProps<typeof Textarea>, 'style'>;

/** The public multiline field with terminal spacing, within the composer's control outline. */
export function TerminalTextarea({ className, ...props }: TerminalTextareaProps) {
  return <Textarea {...props} className={cn('min-h-16 min-w-0 max-h-[24svh] flex-1 rounded-none border-0 bg-transparent px-0 py-1 text-base', className)} />;
}

export type TerminalSelectProps = Omit<ComponentProps<typeof Select>, 'style'>;

/** The public native picker with terminal spacing. It keeps a 44px touch target. */
export function TerminalSelect({ className, ...props }: TerminalSelectProps) {
  return <Select {...props} className={cn('min-h-0 border-0 bg-transparent ps-0 text-base compact-target sm:text-xs', className)} />;
}
