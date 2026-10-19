// Tracer 5.1: share-link contract — slug shape, /w/<slug> + ?s=<slug>,
// save/load payloads, and restore through the existing backend ABI.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  isSlug,
  slugFromPath,
  slugFromSearch,
  slugFromLocation,
  shortlink,
  shareUrl,
  saveWorkspace,
  loadWorkspace,
  restoreWorkspace
} from '../src/lib/share.js';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const read = (rel) => readFileSync(join(root, rel), 'utf8');

const doc = () => ({
  version: 1,
  functions: { f: 'a*x^2' },
  params: { a: 2 },
  viewport: { x_min: -10, x_max: 10, y_min: -10, y_max: 10 }
});

const okJson = (data, status = 200) => ({
  status,
  json: async () => data
});

test('slugs are 8-char URL-safe', () => {
  assert.ok(isSlug('Ab12-_Xy'));
  assert.ok(!isSlug('short'));
  assert.ok(!isSlug('way-too-long'));
  assert.ok(!isSlug('bad slug!'));
  assert.ok(!isSlug(''));
  assert.ok(!isSlug(null));
});

test('primary /w/<slug> path parsing', () => {
  assert.equal(slugFromPath('/w/Ab12-_Xy'), 'Ab12-_Xy');
  assert.equal(slugFromPath('/w/Ab12-_Xy/'), 'Ab12-_Xy');
  assert.equal(slugFromPath('/'), null);
  assert.equal(slugFromPath('/w/short'), null);
  assert.equal(slugFromPath('/w/Ab12-_Xy/extra'), null);
});

test('fallback ?s=<slug> parsing', () => {
  assert.equal(slugFromSearch('?s=Ab12-_Xy'), 'Ab12-_Xy');
  assert.equal(slugFromSearch('s=Ab12-_Xy'), 'Ab12-_Xy');
  assert.equal(slugFromSearch('?x=1&s=Ab12-_Xy&y=2'), 'Ab12-_Xy');
  assert.equal(slugFromSearch('?s=short'), null);
  assert.equal(slugFromSearch('?s=%zz'), null);
  assert.equal(slugFromSearch(''), null);
});

test('location prefers primary path over fallback', () => {
  assert.equal(
    slugFromLocation({ pathname: '/w/AAAAAAAA', search: '?s=BBBBBBBB' }),
    'AAAAAAAA'
  );
  assert.equal(slugFromLocation({ pathname: '/', search: '?s=BBBBBBBB' }), 'BBBBBBBB');
  assert.equal(slugFromLocation({ pathname: '/', search: '' }), null);
});

test('shortlink and share URL shapes', () => {
  assert.equal(shortlink('Ab12-_Xy'), '/w/Ab12-_Xy');
  assert.equal(shareUrl('Ab12-_Xy', 'http://x.test'), 'http://x.test/w/Ab12-_Xy');
  assert.equal(shareUrl('Ab12-_Xy', ''), '/w/Ab12-_Xy');
});

test('save POSTs the workspace verbatim and returns the slug', async () => {
  let seen = null;
  const fetchFn = async (url, opts) => {
    seen = { url, opts };
    return okJson({ slug: 'Ab12-_Xy' }, 201);
  };
  const res = await saveWorkspace(doc(), fetchFn);
  assert.deepEqual(res, { ok: true, slug: 'Ab12-_Xy' });
  assert.equal(seen.url, '/api/workspaces');
  assert.equal(seen.opts.method, 'POST');
  assert.deepEqual(JSON.parse(seen.opts.body), doc());
});

test('save maps 413 / invalid / network to tracer errors', async () => {
  assert.deepEqual(await saveWorkspace(doc(), async () => ({ status: 413 })), {
    ok: false,
    error: 'too-large'
  });
  assert.deepEqual(await saveWorkspace({ version: 0 }, async () => okJson({}, 201)), {
    ok: false,
    error: 'invalid'
  });
  assert.deepEqual(
    await saveWorkspace(doc(), async () => {
      throw new Error('down');
    }),
    { ok: false, error: 'network' }
  );
});

test('load GETs by slug and validates the blob', async () => {
  const d = doc();
  const res = await loadWorkspace('Ab12-_Xy', async (url) => {
    assert.equal(url, '/api/workspaces/Ab12-_Xy');
    return okJson(d, 200);
  });
  assert.deepEqual(res, { ok: true, doc: d });
  assert.deepEqual(await loadWorkspace('short', async () => okJson(d, 200)), {
    ok: false,
    error: 'unknown'
  });
  assert.deepEqual(await loadWorkspace('Ab12-_Xy', async () => ({ status: 404 })), {
    ok: false,
    error: 'unknown'
  });
  assert.deepEqual(
    await loadWorkspace('Ab12-_Xy', async () => {
      throw new Error('down');
    }),
    { ok: false, error: 'network' }
  );
});

test('restore drives setFunction + updateParam + updateViewport only', () => {
  const calls = [];
  const backend = {
    setFunction: (id, formula) => {
      calls.push(['setFunction', id, formula]);
      return { ok: true, params: ['a'] };
    },
    updateParam: (k, v) => calls.push(['updateParam', k, v]),
    updateViewport: (x0, x1, y0, y1) => calls.push(['updateViewport', x0, x1, y0, y1])
  };
  const res = restoreWorkspace(backend, doc());
  assert.deepEqual(res, { ok: true });
  assert.ok(calls.some(([c, id]) => c === 'setFunction' && id === 'f'));
  assert.ok(calls.some(([c, k, v]) => c === 'updateParam' && k === 'a' && v === 2));
  assert.ok(calls.some(([c]) => c === 'updateViewport'));
  assert.deepEqual(restoreWorkspace(backend, { version: 0 }), {
    ok: false,
    error: 'invalid'
  });
});

test('App.svelte wires Share adjacent to FormulaBar with panel states', () => {
  const app = read('src/App.svelte');
  assert.ok(app.includes('Share'), 'Share control must exist');
  assert.ok(app.includes('share-panel'), 'share panel must exist');
  assert.ok(app.includes('Saving…'), 'saving state must use UX microcopy');
  assert.ok(app.includes('Link copied.'), 'copied confirmation must use UX microcopy');
  assert.ok(app.includes('Unknown workspace. Check the link and try again.'), '404 text must match UX');
  assert.ok(app.includes('Workspace too large to share.'), '413 text must match UX');
  assert.ok(
    app.includes("You're offline. Showing your local workspace."),
    'offline text must match UX'
  );
  assert.ok(app.includes('aria-live'), 'status note must be a live region');
  assert.ok(app.includes('slugFromLocation'), 'load must handle /w/<slug> + ?s fallback');
  assert.ok(app.includes('restoreWorkspace'), 'load must restore via the ABI recipe');
});
