//! Coordinate grid ticks with log subdivisions (R4).
//!
//! Major ticks follow human-friendly ("1/2/5") decade steps bounded to
//! `max_ticks`; [`minor_ticks`] subdivides each major step (÷5, or ÷4 for
//! `2`-mantissa steps) so zoom relabelling fills in smoothly instead of
//! jumping. [`format_tick`] renders stable labels at the step's decade
//! precision.

use crate::viewport::Viewport;

/// Nice major-tick positions for one axis range.
pub fn grid_ticks(min: f64, max: f64, max_ticks: usize) -> Vec<f64> {
    let step = tick_step(min, max, max_ticks);
    if step <= 0.0 {
        return Vec::new();
    }
    collect_ticks(min, max, step)
}

/// Major-tick step for a range, or `0.0` when the range is degenerate.
///
/// The negated comparison is deliberate NaN-safe ordering: NaN bounds must
/// yield `0.0`, and `!(a < b)` rejects NaN where `a >= b` would not.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn tick_step(min: f64, max: f64, max_ticks: usize) -> f64 {
    if !(min < max) || max_ticks == 0 {
        return 0.0;
    }
    let span = max - min;
    if !span.is_finite() || span <= 0.0 {
        return 0.0;
    }
    let raw = span / max_ticks as f64;
    if !raw.is_finite() || raw <= 0.0 {
        return 0.0;
    }
    let mag = 10f64.powf(raw.log10().floor());
    if !mag.is_finite() || mag <= 0.0 {
        return 0.0;
    }
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|m| m * mag)
        .find(|s| span / s <= max_ticks as f64)
        .unwrap_or(10.0 * mag)
}

/// Minor-tick positions subdividing the major step (excluding majors).
///
/// A `1`- or `5`-mantissa step splits into 5; a `2`-mantissa step splits
/// into 4 (halves), keeping minor spacing human-readable at every decade.
pub fn minor_ticks(min: f64, max: f64, max_ticks: usize) -> Vec<f64> {
    let step = tick_step(min, max, max_ticks);
    if step <= 0.0 {
        return Vec::new();
    }
    let mantissa = step / 10f64.powf(step.log10().floor());
    let divs = if (mantissa - 2.0).abs() < 1e-9 { 4 } else { 5 };
    let minor = step / divs as f64;
    collect_ticks(min, max, minor)
        .into_iter()
        .filter(|v| {
            // Drop values coinciding with a major tick.
            let k = (v / step).round();
            (v - k * step).abs() > step * 1e-6
        })
        .collect()
}

/// Decimals needed to render labels at this step without flicker.
pub fn tick_decimals(step: f64) -> usize {
    if !step.is_finite() || step <= 0.0 {
        return 0;
    }
    (-step.log10().floor() as i32).max(0) as usize
}

/// Stable axis label for a tick value at the given step.
///
/// Snaps tiny residues (including `-0.0`) to `"0"`.
pub fn format_tick(step: f64, value: f64) -> String {
    let decimals = tick_decimals(step);
    let v = if value.abs() < step * 1e-9 {
        0.0
    } else {
        value
    };
    if v == 0.0 {
        return "0".to_string();
    }
    format!("{v:.decimals$}")
}

/// Tick positions for both axes of a viewport.
pub fn viewport_ticks(viewport: &Viewport, max_ticks: usize) -> (Vec<f64>, Vec<f64>) {
    (
        grid_ticks(viewport.x_min, viewport.x_max, max_ticks),
        grid_ticks(viewport.y_min, viewport.y_max, max_ticks),
    )
}

/// Minor ticks for both axes of a viewport.
pub fn viewport_minor_ticks(viewport: &Viewport, max_ticks: usize) -> (Vec<f64>, Vec<f64>) {
    (
        minor_ticks(viewport.x_min, viewport.x_max, max_ticks),
        minor_ticks(viewport.y_min, viewport.y_max, max_ticks),
    )
}

/// One formatted axis label: the tick value plus its stable display text.
///
/// `text` comes from [`format_tick`] at the axis step, so zero snaps to
/// `"0"` (never `"-0"`) with step-appropriate decimals.
#[derive(Debug, Clone, PartialEq)]
pub struct GridLabel {
    pub value: f64,
    pub text: String,
}

