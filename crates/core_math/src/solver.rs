//! Root bracketing and Brent refinement (AD-3).
//!
//! Sign-change scan distinguishes true zeros from infinite-discontinuity
//! jumps: brackets adjacent to a pole (|f| beyond [`POLE_THRESHOLD`]) are
//! skipped, never emitted as markers.

use std::collections::HashMap;

use crate::ast::Expr;
use crate::diff::differentiate;

/// Magnitude above which a sample counts as diverging toward a pole.
pub const POLE_THRESHOLD: f64 = 1e6;

/// Marker category for calculus overlays (tracer stub for render integration).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    Root,
    LocalMin,
    LocalMax,
    Inflection,
}

/// Coordinate marker emitted by the solver pipeline (stub shape).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Marker {
    pub kind: MarkerKind,
    pub x: f64,
    pub y: f64,
}

impl Marker {
    pub fn new(kind: MarkerKind, x: f64, y: f64) -> Self {
        Self { kind, x, y }
    }
}

/// Tangent coefficients `(y0, slope)` at `x0`: `y0 = f(x0)`, `slope = f'(x0)`.
pub fn tangent_coeffs(
    expr: &Expr,
    params: &HashMap<String, f64>,
    x0: f64,
) -> (f64, f64) {
    let slope = differentiate(expr, "x").eval(x0, params);
    let y0 = expr.eval(x0, params);
    (y0, slope)
}

/// Evaluate the tangent line to `expr` at `x0`, sampled at `x`.
///
/// `tangent_at = f(x0) + f'(x0) * (x - x0)`.
pub fn tangent_at(
    expr: &Expr,
    params: &HashMap<String, f64>,
    x0: f64,
    x: f64,
) -> f64 {
    let (y0, slope) = tangent_coeffs(expr, params, x0);
    y0 + slope * (x - x0)
}

/// Refine a bracketed root with Brent's method.
///
/// `f(a)` and `f(b)` must have opposite signs (or one is exactly zero).
/// Returns `None` when the bracket is invalid or iteration stalls.
pub fn brent(
    f: impl Fn(f64) -> f64,
    mut a: f64,
    mut b: f64,
    tol: f64,
    max_iter: usize,
) -> Option<f64> {
    let mut fa = f(a);
    let mut fb = f(b);
    if !fa.is_finite() || !fb.is_finite() {
        return None;
    }
    if fa == 0.0 {
        return Some(a);
    }
    if fb == 0.0 {
        return Some(b);
    }
    if fa.signum() == fb.signum() {
        return None;
    }
    if fa.abs() < fb.abs() {
        std::mem::swap(&mut a, &mut b);
        std::mem::swap(&mut fa, &mut fb);
    }
    // `b` is the best guess (|fa| >= |fb|); `c` the previous `b`, `d` the one
    // before that. Bracket invariant: f(a) and f(b) disagree in sign.
    let mut c = a;
    let mut fc = fa;
    let mut d = a;
    let mut mflag = true;
    for _ in 0..max_iter {
        if fb == 0.0 || (b - a).abs() < tol {
            return Some(b);
        }
        let mut s = if fa != fc && fb != fc {
            // inverse quadratic interpolation
            a * fb * fc / ((fa - fb) * (fa - fc))
                + b * fa * fc / ((fb - fa) * (fb - fc))
                + c * fa * fb / ((fc - fa) * (fc - fb))
        } else {
            // secant fallback
            let denom = fb - fa;
            if denom == 0.0 {
                return None;
            }
            b - fb * (b - a) / denom
        };
        if !s.is_finite() {
            return None;
        }
        let edge = (3.0 * a + b) / 4.0;
        let in_range = if edge < b {
            s > edge && s < b
        } else {
            s > b && s < edge
        };
        let step_ok = if mflag {
            (s - b).abs() < (b - c).abs() / 2.0
        } else {
            (s - b).abs() < (c - d).abs() / 2.0
        };
        let interval_ok = if mflag {
            (b - c).abs() >= tol
        } else {
            (c - d).abs() >= tol
        };
        if !in_range || !step_ok || !interval_ok {
            s = (a + b) / 2.0;
            mflag = true;
        } else {
            mflag = false;
        }
        let fs = f(s);
        if !fs.is_finite() {
            return None;
        }
        if fs == 0.0 {
            return Some(s);
        }
        d = c;
        c = b;
        fc = fb;
        if fa * fs < 0.0 {
            b = s;
            fb = fs;
        } else {
            a = s;
            fa = fs;
        }
        if fa.abs() < fb.abs() {
            std::mem::swap(&mut a, &mut b);
            std::mem::swap(&mut fa, &mut fb);
        }
    }
    if fb.abs() < 1e-9 { Some(b) } else { None }
}

