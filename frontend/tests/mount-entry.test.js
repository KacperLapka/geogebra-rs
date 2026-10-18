// Regression for bug 1.8: app mounts blank, no canvas.
// Svelte 5 removed the `new App({ target })` constructor API; the entry must
// use `mount(App, { target })` from 'svelte'. Runs on plain node:test, no deps.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const read = (rel) => readFileSync(join(root, rel), 'utf8');

const main = () => read('src/main.js');

// Repro: the defect was `new App({ target })`, which throws
// component_api_invalid_new on Svelte 5 and leaves #app empty.
test('entry mounts via Svelte 5 mount() (fails on new App constructor)', () => {
  const src = main();
  assert.match(src, /\bmount\s*\(\s*App\s*,/, 'main.js must call mount(App, ...)');
  assert.doesNotMatch(src, /new\s+App\s*\(/, 'main.js must not construct via new App(...)');
});

// Related: mount must come from the public 'svelte' surface, not a deep path.
test('entry imports mount from the svelte package root', () => {
  const src = main();
  assert.match(
    src,
    /import\s*\{\s*[^}]*\bmount\b[^}]*\}\s*from\s*['"]svelte['"]/,
    "main.js must import { mount } from 'svelte'"
  );
});

// Related: the mount target is the #app shell; index.html owns the shell,
// Svelte owns everything inside it.
test('entry mounts into the #app element', () => {
  const src = main();
  assert.match(
    src,
    /getElementById\s*\(\s*['"]app['"]\s*\)|querySelector\s*\(\s*['"]#app['"]\s*\)/,
    'main.js must target the #app element'
  );
});

// Related: the mounted instance stays default-exported for HMR and tests.
test('entry default-exports the mounted app', () => {
  const src = main();
  assert.match(src, /export\s+default\s+app\b/, 'main.js must default-export app');
});

// Related: index.html holds only the #app shell by design; the canvas itself
// renders via Canvas.svelte. Guards against "fixing" the blank by inlining
// static markup into index.html.
test('index.html keeps the bare #app shell with no static canvas', () => {
  const html = read('index.html');
  assert.match(html, /<div\s+id="app"\s*><\/div>/, 'index.html must keep <div id="app"></div>');
  assert.doesNotMatch(html, /<canvas\b/i, 'index.html must not contain a static <canvas>');
  assert.match(html, /src="\/src\/main\.js"/, 'index.html must load /src/main.js');
});

// Related: the tracer path stays wired through App.svelte after the fix.
test('App.svelte still wires formula bar, canvas, tooltip, and sliders', () => {
  const app = read('src/App.svelte');
  for (const name of ['FormulaBar', 'Canvas', 'Tooltip', 'Sliders']) {
    assert.ok(app.includes(name), `App.svelte must still reference ${name}`);
  }
  const canvas = read('src/Canvas.svelte');
  assert.match(canvas, /<canvas\b/, 'Canvas.svelte must still render a <canvas>');
});

// Related: the fix is the mount API, not a compat flag that keeps legacy
// construction alive.
test('vite config does not paper over the API with componentApi compat', () => {
  const cfg = read('vite.config.js');
  assert.doesNotMatch(cfg, /componentApi/, 'vite.config.js must not set compatibility.componentApi');
});
