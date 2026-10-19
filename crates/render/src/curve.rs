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

/// Minimum ribbon width in px (ABI + shader clamp floor).
pub const MIN_WIDTH_PX: f32 = 2.5;

/// Maximum ribbon width in px (ABI + shader clamp ceiling).
pub const MAX_WIDTH_PX: f32 = 6.0;

/// Default ribbon width in px for the tracer curve.
pub const DEFAULT_WIDTH_PX: f32 = 3.5;

/// Tracer-blue sRGB hex kept identical between the 2D fallback and GPU path.
pub const TRACER_HEX: &str = "#2563eb";

/// Per-function style owned by `render` (E4-STYLE-STRUCT).
///
/// Carried through the sample pipe as data, never a side channel.
/// `visible=false` skips submission but retains the stored entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FunctionStyle {
    /// Linear-sRGB RGBA, premultiplied-alpha off.
    pub color: [f32; 4],
    /// Ribbon width in px, clamped to `[MIN_WIDTH_PX, MAX_WIDTH_PX]`.
    pub width_px: f32,
    /// False for `f(x)`; true for derivative overlays (dashed, entry 4.2).
    pub dashed: bool,
    /// False skips buffer submission for that id, keeping the stored style.
    pub visible: bool,
}

/// Default style: tracer blue at 3.5px, solid, visible.
pub fn default_style() -> FunctionStyle {
    FunctionStyle {
        color: hex_to_linear(TRACER_HEX).unwrap_or([0.0, 0.0, 0.0, 1.0]),
        width_px: DEFAULT_WIDTH_PX,
        dashed: false,
        visible: true,
    }
}

/// Style for the Nth function. Index 0 is the tracer curve in tracer blue;
/// further indices reuse the default until the 8-colour cycle lands in 4.3.
pub fn for_function_index(index: usize) -> FunctionStyle {
    let _ = index;
    default_style()
}

/// Clamp a width into `[MIN_WIDTH_PX, MAX_WIDTH_PX]`; non-finite falls back
/// to [`DEFAULT_WIDTH_PX`].
pub fn clamp_width(w: f32) -> f32 {
    if !w.is_finite() {
        return DEFAULT_WIDTH_PX;
    }
    w.clamp(MIN_WIDTH_PX, MAX_WIDTH_PX)
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Parse `#RRGGBB` into linear-sRGB RGBA. Returns `None` on any malformed
/// input so the caller can reject it and keep the previous colour.
pub fn hex_to_linear(hex: &str) -> Option<[f32; 4]> {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&h[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&h[4..6], 16).ok()? as f32 / 255.0;
    Some([srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), 1.0])
}

/// One GPU ribbon vertex matching the E4-SHADER-CONTRACT layout:
/// `position_math: vec2f, normal_dir: vec2f, color: vec4f, width_px: f32,
/// dash_flag: f32, arclength_px: f32` (11 floats, 44-byte stride).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct CurveVertex {
    pub position_math: [f32; 2],
    pub normal_dir: [f32; 2],
    pub color: [f32; 4],
    pub width_px: f32,
    pub dash_flag: f32,
    pub arclength_px: f32,
}

/// Stride of [`CurveVertex`] in bytes; must match `curve_vertex_layout`.
pub const CURVE_VERTEX_STRIDE: usize = 44;

