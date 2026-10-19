// 2.6 tangent + marker render integration: fallback shape parity with the
// Rust `sample_points` (curve only, empty overlays) so the dev loop never
// crashes without the bundle. Runs on plain node:test (no browser).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createFallback } from '../src/lib/fallback.js';

test('fallback samplePoints carries empty overlays beside working points', () => {
  const backend = createFallback();
  backend.setFunction('f', 'x^2');
  const sample = backend.samplePoints(16);
  assert.ok(sample.points.length > 0);
  assert.deepEqual(sample.markers, []);
  assert.equal(sample.tangent, null);
});

test('fallback queryHover reports no marker', () => {
  const backend = createFallback();
  backend.setFunction('f', 'x^2-1');
  const hover = backend.queryHover(440, 300);
  assert.equal(hover.marker, null);
  assert.ok(Number.isFinite(hover.math_x));
});

// 6.12 markers float off the curve: DOM overlay geometry must center dots
// on the curve via the same fractions toScreen strokes, skip out-of-range,
// and stay pinned across pan/zoom. Pure-geometry mirror of Canvas.svelte
// (xFrac/yFrac + markerOnScreen + markerTip) plus source pins so the tests
// fail on the defect (button-box offset, missing y-skip) and pass on fix.
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const canvasSrc = () => readFileSync(join(root, 'src/Canvas.svelte'), 'utf8');

const VIEW = { x_min: -5, x_max: 5, y_min: -5, y_max: 5 };
const toScreen = (x, y, w, h, vp = VIEW) => [
  ((x - vp.x_min) / (vp.x_max - vp.x_min)) * w,
  ((vp.y_max - y) / (vp.y_max - vp.y_min)) * h
];
const xFrac = (v, vp = VIEW) => (v - vp.x_min) / (vp.x_max - vp.x_min);
const yFrac = (v, vp = VIEW) => (vp.y_max - v) / (vp.y_max - vp.y_min);
const xSpanOk = (vp = VIEW) =>
  Number.isFinite(vp.x_max - vp.x_min) && vp.x_max > vp.x_min;
const ySpanOk = (vp = VIEW) =>
  Number.isFinite(vp.y_max - vp.y_min) && vp.y_max > vp.y_min;
// Mirror of Canvas.svelte markerOnScreen (the fix under test).
const markerOnScreen = (m, vp = VIEW) => {
  if (!Number.isFinite(m?.x) || !Number.isFinite(m?.y)) return false;
  if (!xSpanOk(vp) || !ySpanOk(vp)) return false;
  const fx = xFrac(m.x, vp);
  const fy = yFrac(m.y, vp);
  return fx >= 0 && fx <= 1 && fy >= 0 && fy <= 1;
};
const markerTip = (m) => {
  const f = (v) => (Number.isFinite(v) ? Number(v).toFixed(3) : '—');
  return `(${f(m.x)}, ${f(m.y)})`;
};
// Engine-assumed markers for x^3-3x+1 (solver values are contract, not math
// under test here): roots y=0, extrema (-1,3)/(1,-1), inflection (0,1).
const CUBIC_MARKERS = [
  { kind: 'root', x: -1.879385241571817, y: 0 },
  { kind: 'root', x: 0.347296355333861, y: 0 },
  { kind: 'root', x: 1.532088886237956, y: 0 },
  { kind: 'max', x: -1, y: 3 },
  { kind: 'min', x: 1, y: -1 },
  { kind: 'inflection', x: 0, y: 1 }
];

test('6.12 markers on curve: fractions agree with toScreen, tooltips agree', () => {
  const W = 800;
  const H = 420;
  for (const m of CUBIC_MARKERS) {
    assert.equal(markerOnScreen(m, VIEW), true, `${m.kind} (${m.x},${m.y}) must render`);
    const fx = xFrac(m.x, VIEW);
    const fy = yFrac(m.y, VIEW);
    const [px, py] = toScreen(m.x, m.y, W, H, VIEW);
    assert.ok(Math.abs(px - fx * W) < 1e-9, 'left% must equal toScreen x');
    assert.ok(Math.abs(py - fy * H) < 1e-9, 'top% must equal toScreen y');
    // Tooltip coordinate agrees with the curve value at same x.
    const tip = markerTip(m);
    assert.equal(tip, `(${Number(m.x).toFixed(3)}, ${Number(m.y).toFixed(3)})`);
  }
  // Roots sit on y=0, extrema/inflection carry their curve y.
  assert.equal(markerTip(CUBIC_MARKERS[0]), `(${CUBIC_MARKERS[0].x.toFixed(3)}, 0.000)`);
  assert.equal(markerTip({ x: -1, y: 3 }), '(-1.000, 3.000)');
});

