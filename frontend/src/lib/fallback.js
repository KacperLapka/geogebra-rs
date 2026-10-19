// Dev fallback: a pure-JS mirror of the tracer thin path, used only when no
// wasm-pack bundle is linked (see scripts/sync-pkg.js). Production math
// always comes from Rust; this exists so `npm run dev` demonstrates the
// parse-plot-hover-slider loop before the WASM toolchain runs.

const FUNCS = {
  sin: Math.sin,
  cos: Math.cos,
  tan: Math.tan,
  exp: Math.exp,
  ln: Math.log,
  log: Math.log,
  sqrt: Math.sqrt
};

function tokenize(input) {
  const tokens = [];
  let i = 0;
  while (i < input.length) {
    const c = input[i];
    if (/\s/.test(c)) {
      i++;
      continue;
    }
    if ('+-*/^(),'.includes(c)) {
      if (c === '*' && input[i + 1] === '*') {
        tokens.push({ kind: '^', start: i, end: i + 2 });
        i += 2;
      } else {
        tokens.push({ kind: c, start: i, end: i + 1 });
        i++;
      }
      continue;
    }
    const num = /^[0-9]*\.?[0-9]+([eE][+-]?[0-9]+)?/.exec(input.slice(i));
    if (num && num[0].length > 0 && /[0-9]/.test(num[0])) {
      tokens.push({ kind: 'num', value: parseFloat(num[0]), start: i, end: i + num[0].length });
      i += num[0].length;
      continue;
    }
    const id = /^[A-Za-z_][A-Za-z0-9_]*/.exec(input.slice(i));
    if (id) {
      tokens.push({ kind: 'id', value: id[0], start: i, end: i + id[0].length });
      i += id[0].length;
      continue;
    }
    throw { span: [i, i + 1], message: `unexpected character '${c}'` };
  }
  return tokens;
}

function stripPrefix(input) {
  const m = /^\s*[A-Za-z_][A-Za-z0-9_]*\s*\(\s*x\s*\)\s*=(.*)$/s.exec(input);
  if (m) return m[1];
  const y = /^\s*y\s*=(.*)$/s.exec(input);
  if (y) return y[1];
  return input;
}

function parseFormula(input) {
  const body = stripPrefix(input);
  if (!body.trim()) throw { span: [0, input.length], message: 'empty expression' };
  const tokens = tokenize(body);
  let pos = 0;
  const peek = () => tokens[pos];
  function parseExpr(minPrec) {
    let left = parsePrefix();
    for (;;) {
      const t = peek();
      const prec = t && (t.kind === '+' || t.kind === '-' ? 10 : t.kind === '*' || t.kind === '/' ? 20 : t.kind === '^' ? 30 : 0);
      if (!prec || prec < minPrec) return left;
      pos++;
      const right = parseExpr(t.kind === '^' ? 30 : prec + 1);
      left = { op: t.kind, left, right };
    }
  }
  function parsePrefix() {
    const t = tokens[pos++];
    if (!t) throw { span: [body.length, body.length], message: 'unexpected end of input' };
    if (t.kind === 'num') return { num: t.value };
    if (t.kind === 'id') {
      if (peek() && peek().kind === '(') {
        if (!FUNCS[t.value]) throw { span: [t.start, t.end], message: `unknown function '${t.value}'` };
        pos++;
        if (peek() && peek().kind === ')') throw { span: [t.start, tokens[pos].end], message: `function '${t.value}' needs one argument` };
        const arg = parseExpr(0);
        const close = tokens[pos++];
        if (!close || close.kind !== ')') throw { span: [t.start, t.end], message: `expected ')'` };
        return { func: t.value, arg };
      }
      return t.value === 'x' ? { x: true } : { param: t.value };
    }
    if (t.kind === '-') return { neg: parseExpr(25) };
    if (t.kind === '+') return parseExpr(25);
    if (t.kind === '(') {
      const e = parseExpr(0);
      const close = tokens[pos++];
      if (!close || close.kind !== ')') throw { span: [t.start, t.end], message: `expected ')'` };
      return e;
    }
    throw { span: [t.start, t.end], message: `unexpected '${t.kind}'` };
  }
  const expr = parseExpr(0);
  if (pos < tokens.length) {
    const t = tokens[pos];
    throw { span: [t.start, t.end], message: `unexpected '${t.kind}'` };
  }
  return expr;
}

