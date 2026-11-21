//! WebGPU plot renderer for the geogebra-rs frontend.
//!
//! The crate is split by responsibility:
//!
//! - `graph_state` owns the active function and applies JS updates atomically.
//! - `graph` converts normalised samples into tessellated curve geometry.
//! - `grid` builds the static grid geometry.
//! - `pipelines` defines the shared vertex format and render pipelines.
//! - `renderer` owns GPU resources and draws frames.
//! - `app` drives winit events and the web canvas.
//! - [`wasm_interface`] exposes the JavaScript API.
//!
//! The plot uses clip space `[-1, 1]` with 1000 samples per curve. Startup
//! shows only the grid; the curve appears after a successful `updateGraph`.

mod app;
mod graph;
mod graph_state;
mod grid;
mod pipelines;
mod renderer;
pub mod wasm_interface;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

pub use app::App;
pub use renderer::Renderer;
pub use wasm_interface::{add_offset, update_graph, update_graph_with_range, zoom};

use winit::event_loop::EventLoop;

/// Starts the native or web event loop.
pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();
    #[cfg(target_arch = "wasm32")]
    console_log::init_with_level(log::Level::Info).unwrap_throw();

    let event_loop = EventLoop::with_user_event().build()?;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App::new();
        event_loop.run_app(&mut app)?;
    }

    #[cfg(target_arch = "wasm32")]
    {
        let app = App::new(&event_loop);
        event_loop.spawn_app(app);
    }

    Ok(())
}

/// Installs the panic hook and starts the app when the WASM module loads.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}
