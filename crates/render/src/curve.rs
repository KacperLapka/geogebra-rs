//! Uniform curve sampler with pole/NaN guards (tracer).
//!
//! Adaptive curvature subdivision lands in the adaptive-sampler follow-up; this uniform pass
//! already meets the 2000-point / 8 ms shape (a straight evaluation loop).

use std::collections::HashMap;

use core_math::Expr;

use crate::viewport::Viewport;

/// Sample `expr` at `n` evenly spaced x positions across the viewport.
///
/// Returns one `[x, y]` pair per position. Non-finite or diverging samples
/// (|y| beyond [`POLE_BREAK`]) become `NaN` y values so the frontend can
/// break the polyline instead of drawing a vertical spike.
pub fn sample_curve(
    expr: &Expr,
    params: &HashMap<String, f64>,
    viewport: &Viewport,
    n: usize,
) -> Vec<[f64; 2]> {
    let n = n.max(2);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let x = viewport.x_min + viewport.width() * i as f64 / (n - 1) as f64;
        let y = expr.eval(x, params);
        out.push([x, guard(y)]);
    }
    out
}

/// Magnitude above which a sample is treated as a pole break, not a point.
pub const POLE_BREAK: f64 = 1e6;

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
        let pts = sample_curve(&expr, &p, &Viewport::default(), 5);
        assert_eq!(pts.len(), 5);
        assert!((pts[0][0] + 10.0).abs() < 1e-9);
        assert!((pts[0][1] - 100.0).abs() < 1e-9);
        assert!((pts[2][1] - 0.0).abs() < 1e-9);
    }

    #[test]
    fn breaks_at_pole() {
        let expr = parse("1/x").unwrap();
        let pts = sample_curve(
            &expr,
            &HashMap::new(),
            &Viewport::new(-1.0, 1.0, -10.0, 10.0),
            5,
        );
        assert!(pts[2][1].is_nan());
    }

    #[test]
    fn two_thousand_points_are_fast() {
        let expr = parse("sin(x)*exp(x/10)").unwrap();
        let start = std::time::Instant::now();
        let pts = sample_curve(&expr, &HashMap::new(), &Viewport::default(), 2000);
        assert_eq!(pts.len(), 2000);
        assert!(
            start.elapsed().as_millis() < 8,
            "2000-point budget exceeded"
        );
    }
}
