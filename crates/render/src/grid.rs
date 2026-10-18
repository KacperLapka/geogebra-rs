//! Coordinate grid tick positions (tracer).
//!
//! Log-subdivision relabelling during zoom lands in the adaptive-sampler follow-up; this pass
//! emits human-friendly ("1/2/5") steps bounded to `max_ticks`.

use crate::viewport::Viewport;

/// Nice tick positions for one axis range.
pub fn grid_ticks(min: f64, max: f64, max_ticks: usize) -> Vec<f64> {
    if !(min < max) || max_ticks == 0 {
        return Vec::new();
    }
    let span = max - min;
    let raw = span / max_ticks as f64;
    let mag = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|m| m * mag)
        .find(|s| span / s <= max_ticks as f64)
        .unwrap_or(10.0 * mag);
    let mut ticks = Vec::new();
    let mut v = (min / step).ceil() * step;
    // snap tiny residues to zero for stable labels
    while v <= max {
        ticks.push(if v.abs() < step * 1e-9 { 0.0 } else { v });
        v += step;
    }
    ticks
}

/// Tick positions for both axes of a viewport.
pub fn viewport_ticks(viewport: &Viewport, max_ticks: usize) -> (Vec<f64>, Vec<f64>) {
    (
        grid_ticks(viewport.x_min, viewport.x_max, max_ticks),
        grid_ticks(viewport.y_min, viewport.y_max, max_ticks),
    )
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
}
