// Backend contract shared by the Rust bundle wrapper and the JS fallback.
// Every backend exposes the same synchronous surface:
//   kind, parseAndValidate, setFunction, updateParam, updateViewport,
//   setCanvasSize, queryHover, samplePoints, gridLines, serializeWorkspace,
//   gpuStatus

function asValid(raw) {
  if (raw && Array.isArray(raw.params)) return { ok: true, params: raw.params };
  return { ok: false, span: raw.span ?? [0, 0], message: raw.message ?? 'parse error' };
}

function wrapRust(pkg) {
  return {
    kind: 'rust',
    parseAndValidate: (formula) => asValid(JSON.parse(pkg.parse_and_validate(formula))),
    setFunction: (id, formula) => asValid(JSON.parse(pkg.set_function(id, formula))),
    updateParam: (key, value) => pkg.update_param(key, value),
    updateViewport: (x0, x1, y0, y1) => pkg.update_viewport(x0, x1, y0, y1),
    setCanvasSize: (w, h) => pkg.set_canvas_size(w, h),
    queryHover: (px, py) => JSON.parse(pkg.query_hover(px, py)),
    samplePoints: (n) => JSON.parse(pkg.sample_points(n)),
    gridLines: () => JSON.parse(pkg.grid_lines()),
    serializeWorkspace: () => JSON.parse(pkg.serialize_workspace()),
    gpuStatus: () => JSON.parse(pkg.gpu_status())
  };
}

let cached = null;

/** Load the Rust bundle when `sync-pkg` linked one, else the JS fallback. */
export async function getBackend() {
  if (cached) return cached;
  try {
    const url = new URL(`${import.meta.env.BASE_URL}pkg/wasm_bridge.js`, window.location.href).href;
    const pkg = await import(/* @vite-ignore */ url);
    if (pkg.default) await pkg.default();
    // sanity: the glue must export the tracer ABI
    if (typeof pkg.parse_and_validate !== 'function') throw new Error('abi mismatch');
    cached = wrapRust(pkg);
  } catch {
    const fallback = await import('./fallback.js');
    cached = fallback.createFallback();
  }
  return cached;
}
