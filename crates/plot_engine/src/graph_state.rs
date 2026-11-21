//! Shared graph state and atomic update helpers.
//!
//! The JS-facing API mutates this state, while the render loop consumes it.
//! Function replacements are applied to a candidate first and committed only on
//! success, so a failed expression can never replace the last valid curve.

use math_engine::Mathematician;
use wasm_bindgen::JsValue;

use crate::error::PlotError;

/// The function currently displayed and whether it needs re-uploading.
pub struct GraphState {
    pub changed: bool,
    function: Mathematician,
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
impl GraphState {
    /// Creates an empty graph with nothing pending.
    pub fn new() -> Self {
        Self {
            changed: false,
            function: Mathematician::new(String::new()),
        }
    }

    /// Returns the y coordinates of the active curve in clip space.
    pub fn curve_points(&self) -> Vec<f32> {
        self.function
            .points_normalised_function()
            .into_iter()
            .map(|point| point.y() as f32)
            .collect()
    }

    /// Replaces the active function only after the candidate fully validates.
    pub fn replace_function(
        &mut self,
        function: &str,
        start: f64,
        end: f64,
    ) -> Result<(), PlotError> {
        if function.trim().is_empty() {
            return Err(PlotError::EmptyFunction);
        }
        if !start.is_finite() || !end.is_finite() || start >= end {
            return Err(PlotError::InvalidRange);
        }

        self.replace_function_with(|| {
            let mut candidate = Mathematician::new(function.to_owned());
            candidate.to_points().map_err(math_error)?;
            Ok(candidate)
        })
    }

    /// Commits a candidate only when `build` succeeds.
    ///
    /// This seam lets host tests inject a failed candidate without a JS runtime;
    /// production updates enter through `replace_function`.
    fn replace_function_with(
        &mut self,
        build: impl FnOnce() -> Result<Mathematician, PlotError>,
    ) -> Result<(), PlotError> {
        let candidate = build()?;

        self.function = candidate;
        self.changed = true;
        Ok(())
    }

    /// Returns the active expression; used by tests to observe failed updates.
    #[cfg(test)]
    fn function_name(&self) -> String {
        self.function.function()
    }

    /// Pans and shifts the active function.
    ///
    /// The dirty flag is only set on success, so the last rendered curve stays
    /// visible when the operation fails.
    pub fn add_offset(&mut self, x_shift: f64, y_shift: f64) -> Result<(), PlotError> {
        if self.function.function().is_empty() {
            return Err(PlotError::NoFunctionLoaded);
        }

        self.function
            .add_offset(x_shift, y_shift)
            .map_err(math_error)?;
        self.changed = true;
        Ok(())
    }

    /// Scales the visible x range.
    ///
    /// The dirty flag is only set on success, so the last rendered curve stays
    /// visible when the operation fails.
    pub fn zoom(&mut self, factor: f64) -> Result<(), PlotError> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(PlotError::InvalidZoom);
        }
        if self.function.function().is_empty() {
            return Err(PlotError::NoFunctionLoaded);
        }

        self.function.zoom(factor).map_err(math_error)?;
        self.changed = true;
        Ok(())
    }
}

impl Default for GraphState {
    fn default() -> Self {
        Self::new()
    }
}

/// Captures the message from a `math_engine` error at the domain boundary.
fn math_error(error: JsValue) -> PlotError {
    #[cfg(target_arch = "wasm32")]
    {
        PlotError::Math(
            error
                .as_string()
                .unwrap_or_else(|| "unknown math engine error".to_owned()),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = error;
        PlotError::Math("math engine error".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::CURVE_SAMPLE_COUNT;

    #[test]
    fn new_state_has_no_pending_change_or_points() {
        let state = GraphState::new();

        assert!(!state.changed);
        assert!(state.curve_points().is_empty());
    }

    #[test]
    fn invalid_input_keeps_previous_state() {
        let mut state = GraphState::new();
        state.replace_function("x^2", -10.0, 10.0).unwrap();
        state.changed = false;

        let previous_function = state.function_name();
        let previous_points = state.curve_points();

        // Empty and whitespace-only expressions fail without disturbing the
        // last valid curve. Unparseable expressions such as `"sin("` exercise
        // the same atomic path but cannot run on host because `math_engine`
        // builds a `JsValue` for its error; the WASM browser probe covers it.
        assert_eq!(
            state.replace_function("   ", -5.0, 5.0),
            Err(PlotError::EmptyFunction)
        );

        assert!(!state.changed);
        assert_eq!(state.function_name(), previous_function);
        assert_eq!(state.curve_points(), previous_points);
    }

    #[test]
    fn failed_candidate_keeps_previous_state() {
        let mut state = GraphState::new();
        state.replace_function("x^2", -10.0, 10.0).unwrap();
        state.changed = false;

        let previous_function = state.function_name();
        let previous_points = state.curve_points();

        assert!(
            state
                .replace_function_with(|| Err(PlotError::Math("injected".to_owned())))
                .is_err()
        );

        assert!(!state.changed);
        assert_eq!(state.function_name(), previous_function);
        assert_eq!(state.curve_points(), previous_points);
    }

    #[test]
    fn replace_function_tracks_pending_changes() {
        let mut state = GraphState::new();
        assert!(!state.changed);

        state.replace_function("x^2", -10.0, 10.0).unwrap();
        assert!(state.changed);
        assert_eq!(state.curve_points().len(), CURVE_SAMPLE_COUNT);

        state.changed = false;
        assert!(!state.changed);
    }

    #[test]
    fn invalid_ranges_are_rejected_without_changing_state() {
        let mut state = GraphState::new();
        state.replace_function("x^2", -10.0, 10.0).unwrap();
        state.changed = false;

        let previous_function = state.function_name();
        let previous_points = state.curve_points();

        for (start, end) in [
            (f64::NAN, 10.0),
            (-10.0, f64::INFINITY),
            (10.0, -10.0),
            (0.0, 0.0),
        ] {
            assert_eq!(
                state.replace_function("x^2", start, end),
                Err(PlotError::InvalidRange)
            );
            assert!(!state.changed);
            assert_eq!(state.function_name(), previous_function);
            assert_eq!(state.curve_points(), previous_points);
        }
    }

    #[test]
    fn invalid_zoom_is_rejected_without_changing_state() {
        let mut state = GraphState::new();
        state.replace_function("x^2", -10.0, 10.0).unwrap();
        state.changed = false;

        let previous_points = state.curve_points();

        for factor in [0.0, -0.5, f64::NAN, f64::INFINITY] {
            assert_eq!(state.zoom(factor), Err(PlotError::InvalidZoom));
            assert!(!state.changed);
            assert_eq!(state.curve_points(), previous_points);
        }
    }

    #[test]
    fn offset_and_zoom_require_a_loaded_function() {
        let mut state = GraphState::new();

        assert_eq!(state.add_offset(1.0, 0.0), Err(PlotError::NoFunctionLoaded));
        assert_eq!(state.zoom(0.5), Err(PlotError::NoFunctionLoaded));
    }

    #[test]
    fn offset_and_zoom_change_points_and_mark_changed() {
        let mut state = GraphState::new();
        state.replace_function("x^2", -10.0, 10.0).unwrap();
        state.changed = false;

        let before_offset = state.curve_points();
        state.add_offset(1.0, 0.5).unwrap();
        assert!(state.changed);
        let after_offset = state.curve_points();
        assert_ne!(before_offset, after_offset);

        state.changed = false;
        state.zoom(0.5).unwrap();
        assert!(state.changed);
        let after_zoom = state.curve_points();
        assert_ne!(after_offset, after_zoom);
    }
}