function evaluate(node, x, params) {
  if (node.num !== undefined) return node.num;
  if (node.x) return x;
  if (node.param) return params[node.param] ?? NaN;
  if (node.neg) return -evaluate(node.neg, x, params);
  if (node.func) return FUNCS[node.func](evaluate(node.arg, x, params));
  const l = evaluate(node.left, x, params);
  const r = evaluate(node.right, x, params);
  switch (node.op) {
    case '+': return l + r;
    case '-': return l - r;
    case '*': return l * r;
    case '/': return l / r;
    case '^': return Math.pow(l, r);
  }
  return NaN;
}

function collectParams(node, out = []) {
  if (!node) return out;
  if (node.param && !out.includes(node.param)) out.push(node.param);
  collectParams(node.left, out);
  collectParams(node.right, out);
  collectParams(node.neg, out);
  collectParams(node.arg, out);
  return out;
}

const POLE = 1e6;
const guard = (y) => (Number.isFinite(y) && Math.abs(y) <= POLE ? y : null);

// Style parity with render::FunctionStyle (E4-STYLE-STRUCT).
const MIN_WIDTH_PX = 2.5;
const MAX_WIDTH_PX = 6.0;
const DEFAULT_WIDTH_PX = 3.5;
const TRACER_HEX = '#2563eb';
// 8-colour colourblind-safe cycle (E4-PALETTE): index 0 tracer-blue,
// `% 8` wraps. Mirrors render::PALETTE_HEXES.
const PALETTE_HEXES = ['#2563eb', '#E69F00', '#56B4E9', '#009E73', '#F0E442', '#0072B2', '#D55E00', '#CC79A7'];

const srgbToLinear = (c) => (c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4));

function hexToLinear(hex) {
  const h = typeof hex === 'string' && hex.startsWith('#') ? hex.slice(1) : hex;
  if (typeof h !== 'string' || h.length !== 6 || !/^[0-9a-fA-F]{6}$/.test(h)) return null;
  const r = parseInt(h.slice(0, 2), 16) / 255;
  const g = parseInt(h.slice(2, 4), 16) / 255;
  const b = parseInt(h.slice(4, 6), 16) / 255;
  return [srgbToLinear(r), srgbToLinear(g), srgbToLinear(b), 1];
}

const clampWidth = (w) => (Number.isFinite(w) ? Math.min(MAX_WIDTH_PX, Math.max(MIN_WIDTH_PX, w)) : DEFAULT_WIDTH_PX);
const isIdent = (s) => typeof s === 'string' && /^[A-Za-z_][A-Za-z0-9_]*$/.test(s);

// Grid parity with render::grid (1/2/5 nice steps, max 10, zero-snapped).
// Rust owns the viewport truth; this mirror exists only so `npm run dev`
// without the bundle demonstrates the same subdivision.
function tickStep(min, max, maxTicks) {
  if (!(min < max) || maxTicks === 0) return 0;
  const span = max - min;
  if (!Number.isFinite(span) || span <= 0) return 0;
  const raw = span / maxTicks;
  if (!Number.isFinite(raw) || raw <= 0) return 0;
  const mag = Math.pow(10, Math.floor(Math.log10(raw)));
  if (!Number.isFinite(mag) || mag <= 0) return 0;
  const found = [1, 2, 5, 10].map((m) => m * mag).find((s) => span / s <= maxTicks);
  return found ?? 10 * mag;
}

function collectTicks(min, max, step) {
  const out = [];
  let t = Math.ceil(min / step) * step;
  let guard = 0;
  while (t <= max && guard < 10000) {
    out.push(Math.abs(t) < step * 1e-9 ? 0 : t);
    t += step;
    guard += 1;
    if (!Number.isFinite(t)) break;
  }
  return out;
}

function gridTicks(min, max, maxTicks = 10) {
  const step = tickStep(min, max, maxTicks);
  if (!(step > 0)) return [];
  return collectTicks(min, max, step);
}

