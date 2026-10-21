import { describe, expect, it } from 'vitest';
import { makeLinker } from '../build/docs-plugin.mjs';

const root = '/repo';
const link = makeLinker(root, '/repo/docs/ejm-cli.md');

describe('link rewriting', () => {
  it('sends links to another rendered reference to its site route, keeping the anchor', () => {
    expect(link('remote-control.md#credentials-and-recovery')).toEqual({
      href: '/docs/reference/remote-control/#credentials-and-recovery',
      external: false,
    });
    expect(
      link('https://github.com/ejmlabs/ejm-cli/blob/main/docs/benchmarks.md'),
    ).toEqual({ href: '/docs/reference/benchmarks/', external: false });
  });
  it('sends every other repository path to GitHub as an external link', () => {
    expect(link('../README.md#quick-start')).toEqual({
      href: 'https://github.com/ejmlabs/ejm-cli/blob/main/README.md#quick-start',
      external: true,
    });
    expect(link('../relay/')).toEqual({
      href: 'https://github.com/ejmlabs/ejm-cli/tree/main/relay/',
      external: true,
    });
    expect(link('https://code.claude.com/docs/en/setup').external).toBe(true);
  });
  it('leaves site paths and same-page anchors alone', () => {
    expect(link('/docs/user-guide/#build')).toEqual({
      href: '/docs/user-guide/#build',
      external: false,
    });
    expect(link('#top')).toEqual({ href: '#top', external: false });
    expect(link('https://cli.ejmlabs.com/docs/')).toEqual({
      href: '/docs/',
      external: false,
    });
  });
});
