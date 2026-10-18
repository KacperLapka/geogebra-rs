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
}
