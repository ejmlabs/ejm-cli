import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactNode } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { expectAccessible, renderIn, themes } from '../components/test-setup';
import { Button } from '../components/ui/button';
import { NativeCheckbox } from '../static/NativeCheckbox';
import { ActionRow, ActionRows } from '../static/ActionRows';
import { AppShell } from './AppShell';
import { Disclosure } from '../static/Disclosure';
import { Header } from './Header';

beforeEach(() => {
  vi.stubGlobal('matchMedia', () => ({ matches: false, addEventListener() {}, removeEventListener() {} }));
});
afterEach(() => vi.unstubAllGlobals());

// Only the wrapper mounts in React. The shared markup inside it is never hydrated.
function renderStatic(children: ReactNode) {
  return render(<div dangerouslySetInnerHTML={{ __html: renderToStaticMarkup(children) }} />);
}

describe('native static controls', () => {
  it('activates the label and preserves required, named submission and reset before hydration', async () => {
    const user = userEvent.setup();
    const { container } = renderStatic(
      <>
        <form id="consent" />
        <NativeCheckbox label="Accept terms" id="terms" name="terms" value="accepted" required form="consent" density="compact" />
        <NativeCheckbox label="Disabled choice" name="excluded" defaultChecked disabled form="consent" />
      </>,
    );
    const form = container.querySelector('form')!;
    const checkbox = screen.getByRole('checkbox', { name: 'Accept terms' });
    expect(form.checkValidity()).toBe(false);
    expect(new FormData(form).has('terms')).toBe(false);
    await user.click(screen.getByText('Accept terms'));
    expect(checkbox).toBeChecked();
    expect(form.checkValidity()).toBe(true);
    expect([...new FormData(form).entries()]).toEqual([['terms', 'accepted']]);
    expect(screen.getByRole('checkbox', { name: 'Disabled choice' })).toBeChecked();
    form.reset();
    expect(checkbox).not.toBeChecked();
    expect(form.checkValidity()).toBe(false);
  });

  it('opens and closes both disclosure presentations without hydration or inline code', async () => {
    const user = userEvent.setup();
    const { container } = renderStatic(
      <>
        <Disclosure label="Advanced"><p>Advanced settings</p></Disclosure>
        <Disclosure label="Account" presentation="menu"><a href="/account">Manage account</a></Disclosure>
      </>,
    );
    for (const details of container.querySelectorAll('details')) {
      const summary = details.querySelector('summary')!;
      expect(details.open).toBe(false);
      await user.click(summary);
      expect(details.open).toBe(true);
      await user.click(summary);
      expect(details.open).toBe(false);
    }
    expect(container.querySelector('script, [style], [onclick]')).toBeNull();
  });
});

describe.each([
  ['Header', (themeControl?: ReactNode) => <Header logo={<span>Product</span>} themeControl={themeControl} />],
  ['AppShell', (themeControl?: ReactNode) => <AppShell brand={<span>Product</span>} nav={<a href="/sessions">Sessions</a>} themeControl={themeControl}><h1>Workspace</h1></AppShell>],
] as const)('%s theme control slot', (_name, frame) => {
  it('keeps the default for undefined, uses a replacement and suppresses null', () => {
    const defaultName = /Switch to (?:light|dark) theme/;
    const { rerender } = render(frame());
    expect(screen.getByRole('button', { name: defaultName })).toBeInTheDocument();
    rerender(frame(<button type="button">Choose theme</button>));
    expect(screen.queryByRole('button', { name: defaultName })).toBeNull();
    expect(screen.getByRole('button', { name: 'Choose theme' })).toBeInTheDocument();
    rerender(frame(null));
    expect(screen.queryByRole('button', { name: defaultName })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Choose theme' })).toBeNull();
    rerender(frame(undefined));
    expect(screen.getByRole('button', { name: defaultName })).toBeInTheDocument();
  });
});

describe.each(themes)('static controls accessibility in %s', theme => {
  it('keeps shared controls and row actions accessible', async () => {
    const { container } = renderIn(
      theme,
      <>
        <NativeCheckbox label="Allow access" density="compact" />
        <Disclosure label="More options"><p>Connection settings</p></Disclosure>
        <Disclosure label="Account actions" presentation="menu" open>
          <Button density="compact" variant="ghost">Sign out</Button>
        </Disclosure>
        <ActionRows aria-label="Active sessions">
          <ActionRow title={<a href="/sessions/one">Local session</a>} meta="Connected" actions={<Button density="compact">Disconnect</Button>} />
        </ActionRows>
      </>,
    );
    expect(screen.getByRole('button', { name: 'Disconnect' }).closest('a')).toBeNull();
    await expectAccessible(container);
  });
});