/// Formatted labels for one axis range at its 1/2/5 step.
pub fn grid_labels_for_range(min: f64, max: f64, max_ticks: usize) -> Vec<GridLabel> {
    let step = tick_step(min, max, max_ticks);
    if step <= 0.0 {
        return Vec::new();
    }
    grid_ticks(min, max, max_ticks)
        .into_iter()
        .map(|value| {
            let text = format_tick(step, value);
            GridLabel { value, text }
        })
        .collect()
}

/// Formatted labels for both axes of a viewport.
///
/// Values match [`viewport_ticks`] one-for-one; texts match [`format_tick`]
/// at each axis step.
pub fn grid_labels(viewport: &Viewport, max_ticks: usize) -> (Vec<GridLabel>, Vec<GridLabel>) {
    (
        grid_labels_for_range(viewport.x_min, viewport.x_max, max_ticks),
        grid_labels_for_range(viewport.y_min, viewport.y_max, max_ticks),
    )
}

/// World-space grid line endpoints for a viewport (major plus minor).
///
/// Returns `f32` pairs in world coordinates: each vertical tick at `x`
/// emits `[x, y_min] -> [x, y_max]`, each horizontal tick at `y` emits
/// `[x_min, y] -> [x_max, y]`. Majors come first per axis, then minors, so
/// a consumer can split the buffer at the major count when it needs
/// distinct styling. The existing [`ViewportUniform`] transforms these
/// verbatim into NDC, keeping the 2D `toScreen` stroke and the future
/// `grid.wgsl` draw identical (f64 CPU / f32 GPU).
///
/// Degenerate or non-finite viewports yield no vertices.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn grid_vertices(viewport: &Viewport, max_ticks: usize) -> Vec<[f32; 2]> {
    if !(viewport.x_min < viewport.x_max) || !(viewport.y_min < viewport.y_max) {
        return Vec::new();
    }
    if ![
        viewport.x_min,
        viewport.x_max,
        viewport.y_min,
        viewport.y_max,
    ]
    .iter()
    .all(|v| v.is_finite())
    {
        return Vec::new();
    }
    let (maj_x, maj_y) = viewport_ticks(viewport, max_ticks);
    let (min_x, min_y) = viewport_minor_ticks(viewport, max_ticks);
    let mut out = Vec::with_capacity(2 * (maj_x.len() + min_x.len() + maj_y.len() + min_y.len()));
    let y0 = viewport.y_min as f32;
    let y1 = viewport.y_max as f32;
    let x0 = viewport.x_min as f32;
    let x1 = viewport.x_max as f32;
    for ticks in [&maj_x[..], &min_x[..]] {
        for &x in ticks {
            if !x.is_finite() {
                continue;
            }
            let fx = x as f32;
            if !fx.is_finite() {
                continue;
            }
            out.push([fx, y0]);
            out.push([fx, y1]);
        }
    }
    for ticks in [&maj_y[..], &min_y[..]] {
        for &y in ticks {
            if !y.is_finite() {
                continue;
            }
            let fy = y as f32;
            if !fy.is_finite() {
                continue;
            }
            out.push([x0, fy]);
            out.push([x1, fy]);
        }
    }
    out
}

