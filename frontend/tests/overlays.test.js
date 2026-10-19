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
