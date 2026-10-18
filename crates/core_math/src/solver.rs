//! Root bracketing and Brent refinement (AD-3).
//!
//! Sign-change scan distinguishes true zeros from infinite-discontinuity
//! jumps: brackets adjacent to a pole (|f| beyond [`POLE_THRESHOLD`]) are
//! skipped, never emitted as markers.

use std::collections::HashMap;

use crate::ast::Expr;

/// Magnitude above which a sample counts as diverging toward a pole.
pub const POLE_THRESHOLD: f64 = 1e6;

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
}
