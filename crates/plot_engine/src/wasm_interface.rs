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

use math_engine::Mathematician;
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

    #[wasm_bindgen(js_name = "addToGraph")]
    pub fn add_to_graph(&mut self, mathematician: Mathematician) -> Result<(), PlotError> {
        self.with_state(|state| state.push_function(mathematician, [1.0, 0.0, 0.0]))?;
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
