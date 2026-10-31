import { defineConfig } from 'vite';
import { ejmlabs } from '@ejm/site/vite';
import { docsPlugin } from './build/docs-plugin.mjs';

// ejmlabs() is the shared setup: Tailwind, React, the design guard, the shared head and headers, prerendering and the build options.
// docsPlugin turns the Markdown guides and the repository's docs/*.md into pages at build time.
export default defineConfig({
  root: import.meta.dirname,
  plugins: [ejmlabs({ project: 'cli' }), docsPlugin({ root: import.meta.dirname })],
});