fn collect_ticks(min: f64, max: f64, step: f64) -> Vec<f64> {
    let mut ticks = Vec::new();
    let mut v = (min / step).ceil() * step;
    // Guard against runaway loops on pathological ranges.
    let max_count = 10_000;
    while v <= max && ticks.len() < max_count {
        ticks.push(if v.abs() < step * 1e-9 { 0.0 } else { v });
        v += step;
        if !v.is_finite() {
            break;
        }
    }
    ticks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_cover_default_viewport() {
        let (xs, ys) = viewport_ticks(&Viewport::default(), 10);
        assert!(!xs.is_empty() && xs.len() <= 11);
        assert!(!ys.is_empty() && ys.len() <= 11);
        assert!(xs.contains(&0.0));
    }

    #[test]
    fn minors_subdivide_majors() {
        let majors = grid_ticks(-10.0, 10.0, 10);
        let minors = minor_ticks(-10.0, 10.0, 10);
        assert!(!majors.is_empty());
        assert!(minors.len() > majors.len());
        // Every minor falls strictly between two consecutive majors.
        for m in &minors {
            assert!(majors.iter().all(|mj| (m - mj).abs() > 1e-9));
            assert!(*m > -10.0 && *m < 10.0 || m.abs() <= 10.0);
        }
        // Minor spacing is an even fraction of the major step
        // (÷4 for a 2-mantissa step, else ÷5).
        let step = tick_step(-10.0, 10.0, 10);
        let minor_step = step / 4.0;
        assert!((minors[1] - minors[0] - minor_step).abs() < 1e-6);
    }

    #[test]
    fn zoom_sweep_relabels_monotonically() {
        // Halving the span never grows the step and keeps labels bounded.
        let mut prev_step = f64::INFINITY;
        let mut width = 20.0;
        for _ in 0..14 {
            let step = tick_step(-width / 2.0, width / 2.0, 10);
            assert!(step > 0.0);
            assert!(step <= prev_step, "step grew while zooming in");
            let ticks = grid_ticks(-width / 2.0, width / 2.0, 10);
            assert!(!ticks.is_empty() && ticks.len() <= 11);
            assert!(ticks.contains(&0.0));
            let minors = minor_ticks(-width / 2.0, width / 2.0, 10);
            assert!(!minors.is_empty());
            prev_step = step;
            width /= 2.0;
        }
    }

    #[test]
    fn degenerate_ranges_return_empty() {
        assert!(grid_ticks(5.0, -5.0, 10).is_empty());
        assert!(grid_ticks(1.0, 1.0, 10).is_empty());
        assert!(grid_ticks(0.0, 1.0, 0).is_empty());
        assert!(minor_ticks(f64::NAN, 1.0, 10).is_empty());
        assert_eq!(tick_step(0.0, 1.0, 0), 0.0);
    }

    #[test]
    fn labels_are_stable() {
        assert_eq!(format_tick(2.0, 0.0), "0");
        assert_eq!(format_tick(2.0, -0.0), "0");
        assert_eq!(format_tick(0.02, 0.06), "0.06");
        assert_eq!(format_tick(200.0, 400.0), "400");
        assert_eq!(tick_decimals(0.02), 2);
        assert_eq!(tick_decimals(2.0), 0);
    }

    #[test]
    fn grid_labels_mirror_ticks_with_stable_text() {
        let vp = Viewport::default();
        let (lx, ly) = grid_labels(&vp, 10);
        let (tx, ty) = viewport_ticks(&vp, 10);
        assert_eq!(lx.len(), tx.len());
        assert_eq!(ly.len(), ty.len());
        assert!(!lx.is_empty() && lx.len() <= 11);
        for (label, &value) in lx.iter().zip(tx.iter()) {
            assert!((label.value - value).abs() < 1e-12);
            let step = tick_step(vp.x_min, vp.x_max, 10);
            assert_eq!(label.text, format_tick(step, value));
        }
        // Zero snaps to "0", never "-0", at any step.
        let zero = lx.iter().find(|l| l.value == 0.0).expect("zero tick");
        assert_eq!(zero.text, "0");
    }

    #[test]
    fn grid_labels_cover_zoom_sweep_without_flicker() {
        let mut width = 20.0;
        for _ in 0..14 {
            let vp = Viewport::new(-width / 2.0, width / 2.0, -width / 2.0, width / 2.0);
            let (lx, ly) = grid_labels(&vp, 10);
            assert!(!lx.is_empty() && lx.len() <= 11);
            assert!(!ly.is_empty() && ly.len() <= 11);
            assert!(lx.iter().any(|l| l.value == 0.0 && l.text == "0"));
            // Step-appropriate decimals: reparsing the text round-trips.
            let step = tick_step(-width / 2.0, width / 2.0, 10);
            let decimals = tick_decimals(step);
            for l in lx.iter().chain(ly.iter()) {
                if l.text != "0" {
                    let dot = l.text.find('.');
                    match dot {
                        Some(i) => assert_eq!(l.text.len() - i - 1, decimals, "label {}", l.text),
                        None => assert_eq!(decimals, 0, "label {}", l.text),
                    }
                }
            }
            width /= 2.0;
        }
    }

    #[test]
    fn grid_vertices_emit_paired_endpoints_for_majors_and_minors() {
        let vp = Viewport::default();
        let verts = grid_vertices(&vp, 10);
        let (maj_x, maj_y) = viewport_ticks(&vp, 10);
        let (min_x, min_y) = viewport_minor_ticks(&vp, 10);
        let lines = maj_x.len() + min_x.len() + maj_y.len() + min_y.len();
        assert_eq!(verts.len(), 2 * lines);
        assert!(verts.len() % 2 == 0);
        // Vertical majors first: endpoints span the viewport height.
        for (i, &x) in maj_x.iter().enumerate() {
            let a = verts[2 * i];
            let b = verts[2 * i + 1];
            assert!(((a[0] as f64) - x).abs() < 1e-5, "x={x}");
            assert!(((b[0] as f64) - x).abs() < 1e-5, "x={x}");
            assert!((a[1] - vp.y_min as f32).abs() < 1e-6);
            assert!((b[1] - vp.y_max as f32).abs() < 1e-6);
        }
        // Minors follow majors per axis and stay strictly between majors.
        let minor_base = 2 * maj_x.len();
        for (j, &x) in min_x.iter().enumerate() {
            let a = verts[minor_base + 2 * j];
            assert!(((a[0] as f64) - x).abs() < 1e-5, "minor x={x}");
            assert!(maj_x.iter().all(|mj| (x - mj).abs() > 1e-9));
        }
        // Horizontal block spans the viewport width.
        let horiz_base = 2 * (maj_x.len() + min_x.len());
        for (k, &y) in maj_y.iter().enumerate() {
            let a = verts[horiz_base + 2 * k];
            let b = verts[horiz_base + 2 * k + 1];
            assert!(((a[1] as f64) - y).abs() < 1e-5, "y={y}");
            assert!(((b[1] as f64) - y).abs() < 1e-5, "y={y}");
            assert!((a[0] - vp.x_min as f32).abs() < 1e-6);
            assert!((b[0] - vp.x_max as f32).abs() < 1e-6);
        }
    }

    #[test]
    fn grid_vertices_reject_degenerate_viewports() {
        // Fully degenerate viewports yield nothing anywhere.
        let fully_bad = [
            Viewport::new(5.0, -5.0, 1.0, 1.0),
            Viewport::new(f64::NAN, 1.0, f64::NAN, 1.0),
            Viewport::new(0.0, f64::INFINITY, 0.0, f64::INFINITY),
        ];
        for vp in fully_bad {
            assert!(grid_vertices(&vp, 10).is_empty(), "vp={vp:?}");
            assert!(grid_labels(&vp, 10).0.is_empty());
            assert!(grid_labels(&vp, 10).1.is_empty());
        }
        // Partially degenerate: vertices still empty (no frame to draw),
        // while the healthy axis keeps its labels per-axis.
        let partial = Viewport::new(5.0, -5.0, 0.0, 1.0);
        assert!(grid_vertices(&partial, 10).is_empty());
        assert!(grid_labels(&partial, 10).0.is_empty());
        assert!(!grid_labels(&partial, 10).1.is_empty());
    }

    #[test]
    fn minor_division_rule_holds_per_mantissa() {
        // 2-mantissa step splits ÷4; 1/5-mantissa steps split ÷5.
        for (lo, hi, expect_divs) in [(-10.0, 10.0, 4), (-5.0, 5.0, 5), (-2.5, 2.5, 5)] {
            let step = tick_step(lo, hi, 10);
            let majors = grid_ticks(lo, hi, 10);
            let minors = minor_ticks(lo, hi, 10);
            assert!(!majors.is_empty() && !minors.is_empty());
            let minor_step = step / expect_divs as f64;
            assert!(
                (minors[1] - minors[0] - minor_step).abs() < 1e-6,
                "range {lo}..{hi}: step={step} minors={minors:?}"
            );
            for m in &minors {
                assert!(majors.iter().all(|mj| (m - mj).abs() > 1e-9));
            }
        }
    }
}
