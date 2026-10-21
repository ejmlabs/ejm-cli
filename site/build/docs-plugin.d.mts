import type { Plugin } from 'vite';
export function docsPlugin(options: { root: string }): Plugin;
export function makeLinker(repoRoot: string, file: string): (href: string) => { href: string; external: boolean };
