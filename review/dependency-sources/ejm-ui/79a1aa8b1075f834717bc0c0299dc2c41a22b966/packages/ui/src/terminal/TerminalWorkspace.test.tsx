import { within } from '@testing-library/react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { expectAccessible } from '../components/test-setup';
import { Button } from '../components/ui/button';
import {
  TerminalComposer, TerminalEntry, TerminalHeader, TerminalLink, TerminalLog, TerminalOutput,
  TerminalPairing, TerminalSelect, TerminalStatus, TerminalTextarea, TerminalWorkspace,
} from './TerminalWorkspace';

describe('TerminalWorkspace', () => {
  it('keeps the static native control and plain-text transcript contract without hydration', async () => {
    const untrusted = '<img src=x onerror=alert(1)>\n<script>alert(2)</script>';
    const html = renderToStaticMarkup(
      <TerminalWorkspace aria-label="Remote terminal">
        <TerminalHeader homeHref="/account" homeLabel="Product account" brand="product" title="Shared session" navigation={<TerminalLink href="/help">Help</TerminalLink>} context="Read access" />
        <TerminalOutput aria-label="Terminal output">
          <TerminalPairing hidden aria-label="Connect a host">Connection instructions</TerminalPairing>
          <TerminalLog aria-label="Session output"><TerminalEntry label="Host" kind="assistant">{untrusted}</TerminalEntry></TerminalLog>
        </TerminalOutput>
        <TerminalStatus connection="Connected"><p role="status">Read access</p></TerminalStatus>
        <TerminalComposer
          aria-label="Send instruction"
          legend="Message"
          formProps={{ id: 'instruction-form', method: 'post' }}
          prompt={<label htmlFor="instruction"><span aria-hidden="true">›</span><span className="sr-only">Instruction</span></label>}
          input={<TerminalTextarea id="instruction" name="instruction" disabled />}
          actions={<><label>Action<TerminalSelect name="action" defaultValue="submit" disabled><option value="submit">Submit</option></TerminalSelect></label><Button type="submit" disabled>Send</Button></>}
          hints="Enter adds a line"
        />
        <template id="entry-template"><TerminalEntry label="">{''}</TerminalEntry></template>
      </TerminalWorkspace>,
    );
    const container = document.createElement('div');
    container.innerHTML = html;
    document.body.append(container);
    try {
      const view = within(container);
      expect(view.getByRole('main', { name: 'Remote terminal' })).toBeInTheDocument();
      expect(view.getByRole('link', { name: 'Product account' })).toHaveAttribute('href', '/account');
      expect(view.getByRole('heading', { level: 1, name: 'Shared session' })).toBeInTheDocument();
      expect(view.getByRole('log', { name: 'Session output' })).toHaveAttribute('tabindex', '0');
      expect(view.getByRole('log')).toHaveAttribute('aria-live', 'off');
      expect(view.getByRole('group', { name: 'Message' }).closest('form')).toHaveAttribute('id', 'instruction-form');
      expect(view.getByRole('textbox', { name: 'Instruction' })).toBeDisabled();
      expect(view.getByRole('combobox', { name: 'Action' })).toBeDisabled();
      expect(view.getByRole('button', { name: 'Send' })).toBeDisabled();

      const entry = container.querySelector('[data-terminal-entry]')!;
      expect(entry.querySelector('[data-terminal-label]')).toHaveTextContent('Host');
      expect(entry.querySelector('[data-terminal-text]')?.textContent).toBe(untrusted);
      expect(entry.querySelector('img, script')).toBeNull();
      const template = container.querySelector<HTMLTemplateElement>('#entry-template')!;
      const clone = template.content.firstElementChild!.cloneNode(true) as HTMLElement;
      clone.querySelector('[data-terminal-label]')!.textContent = 'Tool error';
      clone.querySelector('[data-terminal-text]')!.textContent = untrusted;
      clone.dataset.kind = 'error';
      expect(clone.querySelector('[data-terminal-text]')?.textContent).toBe(untrusted);
      expect(clone.querySelector('img, script')).toBeNull();
      expect(container.querySelector('[style], script')).toBeNull();
      await expectAccessible(container);
    } finally {
      container.remove();
    }
  });
});
