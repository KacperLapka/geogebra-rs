// 6.13 gap between function toggles and plot: legend must sit tight above
// canvas as single DESIGN token 8px. The columns must be independent: a
// shared grid row forces the short legend to the tall derivative card's
// height and reopens the gap below the toggles.
// Runs on plain node:test (no browser) via source pins.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const appSrc = () => readFileSync(join(root, 'src/App.svelte'), 'utf8');
const legendSrc = () => readFileSync(join(root, 'src/StyleControls.svelte'), 'utf8');

const ruleBody = (src, selector) => {
  const m = src.match(new RegExp(`${selector}\\s*\\{[^}]*\\}`));
  assert.ok(m, `${selector} rule must exist`);
  return m[0];
};

const legendMargins = (src) => {
  const block = ruleBody(src, '\\.legend');
  const margins = block.match(/margin:\s*([^;]+);/);
  assert.ok(margins, '.legend must declare margin');
  const parts = margins[1].trim().split(/\s+/).map((v) => Number(v.replace('px', '')));
  if (parts.length === 1) return { top: parts[0], bottom: parts[0] };
  if (parts.length === 2) return { top: parts[0], bottom: parts[0] };
  if (parts.length === 3) return { top: parts[0], bottom: parts[2] };
  if (parts.length === 4) return { top: parts[0], bottom: parts[2] };
  return { top: NaN, bottom: NaN };
};

test('6.13 toggles sit tight above plot: single DESIGN token 8px', () => {
  const app = appSrc();
  const legend = legendSrc();
  const colGap = ruleBody(app, '\\.graph-col').match(/gap:\s*(\d+)px/);
  assert.ok(colGap, 'left column must declare its own gap');
  const { bottom } = legendMargins(legend);
  const total = Number(colGap[1]) + bottom;
  assert.ok(total >= 8 && total <= 12, `toggle-canvas gap must be 8-12px single token, got column gap ${colGap[1]}px + legend bottom ${bottom}px = ${total}px`);
  assert.ok(Number(colGap[1]) === 8 || bottom === 8, 'gap must come from a single DESIGN token 8');
});

test('6.13 columns are independent: no shared grid row stretches the legend', () => {
  const app = appSrc();
  assert.ok(!app.includes('display: contents'), 'column wrappers must not be display:contents');
  const workspace = ruleBody(app, '\\.workspace');
  assert.ok(workspace.includes('align-items: start'), 'workspace must not stretch columns');
  const col = ruleBody(app, '\\.graph-col');
  assert.ok(col.includes('flex') && col.includes('column'), 'left column must stack legend over canvas');
  const rail = ruleBody(app, '\\.rail');
  assert.ok(rail.includes('flex') && rail.includes('column'), 'right column must stack independently');
  assert.ok(!/\.place-toggles\s*\{[^}]*grid-row/.test(app), 'toggles must not share a grid row');
  assert.ok(!/\.deriv-card\s*\{[^}]*grid-row/.test(app), 'derivative card must not share a grid row');
});

test('6.13 legend stays caption-size muted directly above canvas', () => {
  const legend = legendSrc();
  assert.ok(legend.includes('font-size: 12px'), 'legend must stay caption 12px');
  assert.ok(legend.includes('color: #6e6b8a') || legend.includes('color:#6e6b8a'), 'legend must stay ink-muted');
  const app = appSrc();
  const colStart = app.indexOf('graph-col');
  const domToggles = app.indexOf('place-toggles', colStart);
  const domCanvas = app.indexOf('place-canvas', colStart);
  assert.ok(domToggles >= 0 && domCanvas > domToggles, 'toggles must precede canvas in the left column');
  const railStart = app.indexOf('class="rail"');
  assert.ok(railStart > domCanvas, 'point/derivative/share rail must follow the left column');
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
  for (const selector of ['\\.graph-col', '\\.rail']) {
    const gap = ruleBody(app, selector).match(/gap:\s*(\d+)px/);
    assert.ok(gap && [4, 8, 12, 16, 24, 32].includes(Number(gap[1])), `${selector} gap must stay a DESIGN token`);
  }
  const { top, bottom } = legendMargins(legend);
  assert.equal(top, 8, 'legend top spacing preserved at 8px');
  assert.equal(bottom, 0, 'legend bottom contributes 0 so the column gap alone sets the 8px toggle-canvas gap');
});
