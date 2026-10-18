//! `render`: viewport math, curve sampling, grid ticks.
//!
//! Pipeline stage: `Sample -> Submit to GPU`. The `wgpu` device/surface init
//! (with WebGL2 fallback) lands in the GPU-pipeline follow-up; the tracer keeps this crate
//! dependency-free and natively testable. Canvas drawing in the tracer
//! frontend consumes [`curve::sample_curve`] points.

pub mod curve;
pub mod grid;
pub mod pipeline;
pub mod viewport;

pub use curve::sample_curve;
pub use grid::grid_ticks;
pub use pipeline::GpuStatus;
pub use viewport::Viewport;
