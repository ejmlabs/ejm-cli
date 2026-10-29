import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';
import { ejmSite } from '@ejm/site/vite';
import { ejmSsg } from '@ejm/ssg/vite';
import { docsPlugin } from './build/docs-plugin.mjs';

// ejmSite adds the shared head tags and writes theme-boot.js, the icons and _headers. ejmSsg prerenders every route; `npm run perf` checks load time.
// docsPlugin turns the Markdown guides and the repository's docs/*.md into pages at build time.
export default defineConfig({
  root: import.meta.dirname,
  resolve: { dedupe: ['react', 'react-dom'] },
  plugins: [docsPlugin({ root: import.meta.dirname }), tailwindcss(), react(), ejmSite({ project: 'cli' }), ejmSsg({ site: 'src/site.tsx' })],
  build: { target: 'es2022', cssMinify: 'lightningcss' },
});
