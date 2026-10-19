// Frontend symbolic layer (presentation only, contract-only via wasm_bridge ABI).
//
// - Validates the original formula bar text with char-exact spans (<50ms,
//   no debounce) supporting the UX `e^(…)` exponential plus implicit
//   multiplication (`0.5x`, `2(x+1)`), which the Rust parser spells
//   explicitly as `exp(…)` / `*`.
// - Normalizes to Rust-compatible explicit form (`e^X` → `exp(X)`,
//   standalone `e` → `exp(1)`, implicit `*` inserted) before every
//   `setFunction` so evaluation stays 100% in WASM/wgpu, zero server math.
// - Differentiates symbolically (mirroring core_math/diff + simplify) only
//   on Analyze; slider / viewport / toggle updates are eval-only via
//   `updateParam` + `samplePointsFor`, never refiring symbolic analysis.
// - Prints two forms: display (`e^(…)`) for the derivative card, backend
//   (`exp(…)`) for `setFunction("fp"/"fpp")` so f'/f'' sampling stays in
//   WASM via the existing `setFunction` / `samplePointsFor` ABI (no new ABI).

const FUNCS = new Set(['sin', 'cos', 'tan', 'exp', 'ln', 'log', 'sqrt']);

export const TRY_PRESETS = [
  { label: 'Quadratic', formula: 'x^2-3*x+1' },
  { label: 'Sine wave', formula: 'sin(x)' },
  { label: 'Cubic', formula: 'x^3-3*x+1' },
  { label: 'Exponential', formula: 'e^(0.5x)' }
];

function stripPrefix(input) {
  const trimmedStart = input.replace(/^\s+/, '');
  const leadingWs = input.length - trimmedStart.length;
  const paren = trimmedStart.indexOf('(');
  if (paren >= 0) {
    const head = trimmedStart.slice(0, paren);
    if (head && /^[A-Za-z_][A-Za-z0-9_]*$/.test(head)) {
      const close = trimmedStart.indexOf(')', paren);
      if (close >= 0 && trimmedStart.slice(paren + 1, close).trim() === 'x') {
        const eq = trimmedStart.indexOf('=', close + 1);
        if (eq >= 0) {
          const body = trimmedStart.slice(eq + 1);
          return { body, offset: input.length - body.length };
        }
      }
    }
  }
  const eqY = trimmedStart.indexOf('=');
  if (eqY >= 0 && trimmedStart.slice(0, eqY).trim() === 'y') {
    const body = trimmedStart.slice(eqY + 1);
    return { body, offset: input.length - body.length };
  }
  return { body: trimmedStart, offset: leadingWs };
}

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
      // Guard `e` in `0.5e` (trailing exponent marker without digits) from
      // lexing as a number; let the identifier path handle it so `e` stays
      // the Euler constant and spans stay char-exact.
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
    throw { span: [i, i + 1], message: `Unexpected character '${c}'` };
  }
  return tokens;
}

