// Epic 6 symbolic layer: UX `e^(…)` + implicit `*` normalize to explicit
// Rust-compatible form; differentiation mirrors core_math; validation stays
// char-exact under 50ms. Runs on plain node:test (no browser).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  TRY_PRESETS,
  validateOriginal,
  normalizeForBackend,
  deriveAll,
  format3,
  behaviorWord
} from '../src/lib/derive.js';
import { createFallback } from '../src/lib/fallback.js';

test('hero input e^(0.5x) validates and normalizes to exp', () => {
  const v = validateOriginal('e^(0.5x)');
  assert.equal(v.ok, true);
  assert.deepEqual(v.params, []);
  assert.equal(normalizeForBackend('e^(0.5x)'), 'exp(0.5*x)');
  const d = deriveAll('e^(0.5x)');
  assert.equal(d.normalized, 'exp(0.5*x)');
  // Display keeps e^ notation; backend stays Rust-compatible exp().
  assert.ok(d.primeDisplay.includes('e^') || d.primeDisplay.includes('exp'));
  assert.ok(d.primeBackend.includes('exp'));
  // Backend (fallback mirror) accepts both normalized and original.
  const b = createFallback();
  assert.equal(b.setFunction('f', d.normalized).ok, true);
  assert.equal(b.setFunction('f', 'e^(0.5x)').ok, true);
  assert.equal(b.setFunction('fp', d.primeBackend).ok, true);
});

test('mistyped character is char-exact within 50ms and blocks Analyze', () => {
  const bad = 'e^(0.5x';
  const t0 = performance.now();
  const res = validateOriginal(bad);
  const dt = performance.now() - t0;
  assert.equal(res.ok, false);
  assert.ok(Array.isArray(res.span) && res.span[1] <= bad.length + 10);
  assert.ok(res.message.length > 0);
  assert.ok(dt < 50, `validation took ${dt}ms, budget 50ms`);
  // A second typo family: unexpected operator.
  const bad2 = validateOriginal('2+*3');
  assert.equal(bad2.ok, false);
  assert.ok(bad2.span[1] <= '2+*3'.length);
});

test('clear-to-empty and Try chips fill-and-fire', () => {
  assert.equal(validateOriginal('').ok, false);
  assert.equal(TRY_PRESETS.length, 4);
  for (const p of TRY_PRESETS) {
    const v = validateOriginal(p.formula);
    assert.equal(v.ok, true, `${p.label}: ${p.formula}`);
    const d = deriveAll(p.formula);
    assert.ok(d.primeDisplay.length > 0);
    assert.ok(d.primeBackend.length > 0);
  }
});

test('a*x^2+b exposes a/b params with eval-only derivative', () => {
  const d = deriveAll('a*x^2+b');
  assert.deepEqual(d.params, ['a', 'b']);
  assert.ok(d.primeBackend.includes('a'));
  assert.ok(d.primeDisplay.includes('a'));
  // Second derivative exists for the always-on f'' line.
  assert.ok(d.doubleDisplay.length > 0);
  assert.ok(d.doubleBackend.length > 0);
});

test('1/x derivative and pole em-dash readout', () => {
  const d = deriveAll('1/x');
  assert.ok(d.primeDisplay.replace(/\s/g, '').includes('-1/x^2'));
  assert.equal(format3(NaN), '—');
  assert.equal(format3(Infinity), '—');
  assert.equal(format3(0.78432), '0.784');
  assert.equal(behaviorWord(0.5), 'Increasing');
  assert.equal(behaviorWord(-0.2), 'Decreasing');
  assert.equal(behaviorWord(0), 'Stationary');
  assert.equal(behaviorWord(NaN), '—');
});

test('cubic carries roots/extrema/inflection derivatives', () => {
  const d = deriveAll('x^3-3*x+1');
  assert.ok(d.primeDisplay.includes('x^2'));
  assert.ok(d.doubleDisplay.includes('x'));
});