/// Scan `[x_min, x_max]` for sign changes of `expr`, returning brackets.
///
/// Samples `steps` intervals; brackets touching a pole are skipped (AD-3).
pub fn scan_sign_changes(
    expr: &Expr,
    params: &HashMap<String, f64>,
    x_min: f64,
    x_max: f64,
    steps: usize,
) -> Vec<(f64, f64)> {
    let steps = steps.max(1);
    let dx = (x_max - x_min) / steps as f64;
    let mut prev_x = x_min;
    let mut prev_y = expr.eval(prev_x, params);
    let mut out = Vec::new();
    for i in 1..=steps {
        let x = x_min + dx * i as f64;
        let y = expr.eval(x, params);
        if prev_y.is_finite() && y.is_finite() {
            let pole_adjacent = prev_y.abs() > POLE_THRESHOLD || y.abs() > POLE_THRESHOLD;
            if !pole_adjacent && (prev_y == 0.0 || y == 0.0 || prev_y.signum() != y.signum()) {
                out.push((prev_x, x));
            }
        }
        prev_x = x;
        prev_y = y;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn finds_root_of_line() {
        let expr = parse("x-2").unwrap();
        let params = HashMap::new();
        let brackets = scan_sign_changes(&expr, &params, -10.0, 10.0, 100);
        assert!(!brackets.is_empty());
        let (a, b) = brackets[0];
        let root = brent(|x| expr.eval(x, &params), a, b, 1e-12, 100).unwrap();
        assert!((root - 2.0).abs() < 1e-9);
    }

    #[test]
    fn skips_pole_of_reciprocal() {
        // 1/x jumps sign at 0 but diverges: no bracket may be emitted.
        let expr = parse("1/x").unwrap();
        let params = HashMap::new();
        let brackets = scan_sign_changes(&expr, &params, -1.0, 1.0, 4);
        assert!(brackets.is_empty());
    }

    #[test]
    fn tracer_bullet_polynomial_derivatives_simplify_and_tangent() {
        // f(x) = x^3 + 2x → f' = 3x^2 + 2, f'' = 6x (entry 2.1).
        let f = parse("x^3+2*x").unwrap();
        let params = HashMap::new();
        let fp = differentiate(&f, "x");
        let fpp = differentiate(&fp, "x");
        for i in -8..=8 {
            let x = i as f64 * 0.5;
            let want_p = 3.0 * x * x + 2.0;
            let want_pp = 6.0 * x;
            assert!(
                (fp.eval(x, &params) - want_p).abs() < 1e-12,
                "f' x={x}: got {} want {want_p}",
                fp.eval(x, &params)
            );
            assert!(
                (fpp.eval(x, &params) - want_pp).abs() < 1e-12,
                "f'' x={x}: got {} want {want_pp}",
                fpp.eval(x, &params)
            );
        }
        // Tangent at x0: T(x) = f(x0) + f'(x0)*(x - x0).
        for x0 in [-2.0f64, 0.0, 1.5] {
            let y0 = x0.powi(3) + 2.0 * x0;
            let slope = 3.0 * x0 * x0 + 2.0;
            for dx in [-1.0, -0.25, 0.0, 0.5, 2.0] {
                let x = x0 + dx;
                let want = y0 + slope * dx;
                let got = tangent_at(&f, &params, x0, x);
                assert!(
                    (got - want).abs() < 1e-12,
                    "tangent x0={x0} x={x}: got {got} want {want}"
                );
            }
            let (got_y0, got_slope) = tangent_coeffs(&f, &params, x0);
            assert!((got_y0 - y0).abs() < 1e-12, "y0 at {x0}");
            assert!((got_slope - slope).abs() < 1e-12, "slope at {x0}");
        }
        // Marker stub shape carries coordinates for future render wiring.
        let m = Marker::new(MarkerKind::Root, 1.0, 3.0);
        assert_eq!(m.x, 1.0);
        assert_eq!(m.y, 3.0);
    }
}