function parseBody(body, offset) {
  if (!body.trim()) throw { span: [0, offset + body.length], message: 'Empty expression' };
  const raw = tokenize(body);
  const tokens = raw.map((t) => ({
    ...t,
    start: t.start + offset,
    end: t.end + offset
  }));
  let pos = 0;
  const peek = () => tokens[pos];

  function parseExpr(minPrec) {
    let left = parsePrefix();
    for (;;) {
      const t = peek();
      let op = null;
      let prec = 0;
      let rightAssoc = false;
      let implicit = false;
      if (t && (t.kind === '+' || t.kind === '-' || t.kind === '*' || t.kind === '/' || t.kind === '^')) {
        op = t.kind;
        prec = op === '+' || op === '-' ? 10 : op === '*' || op === '/' ? 20 : 30;
        rightAssoc = op === '^';
      } else if (
        t &&
        (t.kind === 'num' || t.kind === 'id' || t.kind === '(')
      ) {
        // Implicit multiplication: `0.5x`, `2(x+1)`, `)(`, `x(x+1)`.
        // Same precedence as explicit `*`, never consumes an operator token.
        op = '*';
        prec = 20;
        implicit = true;
      } else {
        return left;
      }
      if (prec < minPrec) return left;
      if (!implicit) pos++;
      const right = parseExpr(rightAssoc ? prec : prec + 1);
      left = { op, left, right };
    }
  }

  function parsePrefix() {
    const t = tokens[pos++];
    if (!t) {
      const end = tokens.length ? tokens[tokens.length - 1].end : offset + body.length;
      throw { span: [end, end], message: 'Unexpected end of input' };
    }
    if (t.kind === 'num') return { num: t.value };
    if (t.kind === 'id') {
      if (peek() && peek().kind === '(') {
        if (!FUNCS.has(t.value)) throw { span: [t.start, t.end], message: `Unknown function '${t.value}'` };
        pos++;
        if (peek() && peek().kind === ')') {
          throw { span: [t.start, tokens[pos].end], message: `Function '${t.value}' needs one argument` };
        }
        const arg = parseExpr(0);
        const close = tokens[pos++];
        if (!close || close.kind !== ')') throw { span: [t.start, t.end], message: `Expected ')'` };
        return { func: t.value, arg };
      }
      if (t.value === 'x') return { x: true };
      if (t.value === 'e') return { e: true };
      if (t.value === 'pi') return { pi: true };
      return { param: t.value };
    }
    if (t.kind === '-') return { neg: parseExpr(25) };
    if (t.kind === '+') return parseExpr(25);
    if (t.kind === '(') {
      const e = parseExpr(0);
      const close = tokens[pos++];
      if (!close || close.kind !== ')') throw { span: [t.start, t.end], message: `Expected ')'` };
      return e;
    }
    throw { span: [t.start, t.end], message: `Unexpected '${t.kind}'` };
  }

  const expr = parseExpr(0);
  if (pos < tokens.length) {
    const t = tokens[pos];
    if (t.kind === ')') throw { span: [t.start, t.end], message: `Unexpected ')'` };
    throw { span: [t.start, t.end], message: `Unexpected '${t.kind === 'id' ? t.value : t.kind}'` };
  }
  return expr;
}

export function parseOriginal(input) {
  const { body, offset } = stripPrefix(input);
  return parseBody(body, offset);
}