/// Expand sampled math points into a GPU ribbon (two verts per valid point).
///
/// Both verts share the sample position with opposite unit normals; the
/// vertex shader offsets by `normal * width / 2` in screen space. NaN or
/// non-finite samples (pole breaks) emit nothing and reset the arclength
/// accumulator, so no quad ever joins across a discontinuity. Hidden styles
/// (`visible=false`) expand to nothing so no sampler or ribbon work is spent
/// on a curve the frame will skip. `dash_flag` is `1.0` when `style.dashed`,
/// else `0.0` (`2.0` reserved for epic 3, never emitted here); arclength
/// stays CSS px at 1x and is scaled by `dpr` in-shader.
pub fn expand_ribbon(
    points: &[[f64; 2]],
    style: &FunctionStyle,
    viewport: &Viewport,
    canvas_w: f32,
    canvas_h: f32,
) -> Vec<CurveVertex> {
    if !style.visible {
        return Vec::new();
    }
    let width = clamp_width(style.width_px);
    let w = viewport.width();
    let h = viewport.height();
    let sx = if w.is_finite() && w > 0.0 && canvas_w.is_finite() && canvas_w > 0.0 {
        canvas_w as f64 / w
    } else {
        1.0
    };
    let sy = if h.is_finite() && h > 0.0 && canvas_h.is_finite() && canvas_h > 0.0 {
        canvas_h as f64 / h
    } else {
        1.0
    };
    // Unit normals need valid neighbours; collect validity once.
    let valid_at: Vec<bool> = points
        .iter()
        .map(|[x, y]| x.is_finite() && y.is_finite())
        .collect();
    let mut out: Vec<CurveVertex> = Vec::new();
    let mut arc: f32 = 0.0;
    let mut prev_valid: Option<[f64; 2]> = None;
    for (i, &[x, y]) in points.iter().enumerate() {
        if !valid_at[i] {
            prev_valid = None;
            arc = 0.0;
            continue;
        }
        // Direction from neighbours in screen px for a stable normal.
        let a = prev_valid.or_else(|| {
            points[i + 1..]
                .iter()
                .find(|p| p[0].is_finite() && p[1].is_finite())
                .copied()
        });
        let b = points[i + 1..]
            .iter()
            .find(|p| p[0].is_finite() && p[1].is_finite())
            .copied()
            .or(prev_valid);
        let (dx, dy) = match (a, b) {
            (Some(a), Some(b)) if a != [x, y] || b != [x, y] => {
                ((b[0] - a[0]) * sx, (b[1] - a[1]) * sy)
            }
            _ => (1.0, 0.0),
        };
        let len = (dx * dx + dy * dy).sqrt();
        let (nx, ny) = if len.is_finite() && len > 0.0 {
            (-dy / len, dx / len)
        } else {
            (0.0, 1.0)
        };
        if let Some(p) = prev_valid {
            let dpx = (x - p[0]) * sx;
            let dpy = (y - p[1]) * sy;
            let d = (dpx * dpx + dpy * dpy).sqrt();
            if d.is_finite() {
                arc += d as f32;
            }
        } else {
            arc = 0.0;
        }
        prev_valid = Some([x, y]);
        let pos = [x as f32, y as f32];
        let dash_flag = if style.dashed { 1.0 } else { 0.0 };
        out.push(CurveVertex {
            position_math: pos,
            normal_dir: [nx as f32, ny as f32],
            color: style.color,
            width_px: width,
            dash_flag,
            arclength_px: arc,
        });
        out.push(CurveVertex {
            position_math: pos,
            normal_dir: [-(nx as f32), -(ny as f32)],
            color: style.color,
            width_px: width,
            dash_flag,
            arclength_px: arc,
        });
    }
    out
}

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

    #[test]
    fn default_style_is_tracer_blue_at_3_5px() {
        let s = default_style();
        assert_eq!(s.width_px, 3.5);
        assert!(!s.dashed && s.visible);
        let expect = hex_to_linear("#2563eb").unwrap();
        for (a, b) in s.color.iter().zip(expect.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
        assert_eq!(for_function_index(0), s);
    }

    #[test]
    fn width_clamps_to_contract_range() {
        assert_eq!(clamp_width(1.0), MIN_WIDTH_PX);
        assert_eq!(clamp_width(10.0), MAX_WIDTH_PX);
        assert_eq!(clamp_width(3.5), 3.5);
        assert_eq!(clamp_width(f32::NAN), DEFAULT_WIDTH_PX);
        assert_eq!(clamp_width(f32::INFINITY), DEFAULT_WIDTH_PX);
    }

    #[test]
    fn bad_hex_rejected() {
        assert!(hex_to_linear("not-a-color").is_none());
        assert!(hex_to_linear("#zzz").is_none());
        assert!(hex_to_linear("#2563e").is_none());
        assert!(hex_to_linear("").is_none());
        let c = hex_to_linear("#2563eb").unwrap();
        assert_eq!(c[3], 1.0);
        assert!(c[0] < 0.1 && c[2] > 0.7);
    }

    #[test]
    fn ribbon_preserves_nan_breaks() {
        let expr = parse("1/x").unwrap();
        let vp = Viewport::new(-1.0, 1.0, -10.0, 10.0);
        let pts = sample_curve(&expr, &HashMap::new(), &vp, 257);
        assert!(pts.iter().any(|p| p[1].is_nan()));
        let style = default_style();
        let verts = expand_ribbon(&pts, &style, &vp, 800.0, 600.0);
        let valid = pts.iter().filter(|p| p[1].is_finite()).count();
        // Two verts per valid sample, none emitted for NaN breaks.
        assert_eq!(verts.len(), 2 * valid);
        assert!(verts.iter().all(|v| v.dash_flag == 0.0));
        assert!(verts.iter().all(|v| v.width_px == 3.5));
        // Opposite normals per pair, shared position: no quad joins a break.
        for pair in verts.as_chunks::<2>().0 {
            assert_eq!(pair[0].position_math, pair[1].position_math);
            assert!((pair[0].normal_dir[0] + pair[1].normal_dir[0]).abs() < 1e-6);
            assert!((pair[0].normal_dir[1] + pair[1].normal_dir[1]).abs() < 1e-6);
        }
    }

    #[test]
    fn hidden_style_expands_to_nothing() {
        let expr = parse("x^2").unwrap();
        let vp = Viewport::default();
        let pts = sample_curve(&expr, &HashMap::new(), &vp, 64);
        let mut style = default_style();
        style.visible = false;
        assert!(expand_ribbon(&pts, &style, &vp, 800.0, 600.0).is_empty());
    }

    #[test]
    fn dashed_flag_maps_from_style() {
        let vp = Viewport::default();
        let pts = vec![[-1.0, 1.0], [0.0, 0.0], [1.0, 1.0]];
        let solid = default_style();
        assert!(!solid.dashed);
        let solid_verts = expand_ribbon(&pts, &solid, &vp, 800.0, 600.0);
        assert!(!solid_verts.is_empty());
        assert!(solid_verts.iter().all(|v| v.dash_flag == 0.0));
        let mut dashed = default_style();
        dashed.dashed = true;
        let dashed_verts = expand_ribbon(&pts, &dashed, &vp, 800.0, 600.0);
        assert_eq!(dashed_verts.len(), solid_verts.len());
        assert!(dashed_verts.iter().all(|v| v.dash_flag == 1.0));
        // Pairs share the flag so no quad mixes solid/dashed.
        for pair in dashed_verts.as_chunks::<2>().0 {
            assert_eq!(pair[0].dash_flag, pair[1].dash_flag);
        }
    }

    #[test]
    fn dashed_arclength_restarts_after_nan_break() {
        let vp = Viewport::new(-1.0, 1.0, -10.0, 10.0);
        let pts = vec![
            [-1.0, -1.0],
            [-0.5, -0.5],
            [0.0, f64::NAN],
            [0.5, 0.5],
            [1.0, 1.0],
        ];
        let mut style = default_style();
        style.dashed = true;
        let verts = expand_ribbon(&pts, &style, &vp, 800.0, 600.0);
        // 4 valid samples -> 8 verts; NaN emits nothing.
        assert_eq!(verts.len(), 8);
        assert!(verts.iter().all(|v| v.dash_flag == 1.0));
        // One arclength per pair: restart to zero after the break.
        let arcs: Vec<f32> = verts.iter().step_by(2).map(|v| v.arclength_px).collect();
        assert_eq!(arcs.len(), 4);
        assert!((arcs[0] - 0.0).abs() < 1e-6);
        assert!(arcs[1] > arcs[0]);
        assert!((arcs[2] - 0.0).abs() < 1e-6);
        assert!(arcs[3] > arcs[2]);
    }

    #[test]
    fn never_emits_reserved_dash_flag_2_0() {
        let expr = parse("1/x").unwrap();
        let vp = Viewport::new(-1.0, 1.0, -10.0, 10.0);
        let pts = sample_curve(&expr, &HashMap::new(), &vp, 257);
        for dashed in [false, true] {
            let mut style = default_style();
            style.dashed = dashed;
            let verts = expand_ribbon(&pts, &style, &vp, 800.0, 600.0);
            assert!(!verts.is_empty());
            for v in &verts {
                assert!(
                    v.dash_flag == 0.0 || v.dash_flag == 1.0,
                    "bad flag {}",
                    v.dash_flag
                );
                assert!((v.dash_flag - 2.0).abs() > 1e-6);
            }
        }
    }

    #[test]
    fn hidpi_dash_period_scales_with_dpr() {
        // Locks the HiDPI contract with no public surface change: the shader
        // must scale period/on/fade by dpr and scale arclength once in the
        // vertex stage, so dashes stay 8/6 CSS px at any DPR.
        let src = include_str!("../shaders/curve.wgsl");
        assert!(src.contains("14.0 * dpr"), "period must scale with dpr");
        assert!(src.contains("8.0 * dpr"), "dash-on must scale with dpr");
        assert!(
            src.contains("arclength_px * dpr"),
            "arclength must scale by dpr in-shader"
        );
        // CPU keeps arclength in CSS px: identical points give identical
        // arclength regardless of the style flag, so one buffer is correct
        // at any DPR.
        let vp = Viewport::default();
        let pts = vec![[-1.0, 1.0], [0.0, 0.0], [1.0, 1.0]];
        let solid = expand_ribbon(&pts, &default_style(), &vp, 800.0, 600.0);
        let mut dashed_style = default_style();
        dashed_style.dashed = true;
        let dashed = expand_ribbon(&pts, &dashed_style, &vp, 800.0, 600.0);
        assert_eq!(solid.len(), dashed.len());
        for (s, d) in solid.iter().zip(dashed.iter()) {
            assert!((s.arclength_px - d.arclength_px).abs() < 1e-6);
        }
    }

    #[test]
    fn ribbon_vertex_is_44_bytes() {
        assert_eq!(std::mem::size_of::<CurveVertex>(), CURVE_VERTEX_STRIDE);
        assert_eq!(CURVE_VERTEX_STRIDE, 44);
    }
}