function minorTicks(min, max, maxTicks = 10) {
  const step = tickStep(min, max, maxTicks);
  if (!(step > 0)) return [];
  const mantissa = step / Math.pow(10, Math.floor(Math.log10(step)));
  const divs = Math.abs(mantissa - 2) < 1e-9 ? 4 : 5;
  const minor = step / divs;
  return collectTicks(min, max, minor).filter((v) => {
    const k = Math.round(v / step);
    return Math.abs(v - k * step) > step * 1e-6;
  });
}

function tickDecimals(step) {
  if (!Number.isFinite(step) || !(step > 0)) return 0;
  return Math.max(0, -Math.floor(Math.log10(step)));
}

function formatTick(step, value) {
  if (Object.is(value, -0)) return '0';
  if (Math.abs(value) < step * 1e-9) return '0';
  if (value === 0) return '0';
  const decimals = tickDecimals(step);
  return value.toFixed(decimals);
}

function linearToHex(c) {
  const toSrgb = (v) => {
    const s = v <= 0.0031308 ? 12.92 * v : 1.055 * Math.pow(v, 1 / 2.4) - 0.055;
    return Math.round(Math.min(1, Math.max(0, s)) * 255);
  };
  const h = (v) => toSrgb(v).toString(16).padStart(2, '0').toUpperCase();
  return `#${h(c[0])}${h(c[1])}${h(c[2])}`;
}

function paletteStyle(index) {
  return {
    color: hexToLinear(PALETTE_HEXES[index % PALETTE_HEXES.length]),
    width_px: DEFAULT_WIDTH_PX,
    dashed: false,
    visible: true
  };
}

