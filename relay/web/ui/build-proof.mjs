// Build-only proof. Production Worker and controller entries do not import this file.
import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, readdir, rename, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { gzipSync } from "node:zlib";
import tailwind from "@tailwindcss/vite";
import { build } from "vite";

if (process.argv.length !== 2) throw new Error("The UI proof takes no arguments");
const source = path.dirname(fileURLToPath(import.meta.url));
const output = path.resolve(source, "../.ui-proof");

await build({
  configFile: false,
  root: source,
  envDir: false,
  publicDir: false,
  base: "/",
  plugins: [tailwind()],
  build: {
    outDir: output,
    emptyOutDir: true,
    assetsInlineLimit: 0,
    cssCodeSplit: true,
    sourcemap: false,
    rolldownOptions: {
      input: path.join(source, "proof.css"),
      output: { assetFileNames: "common/[name][extname]" },
    },
  },
});
const common = path.join(output, "common");
const stylesheets = (await readdir(common)).filter(name => name.endsWith(".css"));
if (stylesheets.length !== 1) throw new Error("UI proof must emit exactly one stylesheet");
if (stylesheets[0] !== "relay.css") await rename(path.join(common, stylesheets[0]), path.join(common, "relay.css"));
// The shared SSG uses this same Vite build/import path for public TSX packages.
// Its routed, hydrating SiteRoot is not needed for these static relay templates.
const server = path.join(output, ".server");
try {
  const result = await build({
    configFile: false,
    root: source,
    envDir: false,
    publicDir: false,
    mode: "production",
    oxc: { jsx: { runtime: "automatic", importSource: "react" } },
    ssr: { noExternal: [/^@ejm\//] },
    build: {
      ssr: path.join(source, "proof.tsx"),
      outDir: server,
      emptyOutDir: true,
      copyPublicDir: false,
      minify: false,
      sourcemap: false,
      manifest: false,
    },
  });
  const entries = (Array.isArray(result) ? result : [result])
    .flatMap(bundle => "output" in bundle ? bundle.output : [])
    .filter(chunk => chunk.type === "chunk" && chunk.isEntry);
  if (entries.length !== 1) throw new Error("UI proof must compile one server entry");
  const rendered = await import(pathToFileURL(path.join(server, entries[0].fileName)).href);
  await writeFile(path.join(output, "index.html"), rendered.html);
  await writeFile(path.join(output, "classes.json"), JSON.stringify(rendered.classes, null, 2) + "\n");
} finally {
  // Build-only React and shared TSX never remain in the served output.
  await rm(server, { recursive: true, force: true });
}
const copies = [
  ["@ejm/ui/theme-boot.js", "common/theme-boot.js"],
  ["@ejm/tokens/assets/projects/cli/mark-ink.svg", "common/cli-mark-ink.svg"],
  ["@ejm/tokens/assets/projects/cli/mark-paper.svg", "common/cli-mark-paper.svg"],
  ["@ejm/ui/fonts/Manrope-OFL.txt", "notices/Manrope-OFL.txt"],
  ["@ejm/ui/fonts/IBMPlexMono-OFL.txt", "notices/IBMPlexMono-OFL.txt"],
];
for (const [specifier, name] of copies) {
  const destination = path.join(output, name);
  await mkdir(path.dirname(destination), { recursive: true });
  await copyFile(fileURLToPath(import.meta.resolve(specifier)), destination);
}

async function files(directory) {
  const names = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) names.push(...await files(full));
    else if (entry.isFile()) names.push(path.relative(output, full).split(path.sep).join("/"));
    else throw new Error("UI proof output must contain only regular files");
  }
  return names.sort();
}
const expected = [
  "index.html", "classes.json", "common/relay.css", "common/theme-boot.js",
  "common/cli-mark-ink.svg", "common/cli-mark-paper.svg",
  "common/manrope-latin.woff2", "common/ibm-plex-mono-500-latin.woff2",
  "notices/Manrope-OFL.txt", "notices/IBMPlexMono-OFL.txt",
].sort();
const actual = await files(output);
if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error("Unexpected UI proof output; runtime framework chunks are forbidden");
const entries = await Promise.all(actual.map(async name => {
  const bytes = await readFile(path.join(output, name));
  return { path: name, raw: bytes.length, gzip: gzipSync(bytes).length, sha256: createHash("sha256").update(bytes).digest("hex") };
}));
const size = (name, kind) => entries.find(entry => entry.path === name)[kind];
const budgets = {
  cssGzip: { actual: size("common/relay.css", "gzip"), maximum: 14 * 1024 },
  commonScriptGzip: { actual: size("common/theme-boot.js", "gzip"), maximum: 2 * 1024 },
  templateRaw: { actual: size("index.html", "raw"), maximum: 48 * 1024 },
  fontsRaw: { actual: size("common/manrope-latin.woff2", "raw") + size("common/ibm-plex-mono-500-latin.woff2", "raw"), maximum: 34536 },
};
const passed = Object.values(budgets).every(budget => budget.actual <= budget.maximum);
await writeFile(path.join(output, "proof.json"), JSON.stringify({
  scope: "build-only proof; no production route, Worker or controller build",
  passed, budgets, files: entries,
  frameworkRuntimeChunks: 0,
  workerGrowth: "not measured by this proof",
}, null, 2) + "\n");
if (!passed) throw new Error("UI proof exceeds an approved budget; review proof.json before composition");
console.log(JSON.stringify({ output: "web/.ui-proof", passed, budgets }));
