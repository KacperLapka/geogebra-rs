// 4.3 palette/width/visibility polish: fallback backend parity with the
// Rust bridge — cycle order/wrap, width clamp, visibility retain/restore,
// bad-hex rejection, NaN-break preservation, style serialize/restore.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createFallback } from '../src/lib/fallback.js';
import { restoreWorkspace } from '../src/lib/share.js';

const TRACER = [37 / 255, 99 / 255, 235 / 255].map((c) =>
  c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4)
);

const close = (a, b) => Math.abs(a - b) < 1e-6;

test('cycle assigns f/g/h in order with f pinned to tracer', () => {
  const b = createFallback();
  b.setFunction('f', 'x^2');
  b.setFunction('g', 'x^3');
  b.setFunction('h', 'sin(x)');
  const ws = b.serializeWorkspace();
  assert.equal(ws.style.f.color_hex, '#2563EB');
  assert.notEqual(ws.style.g.color_hex, ws.style.f.color_hex);
  assert.notEqual(ws.style.h.color_hex, ws.style.f.color_hex);
  assert.notEqual(ws.style.h.color_hex, ws.style.g.color_hex);
  // g takes entry 1 (Okabe-Ito orange)
  assert.equal(ws.style.g.color_hex, '#E69F00');
});

test('width clamps to [2.5, 6.0] on the fallback layer', () => {
  const b = createFallback();
  b.setFunction('f', 'x^2');
  assert.equal(b.setFunctionStyle('f', '#2563eb', 1.0, false).width_px, 2.5);
  assert.equal(b.setFunctionStyle('f', '#2563eb', 10.0, false).width_px, 6.0);
  assert.equal(b.setFunctionStyle('f', '#2563eb', 3.5, false).width_px, 3.5);
});

test('per-curve visibility retains the entry and restores', () => {
  const b = createFallback();
  b.setFunction('f', 'x^2');
  b.setFunction('g', 'x^3');
  b.setFunctionStyle('g', '#E69F00', 4.0, false);
  b.setFunctionVisibility('g', false);
  assert.equal(b.samplePointsFor('g', 16).points.length, 0);
  // other curves unaffected
  assert.ok(b.samplePointsFor('f', 16).points.length > 0);
  const restored = b.setFunctionVisibility('g', true);
  assert.equal(restored.visible, true);
  assert.equal(restored.width_px, 4.0);
  assert.ok(b.samplePointsFor('g', 16).points.length > 0);
});

test('bad hex keeps the previous style', () => {
  const b = createFallback();
  b.setFunction('f', 'x^2');
  const before = b.setFunctionStyle('f', '#2563eb', 3.5, false);
  const after = b.setFunctionStyle('f', 'not-a-color', 3.5, false);
  assert.deepEqual(after, before);
});

test('pole breaks survive per-id sampling', () => {
  const b = createFallback();
  b.setFunction('g', '1/x');
  b.updateViewport(-1, 1, -10, 10);
  const { points } = b.samplePointsFor('g', 65);
  assert.ok(points.some(([, y]) => y === null), 'expected a null at the pole');
  assert.ok(points.every(([, y]) => y === null || Math.abs(y) <= 1e6));
});

test('serialize omits empty style map; restore reapplies through setters', () => {
  const b = createFallback();
  assert.equal(b.serializeWorkspace().style, undefined);
  b.setFunction('f', 'x^2');
  b.setFunctionStyle('f', '#E69F00', 4.0, true);
  const doc = b.serializeWorkspace();
  assert.equal(doc.style.f.color_hex, '#E69F00');
  assert.equal(doc.style.f.width_px, 4.0);
  assert.equal(doc.style.f.dashed, true);

  const c = createFallback();
  const res = restoreWorkspace(c, doc);
  assert.equal(res.ok, true);
  const back = c.serializeWorkspace();
  assert.equal(back.style.f.color_hex, '#E69F00');
  assert.equal(back.style.f.width_px, 4.0);

  // Missing map and explicit null both restore without custom styles.
  const { style, ...bare } = doc;
  assert.equal(restoreWorkspace(createFallback(), bare).ok, true);
  assert.equal(restoreWorkspace(createFallback(), { ...bare, style: null }).ok, true);
});

test('samplePointsFor shape matches samplePoints exactly', () => {
  const b = createFallback();
  b.setFunction('f', 'x^2');
  const one = b.samplePointsFor('f', 8);
  assert.deepEqual(Object.keys(one).sort(), ['params', 'points']);
  assert.equal(one.points.length, 8);
  assert.deepEqual(b.samplePointsFor('zzz', 8).points, []);
});
