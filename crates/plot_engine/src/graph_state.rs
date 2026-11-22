//! Shared graph state and atomic update helpers.
//!
//! The JS-facing API mutates this state, while the render loop consumes it.
//! Function replacements are applied to a candidate first and committed only on
//! success, so a failed expression can never replace the last valid curve.

use math_engine::Mathematician;
use wasm_bindgen::JsValue;

use crate::error::PlotError;

pub struct Curve {
    pub id: u8,
    pub mathematician: Mathematician,
    pub color: [f32; 3],
}

/// The function currently displayed and whether it needs re-uploading.
pub struct GraphState {
    pub changed: bool,
    pub curves: Vec<Curve>,
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
impl GraphState {
    /// Creates an empty graph with nothing pending.
    pub fn new() -> Self {
        Self {
            changed: false,
            curves: Vec::new(),
        }
    }

    fn get_free_id(&self) -> Result<u8, PlotError> {
        for i in 0..u8::MAX {
            if self.curves.iter().find(|el| el.id == i).is_none() {
                return Ok(i);
            }
        }
        Err(PlotError::TooManyFunctions)
    }

    pub fn add_function(
        &mut self,
        build: impl FnOnce() -> Result<Mathematician, PlotError>,
        color: [f32; 3],
    ) -> Result<u8, PlotError> {
        let mathematician = build()?;
        let id = self.get_free_id()?;
        let new_curve = Curve {
            id,
            mathematician,
            color,
        };
        self.curves.push(new_curve);
        self.changed = true;
        Ok(id)
    }

    pub fn push_function(&mut self, mathematician: Mathematician, color: [f32; 3]) -> Result<u8, PlotError> {
        let id = self.get_free_id()?;
        let new_curve = Curve {
            id,
            mathematician,
            color
        };
        self.curves.push(new_curve);
        self.changed = true;
        Ok(id)
    }

    pub fn list_functions(&self) -> Vec<(u8, String)> {
        self.curves
            .iter()
            .map(|el| (el.id, el.mathematician.function()))
            .collect()
    }

    /// Pans and shifts the active function.
    ///
    /// The dirty flag is only set on success, so the last rendered curve stays
    /// visible when the operation fails.
    pub fn add_offset_all(&mut self, x_shift: f64, y_shift: f64) -> Result<(), PlotError> {
        if self.curves.is_empty() {
            return Err(PlotError::NoFunctionLoaded);
        }

        for el in self.curves.iter_mut() {
            el.mathematician
                .add_offset(x_shift, y_shift)
                .map_err(math_error)?;
        }

        self.changed = true;
        Ok(())
    }

    /// Scales the visible x range.
    ///
    /// The dirty flag is only set on success, so the last rendered curve stays
    /// visible when the operation fails.
    pub fn zoom_all(&mut self, factor: f64) -> Result<(), PlotError> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(PlotError::InvalidZoom);
        }

        for el in self.curves.iter_mut() {
            el.mathematician.zoom(factor).map_err(math_error)?;
        }
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

    fn mathematician(expr: &str) -> Mathematician {
        let mut value = Mathematician::new(expr.to_owned());
        value.set_start(-10.0);
        value.set_end(10.0);
        value.calc_derivatives().unwrap();
        value.to_points().unwrap();
        value
    }

    fn y_values(curve: &Curve) -> Vec<f32> {
        curve
            .mathematician
            .points_normalised_function()
            .into_iter()
            .map(|point| point.y() as f32)
            .collect()
    }

    fn insert_curve(state: &mut GraphState, id: u8) {
        state.curves.push(Curve {
            id,
            mathematician: Mathematician::new("x".to_owned()),
            color: [1.0, 0.0, 0.0],
        });
    }

    #[test]
    fn new_state_is_empty_and_clean() {
        let state = GraphState::new();

        assert!(!state.changed);
        assert!(state.curves.is_empty());
        assert!(state.list_functions().is_empty());
    }

    #[test]
    fn add_function_assigns_ids_colors_and_lists_functions() {
        let mut state = GraphState::new();
        let red = [1.0, 0.0, 0.0];
        let blue = [0.0, 0.0, 1.0];

        let first = state.add_function(|| Ok(mathematician("x")), red).unwrap();
        let second = state
            .add_function(|| Ok(mathematician("x^2")), blue)
            .unwrap();

        assert_eq!((first, second), (0, 1));
        assert_eq!(state.curves.len(), 2);
        assert_eq!(state.curves[0].id, first);
        assert_eq!(state.curves[0].color, red);
        assert_eq!(state.curves[1].id, second);
        assert_eq!(state.curves[1].color, blue);
        assert_eq!(
            state.list_functions(),
            vec![(0, "x".to_owned()), (1, "x^2".to_owned())]
        );
    }

    #[test]
    fn added_function_has_curve_samples() {
        let mut state = GraphState::new();

        let id = state
            .add_function(|| Ok(mathematician("x")), [1.0, 0.0, 0.0])
            .unwrap();

        assert_eq!(state.curves[0].id, id);
        assert_eq!(y_values(&state.curves[0]).len(), CURVE_SAMPLE_COUNT);
    }

