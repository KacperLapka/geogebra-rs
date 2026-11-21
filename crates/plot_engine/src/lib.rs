//! WebGPU plot renderer for the geogebra-rs frontend.
//!
//! The crate is split by responsibility:
//!
//! - `graph_state` owns the active function and validates updates atomically.
//! - `error` defines typed failures converted to JavaScript `Error` objects.
//! - `graph` converts normalised samples into tessellated curve geometry.
//! - `grid` builds the static grid geometry.
//! - `pipelines` defines the shared vertex format and render pipelines.
//! - `renderer` owns GPU resources and draws frames.
//! - `app` drives winit events and the web canvas.
//! - `wasm_interface` exposes the `PlotEngine` instance to JavaScript.
//!
//! The plot uses clip space `[-1, 1]` with 1000 samples per curve. Startup
//! shows only the grid; the curve appears after a successful `updateGraph`.
//! The web build creates a `PlotEngine` instance; native builds call `run`.

mod app;
mod error;
mod graph;
mod graph_state;
mod grid;
mod pipelines;
mod renderer;

#[cfg(target_arch = "wasm32")]
pub mod wasm_interface;

#[cfg(not(target_arch = "wasm32"))]
use winit::event_loop::EventLoop;

#[cfg(not(target_arch = "wasm32"))]
use crate::app::{App, UserEvent};

/// Starts the native event loop.
#[cfg(not(target_arch = "wasm32"))]
pub fn run() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}
