// Backend contract shared by the Rust bundle wrapper and the JS fallback.
// Every backend exposes the same synchronous surface:
//   kind, parseAndValidate, setFunction, setFunctionStyle,
//   setFunctionVisibility, updateParam, updateViewport,
//   setCanvasSize, queryHover, samplePoints, samplePointsFor,
//   gridLines, serializeWorkspace, gpuStatus

function asValid(raw) {
  if (raw && Array.isArray(raw.params)) return { ok: true, params: raw.params };
  return { ok: false, span: raw.span ?? [0, 0], message: raw.message ?? 'parse error' };
}

function wrapRust(pkg) {
  return {
    kind: 'rust',
    parseAndValidate: (formula) => asValid(JSON.parse(pkg.parse_and_validate(formula))),
    setFunction: (id, formula) => asValid(JSON.parse(pkg.set_function(id, formula))),
    setFunctionStyle: (id, colorHex, widthPx, dashed) =>
      JSON.parse(pkg.set_function_style(id, colorHex, widthPx, !!dashed)),
    setFunctionVisibility: (id, visible) => JSON.parse(pkg.set_function_visibility(id, !!visible)),
    updateParam: (key, value) => pkg.update_param(key, value),
    updateViewport: (x0, x1, y0, y1) => pkg.update_viewport(x0, x1, y0, y1),
    setCanvasSize: (w, h) => pkg.set_canvas_size(w, h),
    queryHover: (px, py) => JSON.parse(pkg.query_hover(px, py)),
    samplePoints: (n) => JSON.parse(pkg.sample_points(n)),
    samplePointsFor: (id, n) => JSON.parse(pkg.sample_points_for(id, n)),
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
    // sanity: the glue must export the tracer ABI including the style fns,
    // so a stale bundle predating them falls back instead of failing late
    if (
      typeof pkg.parse_and_validate !== 'function' ||
      typeof pkg.set_function_style !== 'function' ||
      typeof pkg.set_function_visibility !== 'function' ||
      typeof pkg.sample_points_for !== 'function'
    )
      throw new Error('abi mismatch');
    cached = wrapRust(pkg);
  } catch {
    const fallback = await import('./fallback.js');
    cached = fallback.createFallback();
  }
  return cached;
}
