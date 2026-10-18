//! GPU pipeline stub (AD-4).
//!
//! The GPU-pipeline follow-up implements `wgpu` device/surface init requesting
//! `Backends::PRIMARY | Backends::GL`, with silent retry on a WebGL2
//! adapter when no WebGPU adapter is found. The tracer draws sampled
//! polylines on a 2D canvas context; geometry moves to quad-ribbon
//! triangle strips in `shaders/curve.wgsl` with the GPU/sampler follow-ups.

/// GPU pipeline readiness reported to diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuStatus {
    /// Tracer mode: CPU-sampled polyline, no GPU device yet.
    CpuFallback,
    /// WebGPU device active (follow-up).
    WebGpu,
    /// WebGL2 fallback device active (follow-up).
    WebGl2,
}

/// Tracer status: always [`GpuStatus::CpuFallback`] when the GPU pipeline lands.
pub fn status() -> GpuStatus {
    GpuStatus::CpuFallback
}
