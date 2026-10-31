import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { docPages } from '../build/docs-pages.mjs';
import {
  all,
  attr,
  dist,
  first,
  ids,
  ORIGIN,
  pages,
  repo,
  served,
  tag,
  text,
  type Node,
} from './html';

const site = pages();
const real = site.filter((p) => p.route !== '/404');
const byRoute = new Map(site.map((p) => [p.route, p]));

describe('routes', () => {
  it('keeps the URLs the site already had and adds one per documentation page', () => {
    for (const route of [
      '/',
      '/docs/',
      '/docs/user-guide/',
      '/docs/developer-guide/',
    ])
      expect(byRoute.has(route), route).toBe(true);
    for (const p of docPages) expect(byRoute.has(p.path), p.path).toBe(true);
    expect(real.map((p) => p.route).sort()).toEqual(
      ['/', ...docPages.map((p) => p.path)].sort(),
    );
  });

  it('ships the shared files and nothing from the old kit', () => {
    for (const f of [
      '404.html',
      '_headers',
      'theme-boot.js',
      'favicon.svg',
      'favicon.ico',
      'apple-touch-icon.png',
      'site.webmanifest',
      'social-card.png',
    ]) {
      expect(served('/' + f), f).toBeTruthy();
    }
    expect(served('/design-kit/'), 'design-kit').toBeUndefined();
    expect(readdirSync(dist).includes('design-kit')).toBe(false);
  });
});

