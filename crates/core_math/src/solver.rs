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
        if fb == 0.0 {
            return Some(b);
        }
        if (b - a).abs() < tol {
            return if fb.abs() < 1e-9 { Some(b) } else { None };
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

/// Find all real roots of `expr` in `[x_min, x_max]` as markers.
///
/// Viewport scan via [`scan_sign_changes`] plus [`brent`] refine at `tol`.
/// Brackets that converge with large residual (pole jumps) emit nothing,
/// and near-duplicate refinements (exact-zero samples) collapse to one marker.
pub fn find_roots(
    expr: &Expr,
    params: &HashMap<String, f64>,
    x_min: f64,
    x_max: f64,
    steps: usize,
    tol: f64,
) -> Vec<Marker> {
    let mut roots = Vec::new();
    for (a, b) in scan_sign_changes(expr, params, x_min, x_max, steps) {
        if let Some(r) = brent(|x| expr.eval(x, params), a, b, tol, 100) {
            if !r.is_finite() {
                continue;
            }
            let y = expr.eval(r, params);
            if !y.is_finite() || y.abs() > 1e-9 {
                continue;
            }
            if roots.iter().any(|m: &Marker| (m.x - r).abs() < 1e-9) {
                continue;
            }
            roots.push(Marker::new(MarkerKind::Root, r, y));
        }
    }
    roots
}

/// Find pole x-coordinates of `expr` in `[x_min, x_max]`.
///
/// Complement of [`scan_sign_changes`]: the same `|f| > POLE_THRESHOLD`
/// magnitude plus opposite-sign jump, with a Brent-residual second layer.
/// Every strict sign-change bracket (finite or infinite endpoints, exact
/// zeros excluded) is probed: brackets that refine via [`brent`] on `f`
/// with `1e-9` residual are true roots and skipped; the rest refine via
/// [`brent`] on the reciprocal `g = 1/f` (whose zero is the pole) with a
/// bisection fallback. Non-finite (`NaN`) samples never emit.
///
/// Returns refined x-coordinates, deduped at `1e-9`. `1/x` over `[-1,1]`
/// yields one x within `1e-9` of `0`; `x^2` and `sin` yield empty.
///
/// Negated comparisons are deliberate NaN-safe ordering: non-finite
/// bounds and steps must take the early return.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn find_poles(
    expr: &Expr,
    params: &HashMap<String, f64>,
    x_min: f64,
    x_max: f64,
    steps: usize,
    tol: f64,
) -> Vec<f64> {
    if !x_min.is_finite() || !x_max.is_finite() || !(x_min < x_max) {
        return Vec::new();
    }
    let steps = steps.max(1);
    let dx = (x_max - x_min) / steps as f64;
    if !dx.is_finite() || !(dx > 0.0) {
        return Vec::new();
    }
    let tol = if tol.is_finite() && tol > 0.0 {
        tol
    } else {
        1e-12
    };
    // Collect strict sign-jump brackets (inverse of `scan_sign_changes`
    // pole-skip): exact zeros are root touches, never poles; `NaN` has no
    // sign and is skipped, never emitted.
    let mut brackets: Vec<(f64, f64)> = Vec::new();
    let mut prev_x = x_min;
    let mut prev_y = expr.eval(prev_x, params);
    for i in 1..=steps {
        let x = x_min + dx * i as f64;
        let y = expr.eval(x, params);
        if !prev_y.is_nan()
            && !y.is_nan()
            && prev_y != 0.0
            && y != 0.0
            && prev_y.signum() != y.signum()
        {
            brackets.push((prev_x, x));
        }
        prev_x = x;
        prev_y = y;
    }
    let mut poles = Vec::new();
    for (a, b) in brackets {
        // First layer: true roots refine on `f` with small residual.
        let is_root = brent(|x| expr.eval(x, params), a, b, tol, 100)
            .map(|r| {
                let y = expr.eval(r, params);
                y.is_finite() && y.abs() <= 1e-9
            })
            .unwrap_or(false);
        if is_root {
            continue;
        }
        // Second layer: pole refines on reciprocal `g = 1/f`.
        let refined = brent(|x| 1.0 / expr.eval(x, params), a, b, tol, 100)
            .filter(|r| r.is_finite())
            .or_else(|| bisect_sign_jump(expr, params, a, b, tol));
        if let Some(r) = refined {
            if !r.is_finite() || r < x_min || r > x_max {
                continue;
            }
            let fr = expr.eval(r, params);
            if fr.is_nan() {
                continue;
            }
            let gr = 1.0 / fr;
            let not_root = !(fr.is_finite() && fr.abs() <= 1e-9);
            let pole_like = !fr.is_finite()
                || fr.abs() > POLE_THRESHOLD
                || (gr.is_finite() && gr.abs() < 1e-6);
            if not_root
                && pole_like
                && poles.iter().all(|prev: &f64| (*prev - r).abs() >= 1e-9)
            {
                poles.push(r);
            }
        }
    }
    poles
}