    // Expected to fail until `add_function` marks the graph dirty.
    #[test]
    fn adding_a_function_marks_the_graph_dirty() {
        let mut state = GraphState::new();

        state
            .add_function(|| Ok(mathematician("x")), [1.0, 0.0, 0.0])
            .unwrap();

        assert!(state.changed);
    }

    #[test]
    fn push_function_assigns_id_color_and_lists_function() {
        let mut state = GraphState::new();
        let color = [0.25, 0.5, 0.75];

        let id = state.push_function(mathematician("x^2"), color).unwrap();

        assert_eq!(id, 0);
        assert_eq!(state.curves[0].id, id);
        assert_eq!(state.curves[0].color, color);
        assert_eq!(state.list_functions(), vec![(0, "x^2".to_owned())]);
    }

    // Expected to fail until `push_function` marks the graph dirty.
    #[test]
    fn push_function_marks_the_graph_dirty() {
        let mut state = GraphState::new();

        state
            .push_function(mathematician("x"), [1.0, 0.0, 0.0])
            .unwrap();

        assert!(state.changed);
    }

    #[test]
    fn add_function_error_does_not_mutate_state() {
        let mut state = GraphState::new();

        let result = state.add_function(
            || Err(PlotError::Math("injected".to_owned())),
            [1.0, 0.0, 0.0],
        );

        assert_eq!(result, Err(PlotError::Math("injected".to_owned())));
        assert!(!state.changed);
        assert!(state.curves.is_empty());
        assert!(state.list_functions().is_empty());
    }

    #[test]
    fn free_id_reuses_the_lowest_gap() {
        let mut state = GraphState::new();
        insert_curve(&mut state, 0);
        insert_curve(&mut state, 2);

        assert_eq!(state.get_free_id(), Ok(1));
    }

    // Expected to fail until `get_free_id` includes `u8::MAX` in its scan.
    #[test]
    fn free_id_includes_u8_max() {
        let mut state = GraphState::new();
        for id in 0..u8::MAX {
            insert_curve(&mut state, id);
        }

        assert_eq!(state.get_free_id(), Ok(u8::MAX));
    }

    #[test]
    fn add_offset_all_changes_every_curve_and_marks_changed() {
        let mut state = GraphState::new();
        state
            .add_function(|| Ok(mathematician("x")), [1.0, 0.0, 0.0])
            .unwrap();
        state
            .add_function(|| Ok(mathematician("x^2")), [0.0, 0.0, 1.0])
            .unwrap();
        state.changed = false;

        let before: Vec<Vec<f32>> = state.curves.iter().map(y_values).collect();
        state.add_offset_all(1.0, 0.5).unwrap();

        assert!(state.changed);
        let after: Vec<Vec<f32>> = state.curves.iter().map(y_values).collect();
        for (old, new) in before.iter().zip(after.iter()) {
            assert_ne!(old, new);
        }
        assert_eq!(
            state
                .list_functions()
                .into_iter()
                .map(|(_, name)| name)
                .collect::<Vec<_>>(),
            vec!["x".to_owned(), "x^2".to_owned()]
        );
    }

    #[test]
    fn add_offset_all_requires_a_function() {
        let mut state = GraphState::new();

        assert_eq!(
            state.add_offset_all(1.0, 0.0),
            Err(PlotError::NoFunctionLoaded)
        );
        assert!(!state.changed);
    }

    #[test]
    fn zoom_all_changes_every_curve_and_marks_changed() {
        let mut state = GraphState::new();
        state
            .add_function(|| Ok(mathematician("x")), [1.0, 0.0, 0.0])
            .unwrap();
        state
            .add_function(|| Ok(mathematician("x^2")), [0.0, 0.0, 1.0])
            .unwrap();
        state.changed = false;

        let before: Vec<Vec<f32>> = state.curves.iter().map(y_values).collect();
        state.zoom_all(0.5).unwrap();

        assert!(state.changed);
        let after: Vec<Vec<f32>> = state.curves.iter().map(y_values).collect();
        for (old, new) in before.iter().zip(after.iter()) {
            assert_ne!(old, new);
        }
    }

    // Expected to fail until `zoom_all` rejects an empty graph state.
    #[test]
    fn zoom_all_requires_a_function() {
        let mut state = GraphState::new();

        assert_eq!(state.zoom_all(0.5), Err(PlotError::NoFunctionLoaded));
        assert!(!state.changed);
    }

    #[test]
    fn zoom_all_rejects_invalid_factors_without_changing_state() {
        let mut state = GraphState::new();
        state
            .add_function(|| Ok(mathematician("x")), [1.0, 0.0, 0.0])
            .unwrap();
        state.changed = false;
        let before = y_values(&state.curves[0]);

        for factor in [0.0, -0.5, f64::NAN, f64::INFINITY] {
            assert_eq!(state.zoom_all(factor), Err(PlotError::InvalidZoom));
            assert!(!state.changed);
            assert_eq!(y_values(&state.curves[0]), before);
        }
    }
}
