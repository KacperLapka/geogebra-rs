//! Adaptive curve sampler with pole/NaN guards (R3).
//!
//! A coarse uniform base pass (`BASE_SAMPLES`) captures the overall shape;
//! intervals whose midpoint deviates from the chord by more than a sub-pixel
//! y tolerance are recursively bisected (max `MAX_DEPTH`), up to a hard
//! `MAX_POINTS` cap. Pole intervals (non-finite or `|y| > POLE_BREAK`
//! endpoints) are never subdivided, so discontinuities stay clean breaks.

use std::collections::HashMap;

use core_math::Expr;

use crate::viewport::Viewport;

/// Hard cap on points per curve: the 2000-point / 8 ms budget shape.
pub const MAX_POINTS: usize = 2000;

/// Coarse base resolution before refinement.
pub const BASE_SAMPLES: usize = 257;

/// Maximum bisection depth per base interval.
pub const MAX_DEPTH: u8 = 8;

/// Magnitude above which a sample is treated as a pole break, not a point.
pub const POLE_BREAK: f64 = 1e6;

/// Sample `expr` across the viewport with adaptive sub-pixel refinement.
///
/// `n` is the point budget (clamped to at least 2, at most [`MAX_POINTS`]).
/// Returns between 2 and `budget` points sorted by ascending x; flat spans
/// stay near base density while high-curvature spans refine up to the budget.
/// Non-finite or diverging samples become `NaN` y values so the frontend can
/// break the polyline instead of drawing a vertical spike.
pub fn sample_curve(
    expr: &Expr,
    params: &HashMap<String, f64>,
    viewport: &Viewport,
    n: usize,
) -> Vec<[f64; 2]> {
    let budget = n.clamp(2, MAX_POINTS);
    sample_curve_adaptive(expr, params, viewport, budget, default_tolerance(viewport))
}

/// Adaptive sampler with explicit y tolerance.
///
/// Refines any interval whose midpoint deviates from the chord by more than
/// `tol_y`. `max_points` caps the output; `tol_y <= 0` or non-finite falls
/// back to the default viewport-derived tolerance.
///
/// The negated width comparison below is deliberate NaN-safe ordering: a NaN
/// width must take the early return, and `!(w > 0.0)` rejects NaN where
/// `w <= 0.0` would not.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn sample_curve_adaptive(
    expr: &Expr,
    params: &HashMap<String, f64>,
    viewport: &Viewport,
    max_points: usize,
    tol_y: f64,
) -> Vec<[f64; 2]> {
    let cap = max_points.clamp(2, MAX_POINTS);
    let tol = if tol_y.is_finite() && tol_y > 0.0 {
        tol_y
    } else {
        default_tolerance(viewport)
    };
    let width = viewport.width();
    let degenerate = !(width > 0.0) || !width.is_finite();
    if degenerate {
        return Vec::new();
    }

    let base_n = BASE_SAMPLES.min(cap);
    let mut pts: Vec<[f64; 2]> = Vec::with_capacity(cap);
    for i in 0..base_n {
        let x = viewport.x_min + width * i as f64 / (base_n - 1) as f64;
        pts.push([x, guard(expr.eval(x, params))]);
    }
    if base_n >= cap {
        return pts;
    }

    // Refine each base interval; collect new midpoints, then sort once.
    let mut extra: Vec<[f64; 2]> = Vec::new();
    let mut stack: Vec<(f64, f64, f64, f64, u8)> = Vec::with_capacity(256);
    for w in 0..pts.len().saturating_sub(1) {
        let ([x0, y0], [x1, y1]) = (pts[w], pts[w + 1]);
        if valid(y0) && valid(y1) {
            stack.push((x0, y0, x1, y1, 0));
        }
        while let Some((ax0, ay0, ax1, ay1, depth)) = stack.pop() {
            if pts.len() + extra.len() >= cap || depth >= MAX_DEPTH {
                break;
            }
            let xm = 0.5 * (ax0 + ax1);
            let ym = guard(expr.eval(xm, params));
            if !valid(ym) {
                continue;
            }
            let chord = 0.5 * (ay0 + ay1);
            if (ym - chord).abs() > tol {
                extra.push([xm, ym]);
                if pts.len() + extra.len() >= cap {
                    break;
                }
                // Push children only while budget remains.
                if depth + 1 < MAX_DEPTH {
                    stack.push((xm, ym, ax1, ay1, depth + 1));
                    stack.push((ax0, ay0, xm, ym, depth + 1));
                }
            }
        }
        if pts.len() + extra.len() >= cap {
            break;
        }
    }
    pts.extend(extra);
    pts.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));
    pts.truncate(cap);
    pts
}

/// Sub-pixel y tolerance: half a pixel at a 600-line frame.
fn default_tolerance(viewport: &Viewport) -> f64 {
    let h = viewport.height();
    if h.is_finite() && h > 0.0 {
        h / 1200.0
    } else {
        1e-3
    }
}