export function createFallback() {
  const state = {
    functions: new Map(),
    params: new Map(),
    styles: new Map(),
    // First-seen style order for the palette cycle (`f` pinned to 0 and
    // never tracked here). Mirrors the bridge session.
    styleOrder: [],
    viewport: { x_min: -10, x_max: 10, y_min: -10, y_max: 10 },
    viewport_revision: 0,
    gpu: 'cpu-fallback',
    canvas: { w: 800, h: 600 }
  };

  const defaultStyle = () => ({
    color: hexToLinear(TRACER_HEX),
    width_px: DEFAULT_WIDTH_PX,
    dashed: false,
    visible: true
  });

  const ensureOrder = (id) => {
    if (id === 'f') return;
    if (!state.styleOrder.includes(id)) state.styleOrder.push(id);
  };

  const defaultStyleFor = (id) => {
    if (id === 'f') return paletteStyle(0);
    const pos = state.styleOrder.indexOf(id);
    return pos >= 0 ? paletteStyle(pos + 1) : paletteStyle(0);
  };

  const styleFor = (id) => {
    if (!state.styles.has(id)) state.styles.set(id, defaultStyleFor(id));
    return state.styles.get(id);
  };

  function validate(formula) {
    try {
      const node = parseFormula(formula);
      return { ok: true, node, params: collectParams(node) };
    } catch (e) {
      return { ok: false, span: e.span ?? [0, 0], message: e.message ?? 'parse error' };
    }
  }

  const active = () => state.functions.get('f') ?? (state.functions.size === 1 ? [...state.functions.values()][0] : null);
  const toWorld = (px, py) => {
    const v = state.viewport;
    return [v.x_min + (px / state.canvas.w) * (v.x_max - v.x_min), v.y_max - (py / state.canvas.h) * (v.y_max - v.y_min)];
  };

  return {
    kind: 'fallback',
    parseAndValidate(formula) {
      const r = validate(formula);
      return r.ok ? { ok: true, params: r.params } : { ok: false, span: r.span, message: r.message };
    },
    setFunction(id, formula) {
      const r = validate(formula);
      if (!r.ok) return { ok: false, span: r.span, message: r.message };
      for (const p of r.params) if (!state.params.has(p)) state.params.set(p, 1);
      state.functions.set(id, { node: r.node, text: formula });
      if (isIdent(id)) ensureOrder(id);
      styleFor(id);
      return { ok: true, params: r.params };
    },
    setFunctionStyle(id, colorHex, widthPx, dashed) {
      if (!isIdent(id)) return { ...defaultStyle() };
      const rgb = hexToLinear(colorHex);
      if (!rgb) return state.styles.has(id) ? { ...state.styles.get(id) } : { ...defaultStyleFor(id) };
      ensureOrder(id);
      const entry = styleFor(id);
      entry.color = rgb;
      entry.width_px = clampWidth(widthPx);
      entry.dashed = !!dashed;
      return { ...entry };
    },
    setFunctionVisibility(id, visible) {
      if (!isIdent(id)) return { ...defaultStyle() };
      ensureOrder(id);
      const entry = styleFor(id);
      entry.visible = !!visible;
      return { ...entry };
    },
    updateParam(key, value) {
      if (key === 'x' || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(key) || !Number.isFinite(value)) return;
      state.params.set(key, value);
    },
    updateViewport(x0, x1, y0, y1) {
      if (!(x0 < x1) || !(y0 < y1)) return;
      if (![x0, x1, y0, y1].every(Number.isFinite)) return;
      state.viewport = { x_min: x0, x_max: x1, y_min: y0, y_max: y1 };
      state.viewport_revision += 1;
    },
    setCanvasSize(w, h) {
      if (w > 0 && h > 0 && Number.isFinite(w) && Number.isFinite(h)) state.canvas = { w, h };
    },
    queryHover(px, py) {
      const [math_x] = toWorld(px, py);
      const fn = active();
      const math_y = fn ? guard(evaluate(fn.node, math_x, Object.fromEntries(state.params))) : null;
      return { math_x, math_y, marker: null };
    },
    samplePoints(n) {
      const fn = active();
      const count = Math.min(4000, Math.max(2, n | 0));
      const v = state.viewport;
      const params = Object.fromEntries(state.params);
      const points = fn
        ? Array.from({ length: count }, (_, i) => {
            const x = v.x_min + ((v.x_max - v.x_min) * i) / (count - 1);
            return [x, guard(evaluate(fn.node, x, params))];
          })
        : [];
      // Shape parity with the Rust `sample_points` (2.6 + 3.1): curve only,
      // empty overlays, so the dev loop never crashes without the bundle.
      return { points, params: [...state.params.keys()], markers: [], tangent: null, asymptotes: [] };
    },
    samplePointsFor(id, n) {
      const fn = state.functions.get(id);
      if (!fn) return { points: [], params: [...state.params.keys()] };
      const style = state.styles.get(id);
      if (style && style.visible === false) return { points: [], params: [...state.params.keys()] };
      const count = Math.min(4000, Math.max(2, n | 0));
      const v = state.viewport;
      const params = Object.fromEntries(state.params);
      const points = Array.from({ length: count }, (_, i) => {
        const x = v.x_min + ((v.x_max - v.x_min) * i) / (count - 1);
        return [x, guard(evaluate(fn.node, x, params))];
      });
      return { points, params: [...state.params.keys()] };
    },
    gridLines() {
      const v = state.viewport;
      const x = gridTicks(v.x_min, v.x_max);
      const y = gridTicks(v.y_min, v.y_max);
      const x_minor = minorTicks(v.x_min, v.x_max);
      const y_minor = minorTicks(v.y_min, v.y_max);
      const step_x = tickStep(v.x_min, v.x_max, 10);
      const step_y = tickStep(v.y_min, v.y_max, 10);
      const x_labels = x.map((value) => ({ value, text: formatTick(step_x, value) }));
      const y_labels = y.map((value) => ({ value, text: formatTick(step_y, value) }));
      return { x, y, x_minor, y_minor, x_labels, y_labels, step_x, step_y };
    },
    serializeWorkspace() {
      const doc = {
        version: 1,
        functions: Object.fromEntries([...state.functions].map(([k, f]) => [k, f.text])),
        params: Object.fromEntries(state.params),
        viewport: { ...state.viewport }
      };
      // Optional style map (4.3, additive-only): omitted when empty so an
      // unstyled session serializes exactly like the pre-4.3 shape.
      if (state.styles.size > 0) {
        doc.style = Object.fromEntries(
          [...state.styles].map(([id, s]) => [
            id,
            { color_hex: linearToHex(s.color), width_px: s.width_px, dashed: s.dashed, visible: s.visible }
          ])
        );
      }
      return doc;
    },
    gpuStatus() {
      return { status: state.gpu, viewport_revision: state.viewport_revision };
    }
  };
}
