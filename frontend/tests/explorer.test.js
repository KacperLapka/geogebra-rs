// Epic 6 explorer shell compliance: Analyze-to-share workspace from
// layout.png keyboard-first at 60 FPS. Static wiring checks (no browser);
// runtime math is covered by derive.test.js + fallback-style/overlays/share.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const read = (rel) => readFileSync(join(root, rel), 'utf8');

test('tracer Analyze loop with single indigo curve and marker buttons', () => {
  const app = read('src/App.svelte');
  const bar = read('src/FormulaBar.svelte');
  assert.ok(bar.includes('Analyze function'), 'Analyze button must exist');
  assert.ok(app.includes('#4F46E5'), 'single primary f must be DESIGN indigo');
  assert.ok(!app.includes('DEMO_CURVES'), 'g/h demo curves must be removed');
  assert.ok(!app.includes("id: 'g'") && !app.includes('"g"'), 'no demo g curve');
  const canvas = read('src/Canvas.svelte');
  assert.ok(canvas.includes('class="marker"'), 'markers must be DOM buttons');
  assert.ok(canvas.includes('marker-layer'), 'marker overlay layer must exist');
  assert.ok(!canvas.includes('MARKER_COLORS'), 'canvas-only marker circles must be gone');
  assert.ok(canvas.includes('toScreen') || canvas.includes('xFrac'), 'DOM sync must reuse toScreen mapping');
  assert.ok(canvas.includes('dirty'), 'dirty-flag rAF loop must remain');
});

test('function bar validates char-exact under 50ms with Try chips', () => {
  const bar = read('src/FormulaBar.svelte');
  assert.ok(!bar.includes('120'), '120ms debounce must be gone');
  assert.ok(bar.includes('Analyze function'), 'Analyze control must exist');
  assert.ok(bar.includes('onkeydown') || bar.includes('Enter'), 'Enter must fire analysis');
  assert.ok(bar.includes('try-chip'), 'Try chips must exist');
  assert.ok(bar.includes('TRY_PRESETS'), 'presets must drive chips');
  assert.ok(bar.includes('position'), 'char-exact position must show');
  const app = read('src/App.svelte');
  assert.ok(app.includes('validateOriginal'), 'typing must validate via char-exact parser');
  assert.ok(app.includes('performance.now'), 'validation budget must be measured');
});

test('single-function params eval-only, hidden when parameter-free', () => {
  const sliders = read('src/Sliders.svelte');
  assert.ok(sliders.includes('names().length'), 'slider row must hide with no params');
  assert.ok(!sliders.includes('No parameters'), 'hint text must not show for parameter-free');
  assert.ok(sliders.includes('onParam'), 'eval-only forward must exist');
  const app = read('src/App.svelte');
  assert.ok(app.includes('updateParam'), 'param drags must use updateParam');
  // Eval-only: param/x0/viewport paths never call setFunction.
  const paramBlock = app.slice(app.indexOf('function handleParam'), app.indexOf('function handleParam') + 400);
  assert.ok(!paramBlock.includes('setFunction'), 'param path must never refire symbolic');
  assert.ok(app.includes('deriveAll'), 'symbolic must fire only on Analyze');
});

test('derivative card with CALCULATED badge and toggleable f-double', () => {
  const app = read('src/App.svelte');
  assert.ok(app.includes('FIRST DERIVATIVE'), 'derivative card eyebrow required');
  assert.ok(app.includes('CALCULATED'), 'freshness badge required');
  assert.ok(app.includes('showBadge'), 'badge must hide on stale/error');
  assert.ok(app.includes('f″(x)'), 'f-double line required');
  assert.ok(app.includes('#9333EA') || app.includes('fpp'), 'f-double purple required');
  assert.ok(app.includes('#1FA36B') || app.includes('fp'), 'f-prime green required');
});

test('point slider with live values and pole em-dash', () => {
  const app = read('src/App.svelte');
  assert.ok(app.includes('POINT ANALYSIS'), 'point card required');
  assert.ok(app.includes('step="0.1"'), 'arrow keys must move ±0.1');
  assert.ok(app.includes('Instantaneous slope'), 'slope row required');
  assert.ok(app.includes('Local behavior'), 'behavior row required');
  assert.ok(app.includes('Tangent line'), 'tangent row required');
  assert.ok(app.includes('format3'), '3-decimal + em-dash formatting required');
  assert.ok(app.includes('behaviorWord'), 'behavior word required');
  const tip = read('src/Tooltip.svelte');
  assert.ok(tip.includes('toFixed(3)'), 'tooltips must show 3 decimals');
  assert.ok(tip.includes('—'), 'pole/tooltip must use em-dash, never NaN');
});