/// Vertex-buffer mirror of [`sample_curve`] as `f32` pairs for GPU upload.
///
/// NaN breaks are preserved so the line strip can split there.
pub fn sample_points_f32(
    expr: &Expr,
    params: &HashMap<String, f64>,
    viewport: &Viewport,
    n: usize,
) -> Vec<[f32; 2]> {
    sample_curve(expr, params, viewport, n)
        .into_iter()
        .map(|[x, y]| [x as f32, y as f32])
        .collect()
}

fn valid(y: f64) -> bool {
    y.is_finite()
}

fn guard(y: f64) -> f64 {
    if !y.is_finite() || y.abs() > POLE_BREAK {
        f64::NAN
    } else {
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_math::parse;

    #[test]
    fn samples_parabola_endpoints() {
        let expr = parse("a*x^2").unwrap();
        let mut p = HashMap::new();
        p.insert("a".to_string(), 1.0);
        let vp = Viewport::default();
        let pts = sample_curve(&expr, &p, &vp, 257);
        assert!((pts[0][0] + 10.0).abs() < 1e-9);
        assert!((pts[0][1] - 100.0).abs() < 1e-9);
        assert!((pts.last().unwrap()[0] - 10.0).abs() < 1e-9);
        assert!(pts.windows(2).all(|w| w[0][0] < w[1][0]));
    }

    #[test]
    fn small_budget_returns_exact_count() {
        let expr = parse("a*x^2").unwrap();
        let mut p = HashMap::new();
        p.insert("a".to_string(), 1.0);
        let pts = sample_curve(&expr, &p, &Viewport::default(), 5);
        assert_eq!(pts.len(), 5);
        assert!((pts[0][1] - 100.0).abs() < 1e-9);
    }

    #[test]
    fn flat_span_stays_near_base_density() {
        let expr = parse("2*x+1").unwrap();
        let pts = sample_curve(&expr, &HashMap::new(), &Viewport::default(), 2000);
        assert!(pts.len() < 600, "flat line over-refined: {}", pts.len());
        assert!(pts.len() >= BASE_SAMPLES);
    }

    #[test]
    fn high_curvature_refines_beyond_base() {
        let expr = parse("sin(50*x)").unwrap();
        let vp = Viewport::new(-3.0, 3.0, -2.0, 2.0);
        let pts = sample_curve(&expr, &HashMap::new(), &vp, 2000);
        assert!(
            pts.len() > BASE_SAMPLES,
            "expected refinement, got {}",
            pts.len()
        );
        // Refined polyline tracks the true curve within a few tolerances.
        let tol = default_tolerance(&vp);
        for w in pts.windows(2).take(40) {
            let xm = 0.5 * (w[0][0] + w[1][0]);
            let ym = expr.eval(xm, &HashMap::new());
            let chord = 0.5 * (w[0][1] + w[1][1]);
            if ym.is_finite() && w[0][1].is_finite() && w[1][1].is_finite() {
                assert!(
                    (ym - chord).abs() < 4.0 * tol + 1e-9,
                    "chord deviation too large at x={xm}"
                );
            }
        }
    }

    #[test]
    fn breaks_at_pole() {
        let expr = parse("1/x").unwrap();
        let pts = sample_curve(
            &expr,
            &HashMap::new(),
            &Viewport::new(-1.0, 1.0, -10.0, 10.0),
            257,
        );
        assert!(pts.iter().any(|p| p[1].is_nan()));
        // No vertical spike: neighbors of a break are not joined across it
        // with huge values; every finite y is within the pole guard.
        assert!(
            pts.iter()
                .all(|p| p[1].is_nan() || p[1].abs() <= POLE_BREAK)
        );
    }

    #[test]
    fn two_thousand_points_are_fast() {
        let expr = parse("sin(x)*exp(x/10)").unwrap();
        let start = std::time::Instant::now();
        let pts = sample_curve(&expr, &HashMap::new(), &Viewport::default(), 2000);
        assert!(pts.len() >= BASE_SAMPLES);
        assert!(pts.len() <= 2000);
        assert!(
            start.elapsed().as_millis() < 8,
            "2000-point budget exceeded"
        );
    }

    #[test]
    fn full_budget_refinement_stays_fast() {
        // High-frequency curve drives the sampler to its cap.
        let expr = parse("sin(90*x)").unwrap();
        let vp = Viewport::new(-10.0, 10.0, -2.0, 2.0);
        let start = std::time::Instant::now();
        let pts = sample_curve(&expr, &HashMap::new(), &vp, 2000);
        let dt = start.elapsed();
        assert_eq!(pts.len(), 2000);
        assert!(dt.as_millis() < 8, "capped regen exceeded budget: {dt:?}");
    }

    #[test]
    fn vertex_buffer_mirrors_samples() {
        let expr = parse("x^2").unwrap();
        let vp = Viewport::new(-2.0, 2.0, -1.0, 5.0);
        let verts = sample_points_f32(&expr, &HashMap::new(), &vp, 64);
        let pts = sample_curve(&expr, &HashMap::new(), &vp, 64);
        assert_eq!(verts.len(), pts.len());
        assert!(((verts[0][0] as f64) - pts[0][0]).abs() < 1e-5);
    }
}