test('6.12 out-of-range markers skip render, never float mid-canvas', () => {
  assert.equal(markerOnScreen({ x: 0, y: 100 }, VIEW), false, 'y above viewport hides');
  assert.equal(markerOnScreen({ x: 0, y: -100 }, VIEW), false, 'y below viewport hides');
  assert.equal(markerOnScreen({ x: 10, y: 0 }, VIEW), false, 'x outside viewport hides');
  assert.equal(markerOnScreen({ x: -10, y: 0 }, VIEW), false, 'x outside viewport hides');
  assert.equal(markerOnScreen({ x: 5, y: 5 }, VIEW), true, 'top-right edge stays');
  assert.equal(markerOnScreen({ x: -5, y: -5 }, VIEW), true, 'bottom-left edge stays');
  assert.equal(markerOnScreen({ x: NaN, y: 0 }, VIEW), false, 'NaN x hides');
  assert.equal(markerOnScreen({ x: 0, y: Infinity }, VIEW), false, 'infinite y hides');
  assert.equal(
    markerOnScreen({ x: 0, y: 0 }, { x_min: 5, x_max: 5, y_min: -5, y_max: 5 }),
    false,
    'degenerate x span hides (last good retained)'
  );
});

test('6.12 empty marker layer stays silent', () => {
  const visible = [].filter((m) => markerOnScreen(m, VIEW));
  assert.deepEqual(visible, []);
  const src = canvasSrc();
  assert.ok(!src.toLowerCase().includes('no markers found'), 'no empty-layer banner');
});

test('6.12 pan/zoom keeps markers pinned via the single mapping truth', () => {
  const W = 800;
  const H = 420;
  const m = { kind: 'max', x: -1, y: 3 };
  // Panned away: x outside the new domain hides instead of floating.
  const panned = { x_min: 0, x_max: 10, y_min: -5, y_max: 5 };
  assert.equal(markerOnScreen(m, panned), false);
  // Zoomed in: still visible and fractions still equal toScreen pixels.
  const zoomed = { x_min: -2, x_max: 2, y_min: -4, y_max: 4 };
  assert.equal(markerOnScreen(m, zoomed), true);
  const [px, py] = toScreen(m.x, m.y, W, H, zoomed);
  assert.ok(Math.abs(px - xFrac(m.x, zoomed) * W) < 1e-9);
  assert.ok(Math.abs(py - yFrac(m.y, zoomed) * H) < 1e-9);
  // Reset restores the default-domain pin.
  assert.equal(markerOnScreen(m, VIEW), true);
});

test('6.12 source pins: true dot-centering, range skip, viewport sync', () => {
  const src = canvasSrc();
  assert.ok(src.includes('markerOnScreen'), 'visibility must go through markerOnScreen');
  assert.ok(src.includes('{#if markerOnScreen(m)}'), 'layer must gate on markerOnScreen');
  assert.ok(src.includes('fx >= 0') && src.includes('fx <= 1'), 'x range skip required');
  assert.ok(src.includes('fy >= 0') && src.includes('fy <= 1'), 'y range skip required');
  assert.ok(src.includes('left:{xFrac(m.x)'), 'left% must stay tied to xFrac/toScreen');
  assert.ok(src.includes('top:{yFrac(m.y)'), 'top% must stay tied to yFrac/toScreen');
  assert.ok(src.includes('transform: translate(-50%, -50%)'), 'button center must sit on the point');
  // Scope centering/hit-target pins to the .marker rule itself: baseline
  // .toolbar/.empty-hint also flex-center, so a global substring would pass
  // on the defect. The bug was padding:16px + margin:-16px shifting the dot
  // 16px off the curve point.
  const start = src.indexOf('.marker {');
  const end = src.indexOf('.marker:focus-visible');
  assert.ok(start >= 0 && end > start, '.marker rule must exist');
  const markerCss = src.slice(start, end);
  assert.ok(markerCss.includes('display: flex'), '.marker must flex-center the dot');
  assert.ok(markerCss.includes('align-items: center'), 'dot must flex-center vertically');
  assert.ok(markerCss.includes('justify-content: center'), 'dot must flex-center horizontally');
  assert.ok(/(^|[^\w-])width: 44px/.test(markerCss) && !markerCss.includes('min-width'), '44px box width (not min-width) retained');
  assert.ok(/(^|[^\w-])height: 44px/.test(markerCss) && !markerCss.includes('min-height'), '44px box height (not min-height) retained');
  assert.ok(!markerCss.includes('padding: 16px'), 'hit-padding box offset must be gone');
  assert.ok(!markerCss.includes('margin: -16px'), 'negative-margin box offset must be gone');
  assert.ok(src.includes('title={markerTip(m)}'), 'tooltip coordinate retained');
  assert.ok(src.includes('aria-label={markerLabel(m)}'), 'focus label retained');
  assert.ok(src.includes('dirty'), 'dirty-rAF loop retained');
});