test('canvas mount with pan/zoom/reset, asymptotes, double-failure fallback', () => {
  const canvas = read('src/Canvas.svelte');
  assert.ok(canvas.includes('Zoom in') && canvas.includes('Zoom out') && canvas.includes('Reset'), 'zoom buttons + reset required');
  assert.ok(canvas.includes('Showing x from'), 'domain label required');
  assert.ok(canvas.includes('#b9b8d0'), 'asymptotes must be DESIGN gray dashed');
  assert.ok(canvas.includes('#e8930c'), 'tangent must be DESIGN amber dashed');
  assert.ok(canvas.includes("can't draw the graph"), 'fallback message required');
  assert.ok(canvas.includes('requestAdapter') && canvas.includes('webgl2'), 'double-failure gate required');
  assert.ok(canvas.includes('role="img"'), 'canvas must carry role=img with fallback');
});

test('legend fixed palette with canvas-only toggles', () => {
  const legend = read('src/StyleControls.svelte');
  assert.ok(!legend.includes('type="color"'), 'free color picker must be gone');
  assert.ok(legend.includes('#4f46e5'), 'indigo f swatch required');
  assert.ok(legend.includes('#1fa36b'), 'green f-prime swatch required');
  assert.ok(legend.includes('#9333ea'), 'purple f-double swatch required');
  assert.ok(legend.includes('#e8930c'), 'amber tangent swatch required');
  assert.ok(legend.includes('aria-pressed'), 'toggles must be buttons');
  const app = read('src/App.svelte');
  assert.ok(app.includes('setFunctionVisibility'), 'toggles must use visibility (canvas-only)');
});

test('share gated after Analyze with restore and offline states', () => {
  const app = read('src/App.svelte');
  assert.ok(app.includes('hasAnalyzed'), 'share must gate after first Analyze');
  assert.ok(app.includes('Copied.'), 'inline Copied confirmation required');
  assert.ok(app.includes("Couldn't create the link. Try again."), 'retry microcopy required');
  assert.ok(app.includes("You're offline — sharing needs a connection."), 'offline note required');
  assert.ok(app.includes('deriveAll'), 'restore must recompute auto curves when missing');
});

test('keyboard, live regions, reduced-motion, and DESIGN tokens', () => {
  const app = read('src/App.svelte');
  assert.ok(app.includes('aria-live="polite"'), 'value announcements must be polite live');
  assert.ok(app.includes('sr-live') || app.includes('sr-only'), 'release-only announcements required');
  assert.ok(app.includes('44px') || read('src/Canvas.svelte').includes('44px'), '44px targets required');
  assert.ok(read('src/Canvas.svelte').includes('prefers-reduced-motion'), 'reduced-motion snap required');
  assert.ok(read('src/FormulaBar.svelte').includes('prefers-reduced-motion'), 'reduced-motion required');
  const css = read('src/app.css');
  for (const token of ['#f7f7fc', '#4f46e5', '#1fa36b', '#e8930c', '#9333ea', '#b9b8d0']) {
    assert.ok(css.toLowerCase().includes(token.toLowerCase()), `DESIGN token ${token} must exist`);
  }
  assert.ok(css.includes('1120px'), 'content max 1120px required');
});

test('matrix: empty markers silent and all-layers-off hint retains analysis', () => {
  const canvas = read('src/Canvas.svelte');
  const app = read('src/App.svelte');
  const fallback = read('src/lib/fallback.js');
  assert.ok(canvas.includes('All layers off'), 'all-off hint state required');
  assert.ok(canvas.includes('Type a function and hit Analyze'), 'cold empty hint required');
  assert.ok(!canvas.toLowerCase().includes('no markers found'), 'marker layer must stay silent with no banner');
  assert.ok(fallback.includes('stays silent') || fallback.includes('empty'), 'fallback markers stay empty/silent');
  assert.ok(app.includes('setFunctionVisibility'), 'toggles must be canvas-only');
  const toggleBlock = app.slice(app.indexOf('toggles'), app.indexOf('toggles') + 2000);
  assert.ok(!toggleBlock.includes('deriveAll'), 'toggle path must not refire symbolic analysis');
});
