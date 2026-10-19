//! `render`: viewport math, curve sampling, grid ticks, wgpu pipeline.
//!
//! Pipeline stage: `Sample -> Submit to GPU`. [`pipeline::Renderer`] owns the
//! per-frame canvas-loop state: validated viewport, GPU uniform, revision
//! counter, and the 60 FPS ledger. Canvas drawing consumes
//! [`curve::sample_curve`] points on the CPU fallback and GPU vertex buffers
//! once [`pipeline::Renderer::init_gpu`] acquires a device.

pub mod curve;
pub mod grid;
pub mod pipeline;
pub mod viewport;

pub use curve::{
    CURVE_VERTEX_STRIDE, CurveVertex, DEFAULT_WIDTH_PX, FunctionStyle, MAX_POINTS, MAX_WIDTH_PX,
    MIN_WIDTH_PX, PALETTE_HEXES, POLE_BREAK, TRACER_HEX, clamp_width, default_style, expand_ribbon,
    for_function_index, hex_to_linear, sample_curve, sample_curve_adaptive, sample_points_f32,
};
pub use grid::{format_tick, grid_ticks, minor_ticks, tick_decimals, tick_step};
pub use pipeline::{FRAME_BUDGET_MS, FrameStats, GpuStatus, Renderer, ViewportUniform};
pub use viewport::Viewport;