/// Bisection fallback for a sign-jump bracket: narrow the sign change to
/// `tol` width and return the midpoint. Returns `None` on exact-zero
/// midpoint (a root, not a pole) or `NaN` samples.
fn bisect_sign_jump(
    expr: &Expr,
    params: &HashMap<String, f64>,
    a: f64,
    b: f64,
    tol: f64,
) -> Option<f64> {
    let mut lo = a;
    let mut hi = b;
    let mut flo = expr.eval(lo, params);
    if flo.is_nan() || flo == 0.0 {
        return None;
    }
    for _ in 0..100 {
        if !lo.is_finite() || !hi.is_finite() {
            return None;
        }
        if (hi - lo).abs() < tol {
            break;
        }
        let mid = 0.5 * (lo + hi);
        if !mid.is_finite() {
            return None;
        }
        let fm = expr.eval(mid, params);
        if fm.is_nan() || fm == 0.0 {
            return None;
        }
        if flo.signum() != fm.signum() {
            hi = mid;
        } else {
            lo = mid;
            flo = fm;
        }
    }
    let r = 0.5 * (lo + hi);
    if r.is_finite() { Some(r) } else { None }
}

/// Shared scan-refine-filter-dedup pipeline for derived-expression zeros.
///
/// Mirrors [`find_roots`] exactly: [`scan_sign_changes`] brackets plus
/// [`brent`] refine at `tol` (100 iterations), finite + `1e-9` residual
/// filter, `1e-9` dedup. Returns refined x-coordinates.
fn refined_derived_zeros(
    derived: &Expr,
    params: &HashMap<String, f64>,
    x_min: f64,
    x_max: f64,
    steps: usize,
    tol: f64,
) -> Vec<f64> {
    let mut zeros = Vec::new();
    for (a, b) in scan_sign_changes(derived, params, x_min, x_max, steps) {
        if let Some(r) = brent(|x| derived.eval(x, params), a, b, tol, 100) {
            if !r.is_finite() {
                continue;
            }
            let y = derived.eval(r, params);
            if !y.is_finite() || y.abs() > 1e-9 {
                continue;
            }
            if zeros.iter().any(|prev: &f64| (*prev - r).abs() < 1e-9) {
                continue;
            }
            zeros.push(r);
        }
    }
    zeros
}

/// Find local extrema of `expr` in `[x_min, x_max]` as markers.
///
/// Zeros of `f'` (via [`differentiate`], always simplified per AD-3) are
/// classified by the second-derivative test: `f''(r) > 0` emits `LocalMin`,
/// `f''(r) < 0` emits `LocalMax`, and near-zero (`<= 1e-9`, e.g. `x^3` at
/// `0`) is skipped rather than guessed. Marker `y` is `f(r)`.
pub fn find_extrema(
    expr: &Expr,
    params: &HashMap<String, f64>,
    x_min: f64,
    x_max: f64,
    steps: usize,
    tol: f64,
) -> Vec<Marker> {
    let fp = differentiate(expr, "x");
    let fpp = differentiate(&fp, "x");
    let mut out = Vec::new();
    for r in refined_derived_zeros(&fp, params, x_min, x_max, steps, tol) {
        let second = fpp.eval(r, params);
        if !second.is_finite() || second.abs() <= 1e-9 {
            continue;
        }
        let kind = if second > 0.0 {
            MarkerKind::LocalMin
        } else {
            MarkerKind::LocalMax
        };
        let y = expr.eval(r, params);
        if !y.is_finite() {
            continue;
        }
        out.push(Marker::new(kind, r, y));
    }
    out
}

