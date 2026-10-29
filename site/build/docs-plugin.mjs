// Turns a Markdown source into a React page module. `import('../content/user-guide.md?doc')` resolves to a default export that is the
// finished page. Shared packages compile and render Markdown; this adapter owns source paths, route metadata and link policy.
import { readFile } from 'node:fs/promises';
import { dirname, posix, relative, resolve, sep } from 'node:path';
import { PROJECTS } from '@ejm/site/projects';
import { docPages } from './docs-pages.mjs';
import { parseDoc, splitDoc } from '@ejm/site/docs';

const REPO = PROJECTS.cli.repo;
const ORIGIN = PROJECTS.cli.url.replace(/\/$/, '');

/** Rewrites a link written for GitHub or for the repository tree into a site route or a repository URL. */
export function makeLinker(repoRoot, file) {
  const routes = new Map(docPages.map((p) => [p.file, p.path]));
  const toRoute = (repoPath, hash) => {
    const route = routes.get(repoPath);
    return route ? { href: route + hash, external: false } : null;
  };
  return (href) => {
    const [target, ...frag] = href.split('#');
    const hash = frag.length ? `#${frag.join('#')}` : '';
    if (href.startsWith('#')) return { href, external: false };
    if (href.startsWith('/')) return { href, external: false };
    if (/^https?:/i.test(href)) {
      if (href.startsWith(ORIGIN + '/'))
        return { href: href.slice(ORIGIN.length), external: false };
      const blob = new RegExp(
        `^${REPO.replace(/[.]/g, '\\.')}/blob/main/(.+)$`,
      ).exec(target);
      return (blob && toRoute(blob[1], hash)) || { href, external: true };
    }
    if (/^[a-z][a-z0-9+.-]*:/i.test(href)) return { href, external: true }; // mailto: and the like
    const repoPath = posix.normalize(
      posix.join(
        dirname(relative(repoRoot, file).split(sep).join('/')),
        target,
      ),
    );
    return (
      toRoute(repoPath, hash) ?? {
        href: `${REPO}/${target.endsWith('/') ? 'tree' : 'blob'}/main/${repoPath}${hash}`,
        external: true,
      }
    );
  };
}

/** @returns {import('vite').Plugin} */
export function docsPlugin({ root }) {
  const repoRoot = resolve(root, '..');
  const SUFFIX = '?doc';
  const ROUTES = 'virtual:doc-routes';
  const NAVIGATION = 'virtual:doc-navigation';
  return {
    name: 'ejm-cli-docs',
    enforce: 'pre',
    async resolveId(source, importer) {
      if (source === ROUTES || source === NAVIGATION) return '\0' + source;
      if (!source.endsWith('.md' + SUFFIX)) return null;
      const base = importer ? dirname(importer) : root;
      return resolve(source.startsWith('.') ? base : root, source);
    },
    async load(id) {
      if (id === '\0' + NAVIGATION) {
        const pages = docPages.map(({ path: href, title, label, group }) => ({
          href,
          title,
          label,
          group,
        }));
        return `export default ${JSON.stringify(pages)};`;
      }
      if (id === '\0' + ROUTES) {
        // One literal import() per page: ssg finds a lazy route's chunk by reading the import() call in `load`.
        const routes = docPages.map(
          (p) =>
            `{ path: ${JSON.stringify(p.path)}, load: () => import(${JSON.stringify(resolve(repoRoot, p.file) + SUFFIX)}), head: ${JSON.stringify({ title: p.title, description: p.description })} }`,
        );
        return `export const docRoutes = [\n  ${routes.join(',\n  ')},\n];\n`;
      }
      if (!id.endsWith('.md' + SUFFIX)) return null;
      const file = id.slice(0, -SUFFIX.length);
      this.addWatchFile(file);
      const entry = docPages.find((p) => resolve(repoRoot, p.file) === file);
      if (!entry)
        throw new Error(`docs: ${file} is not in build/docs-pages.mjs`);
      // Canonical references live outside site/, so resolve this public export from the site's dependency boundary.
      const proseModule = await this.resolve(
        '@ejm/ui/docs/DocProse',
        resolve(root, 'src/site.tsx'),
      );
      if (!proseModule)
        throw new Error('docs: cannot resolve @ejm/ui/docs/DocProse');
      const codeModule = await this.resolve(
        '@ejm/ui/docs/CodeHighlight',
        resolve(root, 'src/site.tsx'),
      );
      if (!codeModule)
        throw new Error('docs: cannot resolve @ejm/ui/docs/CodeHighlight');
      const { full, client, requiresRich } = splitDoc(
        parseDoc(await readFile(file, 'utf8'), {
          link: makeLinker(repoRoot, file),
          metadata: {
            slug: entry.path,
            title: entry.title,
            lead: entry.lead,
            group: entry.group,
            description: entry.description,
          },
        }),
      );
      const richModule = requiresRich
        ? await this.resolve(
            '@ejm/ui/docs/DocDataBlock',
            resolve(root, 'src/site.tsx'),
          )
        : null;
      if (requiresRich && !richModule)
        throw new Error('docs: cannot resolve @ejm/ui/docs/DocDataBlock');
      return [
        `import { createElement } from 'react';`,
        ...(richModule
          ? [
              `import { DocDataBlock } from ${JSON.stringify(richModule.id)};`,
              `const renderBlock = (block, renderCode) => createElement(DocDataBlock, { block, renderCode });`,
            ]
          : []),
        `import { DocPageView } from '/src/docs/DocPageView.tsx';`,
        `const page = ${JSON.stringify(client)};`,
        `let renderProse, renderCode;`,
        `if (import.meta.env.SSR || import.meta.env.DEV) {`,
        `  const { DocProse } = await import(${JSON.stringify(proseModule.id)});`,
        `  const { CodeHighlight } = await import(${JSON.stringify(codeModule.id)});`,
        `  const full = ${JSON.stringify(full)};`,
        `  renderProse = (si, ii) => createElement(DocProse, { section: full.sections[si], blocks: full.sections[si].items[ii].blocks, first: ii === 0 });`,
        `  renderCode = (block) => createElement(CodeHighlight, { code: block.code, lang: block.lang, label: block.label });`,
        `}`,
        `export default function DocPage() { return createElement(DocPageView, { page, renderProse, renderCode${requiresRich ? ', renderBlock' : ''} }); }`,
        '',
      ].join('\n');
    },
  };
}
