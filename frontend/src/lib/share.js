// Share/load helpers for Epic 5 tracer (5.1).
//
// Pure functions over explicit inputs so `npm test` covers the link contract
// without a browser or backend. Svelte owns UI state; Rust owns math state;
// Axum is a dumb JSON blob store (AD-6).

export const SLUG_RE = /^[A-Za-z0-9_-]{8}$/;

export function isSlug(s) {
  return typeof s === 'string' && SLUG_RE.test(s);
}

/** Primary form: `/w/<slug>` (with optional trailing slash). */
export function slugFromPath(pathname) {
  if (typeof pathname !== 'string') return null;
  const m = /^\/w\/([A-Za-z0-9_-]{8})\/?$/.exec(pathname);
  return m ? m[1] : null;
}

/** Fallback form: `?s=<slug>` query param. First valid value wins. */
export function slugFromSearch(search) {
  if (typeof search !== 'string') return null;
  const q = search.startsWith('?') ? search.slice(1) : search;
  for (const part of q.split('&')) {
    if (!part) continue;
    const eq = part.indexOf('=');
    const key = eq < 0 ? part : part.slice(0, eq);
    if (key !== 's') continue;
    const raw = eq < 0 ? '' : part.slice(eq + 1);
    let value;
    try {
      value = decodeURIComponent(raw);
    } catch {
      continue;
    }
    if (isSlug(value)) return value;
  }
  return null;
}

/**
 * Resolve the incoming share slug: primary `/w/<slug>` first, then
 * `?s=<slug>` fallback (mangled-link path, e.g. LMS rewriters).
 */
export function slugFromLocation({ pathname, search }) {
  return slugFromPath(pathname) ?? slugFromSearch(search) ?? null;
}

export function shortlink(slug) {
  return `/w/${slug}`;
}

/** Absolute share URL for clipboard handoff (falls back to shortlink). */
export function shareUrl(slug, origin) {
  if (typeof origin === 'string' && origin) return `${origin}${shortlink(slug)}`;
  return shortlink(slug);
}

function isWorkspaceDoc(doc) {
  if (!doc || typeof doc !== 'object') return false;
  if (doc.version !== 1) return false;
  if (!doc.functions || typeof doc.functions !== 'object') return false;
  if (!doc.params || typeof doc.params !== 'object') return false;
  const vp = doc.viewport;
  if (!vp || typeof vp !== 'object') return false;
  for (const k of ['x_min', 'x_max', 'y_min', 'y_max']) {
    if (typeof vp[k] !== 'number' || !Number.isFinite(vp[k])) return false;
  }
  return true;
}

/**
 * POST the flat `serializeWorkspace()` document verbatim.
 * Returns `{ok:true, slug}` or `{ok:false, error}` where error is one of
 * `'too-large' | 'invalid' | 'network'`.
 */
export async function saveWorkspace(doc, fetchFn = fetch) {
  if (!isWorkspaceDoc(doc)) return { ok: false, error: 'invalid' };
  let res;
  try {
    res = await fetchFn('/api/workspaces', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(doc)
    });
  } catch {
    return { ok: false, error: 'network' };
  }
  if (res.status === 201) {
    try {
      const data = await res.json();
      if (isSlug(data?.slug)) return { ok: true, slug: data.slug };
    } catch {
      // fall through to invalid
    }
    return { ok: false, error: 'invalid' };
  }
  if (res.status === 413) return { ok: false, error: 'too-large' };
  return { ok: false, error: 'invalid' };
}

/**
 * GET the stored blob. Returns `{ok:true, doc}` or
 * `{ok:false, error}` with `'unknown' | 'invalid' | 'network'`.
 */
export async function loadWorkspace(slug, fetchFn = fetch) {
  if (!isSlug(slug)) return { ok: false, error: 'unknown' };
  let res;
  try {
    res = await fetchFn(`/api/workspaces/${slug}`);
  } catch {
    return { ok: false, error: 'network' };
  }
  if (res.status === 404) return { ok: false, error: 'unknown' };
  if (res.status !== 200) return { ok: false, error: 'invalid' };
  try {
    const doc = await res.json();
    if (!isWorkspaceDoc(doc)) return { ok: false, error: 'invalid' };
    return { ok: true, doc };
  } catch {
    return { ok: false, error: 'invalid' };
  }
}

/**
 * Restore a workspace doc through the existing backend ABI only (AD-5):
 * `setFunction` + `updateParam` + `updateViewport` plus the style setters
 * (`setFunctionStyle` + `setFunctionVisibility`) for the optional 4.3
 * style map. No new ABI. A missing (or null) style map restores without
 * custom styles; malformed style entries are skipped, never fatal.
 * Returns `{ok:true}` or `{ok:false, error}`.
 */
export function restoreWorkspace(backend, doc) {
  if (!backend || !isWorkspaceDoc(doc)) return { ok: false, error: 'invalid' };
  try {
    for (const [id, formula] of Object.entries(doc.functions)) {
      if (typeof formula !== 'string') return { ok: false, error: 'invalid' };
      const res = backend.setFunction(id, formula);
      if (!res?.ok) return { ok: false, error: 'invalid' };
    }
    for (const [k, v] of Object.entries(doc.params)) {
      if (typeof v !== 'number' || !Number.isFinite(v)) continue;
      backend.updateParam(k, v);
    }
    const vp = doc.viewport;
    backend.updateViewport(vp.x_min, vp.x_max, vp.y_min, vp.y_max);
    const style = doc.style ?? {};
    if (style && typeof style === 'object') {
      for (const [id, entry] of Object.entries(style)) {
        if (!entry || typeof entry !== 'object') continue;
        const { color_hex, width_px, dashed, visible } = entry;
        if (
          typeof color_hex === 'string' &&
          typeof width_px === 'number' &&
          typeof dashed === 'boolean'
        ) {
          try {
            backend.setFunctionStyle(id, color_hex, width_px, dashed);
          } catch {
            // additive-only: a bad style entry never fails the restore
          }
        }
        if (typeof visible === 'boolean') {
          try {
            backend.setFunctionVisibility(id, visible);
          } catch {
            // additive-only: a bad visibility entry never fails the restore
          }
        }
      }
    }
    return { ok: true };
  } catch {
    return { ok: false, error: 'invalid' };
  }
}
