#![cfg(target_arch = "wasm32")]

//! JavaScript-facing WASM API.
//!
//! `PlotEngine` is the single owner of the graph state from JavaScript's point
//! of view. Its methods update that state and ask the winit event loop to
//! redraw; the renderer lives inside the event loop and never touches a global.
//! Every update follows `math_engine`'s 1000-sample contract.
//!
//! Errors cross the boundary as JavaScript `Error` objects. `error.name` is the
//! stable `PlotError::code` value and `error.message` is the human-readable
//! description.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use winit::event_loop::EventLoop;
use winit::event_loop::EventLoopProxy;
use winit::platform::web::EventLoopExtWebSys;

use crate::app::{App, UserEvent};
use crate::error::PlotError;
use crate::graph_state::GraphState;

/// Handle that owns the JS-visible graph state and drives the plot window.
#[wasm_bindgen]
pub struct PlotEngine {
    graph_state: Rc<RefCell<GraphState>>,
    proxy: EventLoopProxy<UserEvent>,
}

#[wasm_bindgen]
impl PlotEngine {
    /// Creates the engine and starts the winit event loop for the canvas.
    ///
    /// Call this once per page. `canvasId` is the DOM id of the canvas element.
    #[wasm_bindgen(constructor)]
    #[allow(non_snake_case)]
    pub fn new(canvasId: &str) -> Result<PlotEngine, PlotError> {
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Info);

        let window = web_sys::window().ok_or(PlotError::WindowUnavailable)?;
        let document = window.document().ok_or(PlotError::DocumentUnavailable)?;
        let canvas = document
            .get_element_by_id(canvasId)
            .ok_or_else(|| PlotError::CanvasNotFound(canvasId.to_owned()))?;
        let canvas = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| PlotError::NotACanvas)?;

        let event_loop = EventLoop::<UserEvent>::with_user_event()
            .build()
            .map_err(|error| PlotError::EventLoop(error.to_string()))?;
        let proxy = event_loop.create_proxy();
        let graph_state = Rc::new(RefCell::new(GraphState::new()));

        let app = App::new(proxy.clone(), Rc::clone(&graph_state), canvas);
        event_loop.spawn_app(app);

        Ok(Self { graph_state, proxy })
    }

    /// Updates the graph using the default `[-10, 10]` x range.
    #[wasm_bindgen(js_name = "updateGraph")]
    pub fn update_graph(&mut self, function: &str) -> Result<(), PlotError> {
        self.with_state(|state| state.replace_function(function, -10.0, 10.0))?;
        self.request_redraw();
        Ok(())
    }

    /// Replaces the graph with a function evaluated over `[start, end]`.
    ///
    /// `start` and `end` must be finite with `start < end`; success produces
    /// 1000 samples. Invalid input leaves the last valid curve untouched.
    #[wasm_bindgen(js_name = "updateGraphWithRange")]
    pub fn update_graph_with_range(
        &mut self,
        function: &str,
        start: f64,
        end: f64,
    ) -> Result<(), PlotError> {
        self.with_state(|state| state.replace_function(function, start, end))?;
        self.request_redraw();
        Ok(())
    }

    /// Pans and shifts the active graph.
    ///
    /// `x_shift` is in function-domain x units; `y_shift` is in clip-space y
    /// units. Returns an error when no function is loaded or the operation
    /// fails.
    #[wasm_bindgen(js_name = "addOffset")]
    pub fn add_offset(&mut self, x_shift: f32, y_shift: f32) -> Result<(), PlotError> {
        self.with_state(|state| state.add_offset(x_shift.into(), y_shift.into()))?;
        self.request_redraw();
        Ok(())
    }

    /// Scales the visible x range about its centre.
    ///
    /// `factor > 1` zooms out, `0 < factor < 1` zooms in, and non-finite or
    /// non-positive factors are rejected. Returns an error when no function is
    /// loaded.
    #[wasm_bindgen(js_name = "zoom")]
    pub fn zoom(&mut self, factor: f32) -> Result<(), PlotError> {
        self.with_state(|state| state.zoom(factor.into()))?;
        self.request_redraw();
        Ok(())
    }

    /// Borrows the graph state once per operation.
    fn with_state<T>(
        &self,
        update: impl FnOnce(&mut GraphState) -> Result<T, PlotError>,
    ) -> Result<T, PlotError> {
        let mut state = self
            .graph_state
            .try_borrow_mut()
            .map_err(|_| PlotError::StateBusy)?;
        update(&mut state)
    }

    /// Wakes the event loop so the renderer picks up the pending change.
    fn request_redraw(&self) {
        if self.proxy.send_event(UserEvent::Redraw).is_err() {
            log::error!("event loop closed; cannot request a redraw");
        }
    }
}
