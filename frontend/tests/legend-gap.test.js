// 6.13 gap between function toggles and plot: legend must sit tight above
// canvas as single DESIGN token 8-12px, caption muted, toggles canvas-only.
// Runs on plain node:test (no browser) via source pins.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const appSrc = () => readFileSync(join(root, 'src/App.svelte'), 'utf8');
const legendSrc = () => readFileSync(join(root, 'src/StyleControls.svelte'), 'utf8');

const rowGapPx = (src) => {
  const m = src.match(/\.workspace\s*\{[^}]*row-gap:\s*(\d+)px/);
  return m ? Number(m[1]) : null;
};

const legendMargins = (src) => {
  const block = src.match(/\.legend\s*\{[^}]*\}/);
  assert.ok(block, '.legend rule must exist');
  const margins = block[0].match(/margin:\s*([^;]+);/);
  assert.ok(margins, '.legend must declare margin');
  const parts = margins[1].trim().split(/\s+/).map((v) => Number(v.replace('px', '')));
  if (parts.length === 1) return { top: parts[0], bottom: parts[0] };
  if (parts.length === 2) return { top: parts[0], bottom: parts[0] };
  if (parts.length === 3) return { top: parts[0], bottom: parts[2] };
  if (parts.length === 4) return { top: parts[0], bottom: parts[2] };
  return { top: NaN, bottom: NaN };
};

test('6.13 toggles sit tight above plot: single DESIGN token 8-12px', () => {
  const app = appSrc();
  const legend = legendSrc();
  const gap = rowGapPx(app);
  assert.ok(gap !== null, 'workspace must declare row-gap');
  const { bottom } = legendMargins(legend);
  const total = gap + bottom;
  assert.ok(total >= 8 && total <= 12, `toggle-canvas gap must be 8-12px single token, got row-gap ${gap}px + legend bottom ${bottom}px = ${total}px`);
  assert.ok(gap === 8 || gap === 12 || bottom === 8 || bottom === 12, 'gap must come from a single DESIGN token 8 or 12');
});

test('6.13 legend stays caption-size muted directly above canvas', () => {
  const legend = legendSrc();
  assert.ok(legend.includes('font-size: 12px'), 'legend must stay caption 12px');
  assert.ok(legend.includes('color: #6e6b8a') || legend.includes('color:#6e6b8a'), 'legend must stay ink-muted');
  const app = appSrc();
  const togglesBlock = app.match(/\.place-toggles\s*\{[^}]*\}/);
  const canvasBlock = app.match(/\.place-canvas\s*\{[^}]*\}/);
  assert.ok(togglesBlock, 'toggles placement must exist');
  assert.ok(canvasBlock, 'canvas placement must exist');
  assert.ok(/grid-column:\s*1/.test(togglesBlock[0]) && /grid-row:\s*1/.test(togglesBlock[0]), 'toggles must sit grid 1,1');
  assert.ok(/grid-column:\s*1/.test(canvasBlock[0]) && /grid-row:\s*2/.test(canvasBlock[0]), 'canvas must sit grid 1,2 directly below toggles');
  const domToggles = app.indexOf('place-toggles');
  const domCanvas = app.indexOf('place-canvas');
  assert.ok(domToggles >= 0 && domCanvas > domToggles, 'toggles must precede canvas in DOM tab order');
});

test('6.13 toggles stay canvas-only without refiring analysis', () => {
  const app = appSrc();
  const start = app.indexOf('function handleToggle');
  assert.ok(start >= 0, 'handleToggle must exist');
  const end = app.indexOf('}', app.indexOf('refresh()', start)) + 1;
  const fn = app.slice(start, end);
  assert.ok(fn.includes('setFunctionVisibility'), 'toggles must re-render canvas via visibility');
  assert.ok(fn.includes('refresh()'), 'toggles must refresh canvas');
  assert.ok(!fn.includes('setFunction('), 'toggles must not refire analysis via setFunction');
});

test('6.13 related cases stay within DESIGN tokens', () => {
  const app = appSrc();
  const legend = legendSrc();
  const gap = rowGapPx(app);
  assert.ok([4, 8, 12, 16, 24, 32].includes(gap), `workspace row-gap must stay a DESIGN token, got ${gap}px`);
  const { top, bottom } = legendMargins(legend);
  assert.equal(top, 8, 'legend top spacing above toggles preserved at 8px');
  assert.equal(bottom, 0, 'legend bottom contributes 0 so row-gap alone sets the 8px toggle-canvas gap');
  assert.ok(app.includes('margin-top: 16px'), 'workspace top rhythm unchanged');
});