export function validateOriginal(input) {
  try {
    const node = parseOriginal(input);
    return { ok: true, params: collectParams(node) };
  } catch (e) {
    return { ok: false, span: e.span ?? [0, 0], message: e.message ?? 'Parse error' };
  }
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

// --- Symbolic differentiation (mirrors core_math/diff) ---

function num(v) {
  return { num: v };
}
function add(l, r) {
  return { op: '+', left: l, right: r };
}
function sub(l, r) {
  return { op: '-', left: l, right: r };
}
function mul(l, r) {
  return { op: '*', left: l, right: r };
}
function div(l, r) {
  return { op: '/', left: l, right: r };
}
function pow(l, r) {
  return { op: '^', left: l, right: r };
}
function neg(e) {
  return { neg: e };
}
function call(func, arg) {
  return { func, arg };
}

function diffRaw(n) {
  if (n.num !== undefined) return num(0);
  if (n.x) return num(1);
  if (n.param) return num(0);
  if (n.e || n.pi) return num(0);
  if (n.neg) {
    const d = diffRaw(n.neg);
    if (d.num === 0) return num(0);
    return neg(d);
  }
  if (n.func) {
    const darg = diffRaw(n.arg);
    if (darg.num === 0) return num(0);
    const inner = n.arg;
    let outer;
    switch (n.func) {
      case 'sin':
        outer = call('cos', inner);
        break;
      case 'cos':
        outer = neg(call('sin', inner));
        break;
      case 'tan':
        outer = div(num(1), pow(call('cos', inner), num(2)));
        break;
      case 'exp':
        outer = call('exp', inner);
        break;
      case 'ln':
      case 'log':
        outer = div(num(1), inner);
        break;
      case 'sqrt':
        outer = div(num(1), mul(num(2), call('sqrt', inner)));
        break;
      default:
        return num(0);
    }
    return mul(outer, darg);
  }
  const l = n.left;
  const r = n.right;
  switch (n.op) {
    case '+':
      return add(diffRaw(l), diffRaw(r));
    case '-':
      return sub(diffRaw(l), diffRaw(r));
    case '*': {
      const dl = diffRaw(l);
      const dr = diffRaw(r);
      return add(mul(dl, clone(r)), mul(clone(l), dr));
    }
    case '/': {
      const dl = diffRaw(l);
      const dr = diffRaw(r);
      return div(sub(mul(dl, clone(r)), mul(clone(l), dr)), pow(clone(r), num(2)));
    }
    case '^':
      return diffPow(l, r, diffRaw(l), diffRaw(r));
    default:
      return num(0);
  }
}

function clone(n) {
  return JSON.parse(JSON.stringify(n));
}

function diffPow(base, exp, dbase, dexp) {
  // Constant exponent: c*base^(c-1)*dbase.
  if (exp.num !== undefined) {
    const c = exp.num;
    if (c === 0) return num(0);
    return mul(mul(num(c), pow(clone(base), num(c - 1))), dbase);
  }
  // Constant base `e`: e^v * d(v) (ln e = 1, no log factor).
  if (base.e) {
    return mul(pow(clone(base), clone(exp)), dexp);
  }
  // Constant numeric base: base^exp * ln(base) * dexp.
  if (base.num !== undefined) {
    return mul(mul(pow(clone(base), clone(exp)), call('ln', num(base.num))), dexp);
  }
  // General log-diff: u^v * (v' ln u + v u'/u).
  const u = clone(base);
  const v = clone(exp);
  return mul(
    pow(clone(base), clone(exp)),
    add(mul(dexp, call('ln', u)), mul(v, div(dbase, clone(base))))
  );
}

// --- Simplify (constant folding + identity elimination + sign norm) ---

function isZero(n) {
  return n && n.num === 0;
}
function isOne(n) {
  return n && n.num === 1;
}
function asNum(n) {
  return n && n.num !== undefined ? n.num : null;
}
function sameShape(a, b) {
  if (!a || !b) return false;
  if (a.num !== undefined && b.num !== undefined) return a.num === b.num;
  if (a.x && b.x) return true;
  if (a.e && b.e) return true;
  if (a.pi && b.pi) return true;
  if (a.param && b.param) return a.param === b.param;
  if (a.neg && b.neg) return sameShape(a.neg, b.neg);
  if (a.func && b.func) return a.func === b.func && sameShape(a.arg, b.arg);
  if (a.op && b.op) return a.op === b.op && sameShape(a.left, b.left) && sameShape(a.right, b.right);
  return false;
}

const FUNC_EVAL = {
  sin: Math.sin,
  cos: Math.cos,
  tan: Math.tan,
  exp: Math.exp,
  ln: Math.log,
  log: Math.log,
  sqrt: Math.sqrt
};

export function simplify(n) {
  if (!n) return n;
  if (n.num !== undefined || n.x || n.param || n.e || n.pi) return n;
  if (n.neg) {
    const e = simplify(n.neg);
    if (e.num !== undefined) return num(-e.num);
    if (e.neg) return e.neg;
    return { neg: e };
  }
  if (n.func) {
    const a = simplify(n.arg);
    // ln(e) = 1, log(e) = 1.
    if ((n.func === 'ln' || n.func === 'log') && a.e) return num(1);
    // exp(0) = 1, etc. via numeric fold.
    if (a.num !== undefined) {
      const f = FUNC_EVAL[n.func];
      if (f) {
        const v = f(a.num);
        if (Number.isFinite(v)) return num(v);
        // Non-finite folds (e.g. ln(-1)) stay symbolic; sampling shows a gap.
        return { func: n.func, arg: a };
      }
    }
    return { func: n.func, arg: a };
  }
  const l = simplify(n.left);
  const r = simplify(n.right);
  // Constant folding.
  if (l.num !== undefined && r.num !== undefined) {
    const a = l.num;
    const b = r.num;
    let v;
    switch (n.op) {
      case '+':
        v = a + b;
        break;
      case '-':
        v = a - b;
        break;
      case '*':
        v = a * b;
        break;
      case '/':
        v = a / b;
        break;
      case '^':
        v = Math.pow(a, b);
        break;
      default:
        v = NaN;
    }
    if (Number.isFinite(v)) return num(v);
    return { op: n.op, left: l, right: r };
  }
  switch (n.op) {
    case '+':
      if (isZero(l)) return r;
      if (isZero(r)) return l;
      return { op: '+', left: l, right: r };
    case '-':
      if (isZero(r)) return l;
      if (isZero(l)) return { neg: r };
      if (sameShape(l, r)) return num(0);
      return { op: '-', left: l, right: r };
    case '*':
      if (isZero(l) || isZero(r)) return num(0);
      if (isOne(l)) return r;
      if (isOne(r)) return l;
      // -1 * x → -x normalization is left to the printer.
      return { op: '*', left: l, right: r };
    case '/':
      if (isZero(l)) return num(0);
      if (isOne(r)) return l;
      if (sameShape(l, r)) return num(1);
      return { op: '/', left: l, right: r };
    case '^':
      if (r.num === 0) return num(1);
      if (r.num === 1) return l;
      if (l.num === 0 || l.num === 1) return num(l.num);
      // e^0 = 1, e^1 = e.
      if (l.e && r.num === 0) return num(1);
      if (l.e && r.num === 1) return { e: true };
      return { op: '^', left: l, right: r };
    default:
      return { op: n.op, left: l, right: r };
  }
}

export function differentiate(node) {
  return simplify(diffRaw(node));
}

// --- Printers ---

function precOf(n) {
  if (!n || n.num !== undefined || n.x || n.param || n.e || n.pi || n.func) return 100;
  if (n.neg) return 90;
  if (n.op === '^') return 30;
  if (n.op === '*' || n.op === '/') return 20;
  return 10;
}

function printNode(n, backend, parentPrec = 0) {
  if (n.num !== undefined) {
    // Print integers without trailing `.0` for readable `2*x` forms.
    const v = n.num;
    const s = Number.isInteger(v) ? String(v) : String(v);
    return { text: s, prec: 100 };
  }
  if (n.x) return { text: 'x', prec: 100 };
  if (n.param) return { text: n.param, prec: 100 };
  if (n.e) {
    if (backend) return { text: 'exp(1)', prec: 100 };
    return { text: 'e', prec: 100 };
  }
  if (n.pi) {
    if (backend) return { text: '(3.141592653589793)', prec: 100 };
    return { text: 'pi', prec: 100 };
  }
  if (n.neg) {
    const inner = printNode(n.neg, backend, 90);
    const t = inner.prec < 90 ? `-(${inner.text})` : `-${inner.text}`;
    return { text: t, prec: 90 };
  }
  if (n.func) {
    const a = printNode(n.arg, backend, 0);
    return { text: `${n.func}(${a.text})`, prec: 100 };
  }
  // Binary.
  if (n.op === '^' && n.left && n.left.e && backend) {
    // Backend has no `e` constant: `e^X` → `exp(X)`.
    const e = printNode(n.right, backend, 0);
    return { text: `exp(${e.text})`, prec: 100 };
  }
  const p = n.op === '^' ? 30 : n.op === '*' || n.op === '/' ? 20 : 10;
  const left = printNode(n.left, backend, p);
  // `^` is right-associative: right side binds at same precedence.
  const rightPrec = n.op === '^' ? p : p + 1;
  const right = printNode(n.right, backend, rightPrec);
  let l = left.text;
  let r = right.text;
  if (left.prec < p) l = `(${l})`;
  if (right.prec < rightPrec || (n.op === '-' && right.prec <= p) || (n.op === '/' && right.prec <= p)) r = `(${r})`;
  // Keep `*` explicit in both forms (backend requires it; display stays clear).
  return { text: `${l}${n.op}${r}`, prec: p };
}

export function printDisplay(node) {
  return printNode(node, false, 0).text;
}

export function printBackend(node) {
  return printNode(node, true, 0).text;
}

export function normalizeForBackend(input) {
  const node = parseOriginal(input);
  return printBackend(node);
}

export function deriveAll(input) {
  const node = parseOriginal(input);
  const f1 = differentiate(node);
  const f2 = differentiate(f1);
  return {
    node,
    params: collectParams(node),
    prime: f1,
    double: f2,
    primeDisplay: printDisplay(f1),
    doubleDisplay: printDisplay(f2),
    primeBackend: printBackend(f1),
    doubleBackend: printBackend(f2),
    normalized: printBackend(node)
  };
}

// --- Point-analysis helpers (formatting only, never spatial math) ---

export function format3(v) {
  if (v === null || v === undefined || !Number.isFinite(v)) return '—';
  return Number(v).toFixed(3);
}

export function behaviorWord(slope) {
  if (!Number.isFinite(slope)) return '—';
  if (Math.abs(slope) < 1e-9) return 'Stationary';
  return slope > 0 ? 'Increasing' : 'Decreasing';
}