describe.each(site)('$route', (page) => {
  const { doc } = page;
  const body = first(doc, tag('body'))!;

  it('ends the header with the theme toggle', () => {
    const header = first(doc, tag('header'))!;
    const controls = all(
      header,
      (n) => n.tagName === 'a' || n.tagName === 'button',
    );
    const last = controls[controls.length - 1]!;
    expect(last.tagName).toBe('button');
    expect(`${attr(last, 'aria-label') ?? ''} ${text(last)}`).toMatch(/theme/i);
  });

  it('has a title and a description, and a canonical URL on every page but the 404', () => {
    const title = text(first(doc, tag('title'))!).trim();
    expect(title.length).toBeGreaterThan(3);
    const meta = (name: string) =>
      attr(
        first(doc, (n) => n.tagName === 'meta' && attr(n, 'name') === name)!,
        'content',
      );
    if (page.route !== '/404')
      expect(meta('description')?.length ?? 0).toBeGreaterThan(40);
    const canonical = first(
      doc,
      (n) => n.tagName === 'link' && attr(n, 'rel') === 'canonical',
    );
    if (page.route === '/404') expect(meta('robots')).toContain('noindex');
    else expect(attr(canonical!, 'href')).toBe(ORIGIN + page.route);
  });

  it('loads the theme script from a file and has no inline script or style element', () => {
    const scripts = all(doc, tag('script'));
    expect(scripts.every((s) => attr(s, 'src'))).toBe(true);
    expect(attr(scripts[0]!, 'src')).toBe('/theme-boot.js');
    expect(all(doc, tag('style'))).toHaveLength(0);
  });

  it('keeps the privacy link in the footer and every local link and anchor working', () => {
    const footer = first(doc, tag('footer'))!;
    expect(
      first(
        footer,
        (n) =>
          n.tagName === 'a' &&
          attr(n, 'href') === 'https://ejmlabs.com/privacy/',
      ),
    ).toBeDefined();
    for (const a of all(doc, tag('a'))) {
      const href = attr(a, 'href');
      if (!href || href.startsWith('mailto:')) continue;
      if (/^https?:/.test(href) && !href.startsWith(ORIGIN)) continue;
      const hash = href.includes('#')
        ? decodeURIComponent(href.slice(href.indexOf('#') + 1))
        : '';
      if (href.startsWith('#')) {
        expect(
          hash === '' || ids(doc).has(hash),
          `${page.route} links to ${href}`,
        ).toBe(true);
        continue;
      }
      const path = new URL(href, ORIGIN + page.route).pathname;
      const target = served(path);
      expect(target, `${page.route} links to ${href}`).toBeTruthy();
      if (hash && target!.endsWith('.html')) {
        const dest =
          byRoute.get(path.endsWith('/') || path === '/' ? path : path)?.doc ??
          site.find((p) => p.file === target)?.doc;
        expect(ids(dest!).has(hash), `${page.route} links to ${href}`).toBe(
          true,
        );
      }
    }
  });

  it('uses no studio wording, no we, our or us, no em dash and no first-person plural', () => {
    const prose = text(body, true)
      .replace(/\s+/g, ' ')
      .replace(/Visual Studio/g, '');
    // Report the words around a hit, not the whole page.
    const hit = (re: RegExp) => re.exec(prose)?.index ?? -1;
    for (const re of [
      /\bone[- ]person lab\b/i,
      /\bstudios?\b/i,
      /\b(we|our|ours|us|we're|we'll|we've)\b/i,
    ]) {
      const at = hit(re);
      expect(at < 0 ? '' : prose.slice(Math.max(0, at - 40), at + 40)).toBe('');
    }
    expect(page.html).not.toContain('\u2014');
  });
});

describe('content', () => {
  it('shows only commands and flags that the README, the docs or the argument parser define', () => {
    const corpus = [
      join(repo, 'README.md'),
      join(repo, 'crates/ejm-cli/src/args.rs'),
      ...readdirSync(join(repo, 'docs'))
        .filter((f) => f.endsWith('.md'))
        .map((f) => join(repo, 'docs', f)),
    ]
      .map((f) => readFileSync(f, 'utf8'))
      .join('\n');
    const shown = (n: Node) =>
      all(n, (x) => x.tagName === 'code' || x.tagName === 'pre')
        .map((x) => text(x))
        .join('\n');
    const missing = new Set<string>();
    for (const p of real) {
      // Cargo, npm and git flags belong to those tools; only lines about ejm-cli itself are checked.
      const code = shown(p.doc)
        .split('\n')
        .filter(
          (l) =>
            !/^\s*(cargo (?!run)|npm |npx |rustup |python3? |pip |git |node |cd )/.test(
              l,
            ),
        )
        .join('\n');
      for (const [, flag] of code.matchAll(/(?<![\w-])(--[a-z][a-z0-9-]*)/g))
        // clap derives `--tool-concurrency` from the field `tool_concurrency` in args.rs.
        if (
          !corpus.includes(flag!) &&
          !corpus.includes(flag!.slice(2).replaceAll('-', '_'))
        )
          missing.add(`${p.route} ${flag}`);
      for (const [, cmd] of code.matchAll(
        /\bejm-cli ((?:remote |package )?[a-z][a-z-]*)(?= |\n|$|`)/g,
      )) {
        if (!/^(--|[A-Z])/.test(cmd!) && !corpus.includes(`ejm-cli ${cmd}`))
          missing.add(`${p.route} ejm-cli ${cmd}`);
      }
    }
    expect([...missing]).toEqual([]);
  });

  it('keeps a useful recorded workflow without scripts or editable terminal input', () => {
    const showcase = first(
      byRoute.get('/')!.doc,
      (n) => attr(n, 'id') === 'try',
    )!;
    const description = attr(showcase, 'aria-describedby');
    expect(description).toBeDefined();
    const copy = text(
      first(byRoute.get('/')!.doc, (n) => attr(n, 'id') === description)!,
    ).replace(/\s+/g, ' ');
    expect(copy).toMatch(/recorded local example/i);
    expect(copy).toMatch(/no model calls run in the browser/i);
    const terminal = first(
      showcase,
      (n) => attr(n, 'data-showcase') === 'ejm-cli',
    )!;
    const output = text(terminal).replace(/\s+/g, ' ');
    expect(output).toContain('Patched src/retry.rs');
    expect(output).toContain('3 passed; 0 failed');
    expect(output).toContain('git diff --check is clean');
    expect(
      all(
        showcase,
        (n) =>
          n.tagName === 'input' ||
          n.tagName === 'textarea' ||
          attr(n, 'contenteditable') === 'true',
      ),
    ).toHaveLength(0);
  });

  it('puts the guides and the references on the shared docs layout, with the page title as the only h1', () => {
    for (const p of docPages) {
      const page = byRoute.get(p.path)!;
      expect(text(all(page.doc, tag('h1'))[0]!).trim()).toBe(p.title);
      expect(
        all(
          page.doc,
          (n) => n.tagName === 'nav' && attr(n, 'aria-label') === 'Docs pages',
        ),
      ).toHaveLength(1);
      expect(
        all(
          page.doc,
          (n) =>
            n.tagName === 'nav' && attr(n, 'aria-label') === 'On this page',
        ).length,
      ).toBeGreaterThan(0);
    }
  });
});

it('makes one shared playback the primary hero preview and preserves its watch link', () => {
  const home = byRoute.get('/')!.doc;
  const main = first(home, tag('main'))!;
  const hero = first(main, tag('section'))!;
  expect(first(hero, tag('h1'))).toBeDefined();
  const anchor = first(hero, (n) => attr(n, 'id') === 'try')!;
  expect(anchor).toBeDefined();
  const players = all(home, (n) => attr(n, 'data-showcase') !== undefined);
  expect(players).toHaveLength(1);
  expect(first(anchor, (n) => attr(n, 'data-showcase') === 'ejm-cli')).toBe(
    players[0],
  );
  expect(
    all(players[0]!, tag('button')).map((n) => attr(n, 'aria-label')),
  ).toEqual(['Pause ejm-cli playback', 'Restart ejm-cli playback']);
  expect(
    all(home, (n) => n.tagName === 'a' && attr(n, 'href') === '/#try').some(
      (n) => text(n).trim() === 'Watch',
    ),
  ).toBe(true);
  expect(attr(first(hero, tag('a'))!, 'href')).toBe('#try');
  const setup = first(
    hero,
    (n) => n.tagName === 'a' && attr(n, 'href') === '/docs/user-guide/#build',
  )!;
  expect(setup).toBeDefined();
  expect(
    first(hero, (n) => n.tagName === 'a' && attr(n, 'href') === '#contribute'),
  ).toBeDefined();
  expect(
    all(
      home,
      (n) => n.tagName === 'figure' && attr(n, 'data-shot') !== undefined,
    ),
  ).toHaveLength(0);
  // Logo preloads remain valid; the removed responsive capture must not be preloaded.
  expect(
    all(
      home,
      (n) =>
        n.tagName === 'link' &&
        attr(n, 'as') === 'image' &&
        attr(n, 'imagesrcset') !== undefined,
    ),
  ).toHaveLength(0);
});