/// Find inflection points of `expr` in `[x_min, x_max]` as markers.
///
/// Zeros of `f''` (derived `differentiate`-then-`simplify` per AD-3) are
/// kept only on true concavity sign change: `f''` evaluated on both sides
/// of the refined zero must have strictly opposite finite signs. Flat
/// points without concavity change never emit.
pub fn find_inflections(
    expr: &Expr,
    params: &HashMap<String, f64>,
    x_min: f64,
    x_max: f64,
    steps: usize,
    tol: f64,
) -> Vec<Marker> {
    let fp = differentiate(expr, "x");
    let fpp = differentiate(&fp, "x");
    // Probe offset well above solver `tol` (numerically robust sign read)
    // and well below feature spacing (sin inflections are π apart).
    let delta = (tol * 100.0).clamp(1e-7, 1e-4);
    let mut out = Vec::new();
    for r in refined_derived_zeros(&fpp, params, x_min, x_max, steps, tol) {
        let left = fpp.eval(r - delta, params);
        let right = fpp.eval(r + delta, params);
        if !left.is_finite() || !right.is_finite() {
            continue;
        }
        if left.abs() > POLE_THRESHOLD || right.abs() > POLE_THRESHOLD {
            continue;
        }
        if left * right >= 0.0 {
            continue;
        }
        let y = expr.eval(r, params);
        if !y.is_finite() {
            continue;
        }
        out.push(Marker::new(MarkerKind::Inflection, r, y));
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

    #[test]
    fn tangent_with_slider_param_tracks_slope() {
        // f(x) = a*x^2, a = 2 → f' = 2*a*x; the slider path evaluates with
        // params, so tangent_at must thread them to both y0 and slope.
        let f = parse("a*x^2").unwrap();
        let mut params = HashMap::new();
        params.insert("a".to_string(), 2.0);
        let x0 = 1.0;
        let (y0, slope) = tangent_coeffs(&f, &params, x0);
        assert!((y0 - 2.0).abs() < 1e-12, "y0: got {y0}");
        assert!((slope - 4.0).abs() < 1e-12, "slope: got {slope}");
        for dx in [-0.5, 0.0, 1.0] {
            let got = tangent_at(&f, &params, x0, x0 + dx);
            assert!((got - (2.0 + 4.0 * dx)).abs() < 1e-12, "dx={dx}: got {got}");
        }
    }

    #[test]
    fn finds_all_cubic_roots_in_viewport() {
        // f(x) = x^3 - 3x → roots -√3, 0, √3 (entry 2.2 verify: cubic).
        let expr = parse("x^3-3*x").unwrap();
        let params = HashMap::new();
        let markers = find_roots(&expr, &params, -3.0, 3.0, 200, 1e-12);
        assert_eq!(markers.len(), 3, "brackets: {markers:?}");
        let s3 = 3.0f64.sqrt();
        for want in [-s3, 0.0, s3] {
            let m = markers
                .iter()
                .min_by(|a, b| {
                    (a.x - want)
                        .abs()
                        .partial_cmp(&(b.x - want).abs())
                        .unwrap()
                })
                .unwrap();
            assert!(
                (m.x - want).abs() < 1e-12,
                "root near {want}: got {}",
                m.x
            );
            assert!(
                m.y.abs() < 1e-12,
                "residual at {}: got {}",
                m.x,
                m.y
            );
            assert_eq!(m.kind, MarkerKind::Root);
        }
    }

    #[test]
    fn finds_trig_roots_across_viewport() {
        // sin(x) on [-7, 7] → roots -2π, -π, 0, π, 2π (trig half of verify).
        let expr = parse("sin(x)").unwrap();
        let params = HashMap::new();
        let markers = find_roots(&expr, &params, -7.0, 7.0, 400, 1e-12);
        assert_eq!(markers.len(), 5, "markers: {markers:?}");
        let pi = std::f64::consts::PI;
        for want in [-2.0 * pi, -pi, 0.0, pi, 2.0 * pi] {
            let m = markers
                .iter()
                .min_by(|a, b| {
                    (a.x - want)
                        .abs()
                        .partial_cmp(&(b.x - want).abs())
                        .unwrap()
                })
                .unwrap();
            assert!(
                (m.x - want).abs() < 1e-12,
                "root near {want}: got {}",
                m.x
            );
            assert!(m.y.abs() < 1e-12, "residual at {}: {}", m.x, m.y);
        }
    }

    #[test]
    fn pole_jumps_emit_no_markers() {
        let params = HashMap::new();
        // 1/x diverges at 0: no marker even on a dense viewport scan.
        let recip = parse("1/x").unwrap();
        assert!(find_roots(&recip, &params, -1.0, 1.0, 100, 1e-12).is_empty());
        // tan(x) has true zeros at -π, 0, π but poles at ±π/2: only zeros emit.
        let tan = parse("tan(x)").unwrap();
        let markers = find_roots(&tan, &params, -3.2, 3.2, 400, 1e-12);
        let pi = std::f64::consts::PI;
        assert_eq!(markers.len(), 3, "markers: {markers:?}");
        for want in [-pi, 0.0, pi] {
            assert!(
                markers.iter().any(|m| (m.x - want).abs() < 1e-12),
                "missing tan zero near {want}: {markers:?}"
            );
        }
        for pole in [-pi / 2.0, pi / 2.0] {
            assert!(
                markers.iter().all(|m| (m.x - pole).abs() > 1e-6),
                "pole marker near {pole}: {markers:?}"
            );
        }
    }

    #[test]
    fn finds_cubic_extrema_with_marker_data() {
        // f(x) = x^3 - 3x → LocalMax at x=-1 (y=2), LocalMin at x=+1 (y=-2).
        let expr = parse("x^3-3*x").unwrap();
        let params = HashMap::new();
        let markers = find_extrema(&expr, &params, -3.0, 3.0, 200, 1e-12);
        assert_eq!(markers.len(), 2, "markers: {markers:?}");
        let max = markers
            .iter()
            .find(|m| m.kind == MarkerKind::LocalMax)
            .expect("missing LocalMax");
        let min = markers
            .iter()
            .find(|m| m.kind == MarkerKind::LocalMin)
            .expect("missing LocalMin");
        assert!((max.x + 1.0).abs() < 1e-12, "max x: got {}", max.x);
        assert!((min.x - 1.0).abs() < 1e-12, "min x: got {}", min.x);
        for m in [&max, &min] {
            let want_y = m.x.powi(3) - 3.0 * m.x;
            assert!(
                (m.y - want_y).abs() < 1e-12,
                "y at {}: got {} want {want_y}",
                m.x,
                m.y
            );
        }
        assert!((max.y - 2.0).abs() < 1e-12, "max y: got {}", max.y);
        assert!((min.y + 2.0).abs() < 1e-12, "min y: got {}", min.y);
    }

    #[test]
    fn finds_sin_extrema_alternating() {
        // sin(x) on [-7, 7] → extrema at odd multiples of π/2.
        let expr = parse("sin(x)").unwrap();
        let params = HashMap::new();
        let mut markers = find_extrema(&expr, &params, -7.0, 7.0, 400, 1e-12);
        markers.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
        let pi = std::f64::consts::PI;
        let wants = [
            (-1.5 * pi, MarkerKind::LocalMax),
            (-0.5 * pi, MarkerKind::LocalMin),
            (0.5 * pi, MarkerKind::LocalMax),
            (1.5 * pi, MarkerKind::LocalMin),
        ];
        assert_eq!(markers.len(), wants.len(), "markers: {markers:?}");
        for (m, (want_x, want_kind)) in markers.iter().zip(wants.iter()) {
            assert!(
                (m.x - want_x).abs() < 1e-12,
                "x near {want_x}: got {}",
                m.x
            );
            assert_eq!(m.kind, *want_kind, "kind at {}", m.x);
            let want_y = want_x.sin();
            assert!(
                (m.y - want_y).abs() < 1e-12,
                "y at {}: got {} want {want_y}",
                m.x,
                m.y
            );
        }
        // Kinds must alternate min/max along x (sin extrema alternate).
        for w in markers.windows(2) {
            assert_ne!(w[0].kind, w[1].kind, "adjacent kinds: {w:?}");
        }
    }

    #[test]
    fn finds_cubic_inflection() {
        // f(x) = x^3 - 3x → single inflection at x=0 (y=0).
        let expr = parse("x^3-3*x").unwrap();
        let params = HashMap::new();
        let markers = find_inflections(&expr, &params, -3.0, 3.0, 200, 1e-12);
        assert_eq!(markers.len(), 1, "markers: {markers:?}");
        let m = &markers[0];
        assert_eq!(m.kind, MarkerKind::Inflection);
        assert!(m.x.abs() < 1e-12, "x: got {}", m.x);
        assert!(m.y.abs() < 1e-12, "y: got {}", m.y);
    }

    #[test]
    fn finds_sin_inflections_at_multiples_of_pi() {
        // sin(x) on [-7, 7] → inflections at -2π, -π, 0, π, 2π.
        let expr = parse("sin(x)").unwrap();
        let params = HashMap::new();
        let markers = find_inflections(&expr, &params, -7.0, 7.0, 400, 1e-12);
        let pi = std::f64::consts::PI;
        assert_eq!(markers.len(), 5, "markers: {markers:?}");
        for want in [-2.0 * pi, -pi, 0.0, pi, 2.0 * pi] {
            let m = markers
                .iter()
                .min_by(|a, b| {
                    (a.x - want)
                        .abs()
                        .partial_cmp(&(b.x - want).abs())
                        .unwrap()
                })
                .unwrap();
            assert!(
                (m.x - want).abs() < 1e-12,
                "inflection near {want}: got {}",
                m.x
            );
            assert_eq!(m.kind, MarkerKind::Inflection);
            assert!(
                (m.y - want.sin()).abs() < 1e-12,
                "y at {}: got {}",
                m.x,
                m.y
            );
        }
    }

    #[test]
    fn monotonic_function_has_no_extrema() {
        // f(x) = x^3 + 2x → f' = 3x^2 + 2 > 0 everywhere.
        let expr = parse("x^3+2*x").unwrap();
        let params = HashMap::new();
        assert!(find_extrema(&expr, &params, -3.0, 3.0, 200, 1e-12).is_empty());
    }

    #[test]
    fn pole_adjacent_derivatives_emit_no_markers() {
        // 1/x → f' = -1/x^2 never zero, f'' = 2/x^3 never zero.
        let expr = parse("1/x").unwrap();
        let params = HashMap::new();
        assert!(find_extrema(&expr, &params, -1.0, 1.0, 100, 1e-12).is_empty());
        assert!(find_inflections(&expr, &params, -1.0, 1.0, 100, 1e-12).is_empty());
    }

    #[test]
    fn poles_detect_reciprocal_but_not_parabola_or_sine() {
        // 1/x over [-1,1] → single pole within 1e-9 of 0.
        let params = HashMap::new();
        let recip = parse("1/x").unwrap();
        let poles = find_poles(&recip, &params, -1.0, 1.0, 200, 1e-12);
        assert_eq!(poles.len(), 1, "poles: {poles:?}");
        assert!(
            (poles[0] - 0.0).abs() < 1e-9,
            "pole x: got {}",
            poles[0]
        );
        // x^2 never jumps sign: no poles.
        let parab = parse("x^2").unwrap();
        assert!(find_poles(&parab, &params, -1.0, 1.0, 200, 1e-12).is_empty());
        // sin(x) sign jumps are true roots (Brent residual small): no poles.
        let sin = parse("sin(x)").unwrap();
        assert!(find_poles(&sin, &params, -7.0, 7.0, 400, 1e-12).is_empty());
        // Non-finite samples never emit: degenerate viewport is empty.
        assert!(find_poles(&recip, &params, 1.0, -1.0, 100, 1e-12).is_empty());
        assert!(find_poles(&recip, &params, f64::NAN, 1.0, 100, 1e-12).is_empty());
    }
}
