//! wgpu pipeline with WebGL2 fallback and the viewport render loop (AD-4, R4).
//!
//! Init requests [`Backends::PRIMARY`] first and silently retries on a
//! [`Backends::GL`] (WebGL2) adapter when no WebGPU adapter is found. With no
//! GPU at all — CI containers, SSR — the renderer degrades to
//! [`GpuStatus::CpuFallback`] and the Svelte 2D-canvas path keeps drawing, so
//! panning and zooming never blank the viewport.
//!
//! [`Renderer`] owns the per-frame state the canvas loop needs: the validated
//! viewport, its GPU uniform, a revision counter so the loop can skip
//! redundant uniform uploads, and a [`FrameStats`] ledger proving the 60 FPS
//! (< 16 ms) drag/zoom baseline.

use std::collections::VecDeque;
use std::mem;

use crate::viewport::Viewport;

// ---------------------------------------------------------------------------
// Backend selection
// ---------------------------------------------------------------------------

/// GPU pipeline readiness reported to diagnostics and the Svelte status banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuStatus {
    /// No GPU adapter: the CPU-sampled 2D fallback draws the frame.
    CpuFallback,
    /// WebGPU device active (native Vulkan/Metal/DX12 or browser WebGPU).
    WebGpu,
    /// WebGL2 fallback device active.
    WebGl2,
}

impl GpuStatus {
    /// Wire-stable name used in the bridge JSON and the frontend banner.
    pub fn as_str(self) -> &'static str {
        match self {
            GpuStatus::CpuFallback => "cpu-fallback",
            GpuStatus::WebGpu => "webgpu",
            GpuStatus::WebGl2 => "webgl2",
        }
    }

    /// Parse a wire name back; unknown input maps to [`GpuStatus::CpuFallback`].
    pub fn from_str(s: &str) -> Self {
        match s {
            "webgpu" => GpuStatus::WebGpu,
            "webgl2" => GpuStatus::WebGl2,
            _ => GpuStatus::CpuFallback,
        }
    }
}

/// Full backend mask requested at init: WebGPU-class plus GL (AD-4).
pub fn requested_backends() -> wgpu::Backends {
    wgpu::Backends::PRIMARY | wgpu::Backends::GL
}

/// Probe order: WebGPU first, silent retry on GL second (AD-4).
pub fn backend_attempts() -> [wgpu::Backends; 2] {
    [wgpu::Backends::PRIMARY, wgpu::Backends::GL]
}

/// Classify an acquired adapter's backend into a pipeline status.
pub fn classify_backend(backend: wgpu::Backend) -> GpuStatus {
    match backend {
        wgpu::Backend::Gl => GpuStatus::WebGl2,
        wgpu::Backend::Noop => GpuStatus::CpuFallback,
        _ => GpuStatus::WebGpu,
    }
}

/// Build a display-less instance for one probe attempt.
///
/// Native hosts have no window at probe time; the canvas surface attaches
/// later in the wasm `init_gpu_canvas` path (story 1.2 follow-up owns the
/// surface once `wasm-pack` links the bundle).
fn instance_for(backends: wgpu::Backends) -> wgpu::Instance {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backends;
    wgpu::Instance::new(desc)
}

/// One backend probe: which mask was tried and whether an adapter answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeAttempt {
    pub backends: wgpu::Backends,
    pub adapter_found: bool,
}

/// Probe each backend in [`backend_attempts`] order without creating devices.
///
/// Runs the real `request_adapter` calls, so on a GPU-less host both attempts
/// report `adapter_found: false` and the caller degrades silently.
pub async fn probe_backends() -> [ProbeAttempt; 2] {
    let attempts = backend_attempts();
    let mut out = [ProbeAttempt {
        backends: attempts[0],
        adapter_found: false,
    }; 2];
    for (i, backends) in attempts.iter().enumerate() {
        let instance = instance_for(*backends);
        let found = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                ..Default::default()
            })
            .await
            .is_ok();
        out[i] = ProbeAttempt {
            backends: *backends,
            adapter_found: found,
        };
    }
    out
}

// ---------------------------------------------------------------------------
// Shaders and vertex layout
// ---------------------------------------------------------------------------

/// Curve line-strip shader (math-space points, viewport uniform).
pub const CURVE_SHADER: &str = include_str!("../shaders/curve.wgsl");

/// Grid-line shader sharing the viewport uniform layout.
pub const GRID_SHADER: &str = include_str!("../shaders/grid.wgsl");

/// Vertex layout for sampled curve points: one `vec2<f32>` math point.
pub fn curve_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 0,
            shader_location: 0,
        }],
    }
}

// ---------------------------------------------------------------------------
// Viewport uniform
// ---------------------------------------------------------------------------

/// GPU mirror of the viewport: `ndc = (world - center) * scale`.
///
/// NDC y stays up like math y; the surface blit owns the canvas flip.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportUniform {
    pub center: [f32; 2],
    pub scale: [f32; 2],
}

impl ViewportUniform {
    pub fn from_viewport(v: &Viewport) -> Self {
        let w = v.width().max(f64::MIN_POSITIVE);
        let h = v.height().max(f64::MIN_POSITIVE);
        Self {
            center: [
                ((v.x_min + v.x_max) / 2.0) as f32,
                ((v.y_min + v.y_max) / 2.0) as f32,
            ],
            scale: [(2.0 / w) as f32, (2.0 / h) as f32],
        }
    }

    /// CPU mirror of the vertex-shader transform, for tests and the fallback.
    pub fn world_to_ndc(&self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.center[0] as f64) * self.scale[0] as f64,
            (y - self.center[1] as f64) * self.scale[1] as f64,
        )
    }
}

// ---------------------------------------------------------------------------
// Frame budget ledger
// ---------------------------------------------------------------------------

/// 60 FPS frame budget: every frame must land under 16 ms during drag/zoom.
pub const FRAME_BUDGET_MS: f32 = 16.0;

/// Rolling frame-time ledger backing the 60 FPS baseline readout.
#[derive(Debug, Clone)]
pub struct FrameStats {
    samples: VecDeque<f32>,
    capacity: usize,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self::new(120)
    }
}

impl FrameStats {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(capacity.max(1)),
            capacity: capacity.max(1),
        }
    }

    /// Record one presented frame. Non-finite or negative deltas are ignored.
    pub fn push(&mut self, dt_ms: f32) {
        if !dt_ms.is_finite() || dt_ms < 0.0 {
            return;
        }
        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(dt_ms);
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Mean frame time over the window; `None` before the first frame.
    pub fn avg_ms(&self) -> Option<f32> {
        if self.samples.is_empty() {
            return None;
        }
        Some(self.samples.iter().sum::<f32>() / self.samples.len() as f32)
    }

    /// Worst frame in the window; `None` before the first frame.
    pub fn worst_ms(&self) -> Option<f32> {
        self.samples.iter().copied().reduce(f32::max)
    }

    /// Implied frame rate from the mean; `None` before the first frame.
    pub fn fps(&self) -> Option<f32> {
        self.avg_ms().map(|avg| 1000.0 / avg.max(f32::MIN_POSITIVE))
    }

    /// Baseline verdict: mean frame time within the 16 ms budget.
    pub fn meets_budget(&self) -> bool {
        self.avg_ms().is_some_and(|avg| avg <= FRAME_BUDGET_MS)
    }
}

// ---------------------------------------------------------------------------
// Renderer: per-frame state for the canvas loop
// ---------------------------------------------------------------------------

/// Canvas-loop state: viewport, uniform, revision, GPU handles, frame ledger.
///
/// Starts in [`GpuStatus::CpuFallback`]; [`Renderer::init_gpu`] upgrades to a
/// real device when an adapter answers. Every method is safe without a GPU.
pub struct Renderer {
    status: GpuStatus,
    viewport: Viewport,
    uniform: ViewportUniform,
    canvas_px: (u32, u32),
    revision: u64,
    frames: FrameStats,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
}

impl Renderer {
    pub fn new(viewport: Viewport) -> Self {
        Self {
            uniform: ViewportUniform::from_viewport(&viewport),
            viewport,
            status: GpuStatus::CpuFallback,
            canvas_px: (800, 600),
            revision: 0,
            frames: FrameStats::default(),
            device: None,
            queue: None,
        }
    }

    pub fn status(&self) -> GpuStatus {
        self.status
    }

    pub fn viewport(&self) -> Viewport {
        self.viewport
    }

    pub fn uniform(&self) -> ViewportUniform {
        self.uniform
    }

    /// Monotonic counter bumped by [`Renderer::update_viewport`]; the loop
    /// skips uniform re-upload while this is unchanged.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn canvas_px(&self) -> (u32, u32) {
        self.canvas_px
    }

    pub fn frames(&self) -> &FrameStats {
        &self.frames
    }

    pub fn has_device(&self) -> bool {
        self.device.is_some()
    }

    /// Validated viewport update: recomputes the uniform and bumps the
    /// revision. Degenerate or non-finite ranges are rejected and the
    /// previous viewport is kept, so the loop never renders a blank frame.
    pub fn update_viewport(&mut self, next: Viewport) -> bool {
        if !(next.x_min < next.x_max) || !(next.y_min < next.y_max) {
            return false;
        }
        if ![next.x_min, next.x_max, next.y_min, next.y_max]
            .iter()
            .all(|v| v.is_finite())
        {
            return false;
        }
        self.viewport = next;
        self.uniform = ViewportUniform::from_viewport(&next);
        self.revision += 1;
        true
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w > 0 && h > 0 {
            self.canvas_px = (w, h);
        }
    }

    /// Record a presented frame's wall time for the 60 FPS ledger.
    pub fn record_frame(&mut self, dt_ms: f32) {
        self.frames.push(dt_ms);
    }

    /// Acquire a device: WebGPU first, silent retry on WebGL2, CPU fallback
    /// when nothing answers. Never panics and never surfaces an error — a
    /// missing GPU is an expected host condition, not a failure.
    pub async fn init_gpu(&mut self) {
        for backends in backend_attempts() {
            let instance = instance_for(backends);
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    ..Default::default()
                })
                .await;
            let Ok(adapter) = adapter else { continue };
            let backend = adapter.get_info().backend;
            let Ok((device, queue)) = adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await
            else {
                continue;
            };
            self.status = classify_backend(backend);
            self.device = Some(device);
            self.queue = Some(queue);
            return;
        }
        self.status = GpuStatus::CpuFallback;
    }

    /// Blocking wrapper for native hosts and tests.
    pub fn init_gpu_blocking(&mut self) {
        pollster::block_on(self.init_gpu());
    }
}

/// Tracer status: [`GpuStatus::CpuFallback`] until [`Renderer::init_gpu`] runs.
pub fn status() -> GpuStatus {
    GpuStatus::CpuFallback
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_primary_and_gl_in_order() {
        assert_eq!(requested_backends(), wgpu::Backends::PRIMARY | wgpu::Backends::GL);
        assert_eq!(backend_attempts(), [wgpu::Backends::PRIMARY, wgpu::Backends::GL]);
    }

    #[test]
    fn classifies_adapter_backends() {
        assert_eq!(classify_backend(wgpu::Backend::Gl), GpuStatus::WebGl2);
        assert_eq!(classify_backend(wgpu::Backend::BrowserWebGpu), GpuStatus::WebGpu);
        assert_eq!(classify_backend(wgpu::Backend::Vulkan), GpuStatus::WebGpu);
        assert_eq!(classify_backend(wgpu::Backend::Metal), GpuStatus::WebGpu);
        assert_eq!(classify_backend(wgpu::Backend::Dx12), GpuStatus::WebGpu);
        assert_eq!(classify_backend(wgpu::Backend::Noop), GpuStatus::CpuFallback);
    }

    #[test]
    fn status_wire_names_round_trip() {
        for s in [GpuStatus::CpuFallback, GpuStatus::WebGpu, GpuStatus::WebGl2] {
            assert_eq!(GpuStatus::from_str(s.as_str()), s);
        }
        assert_eq!(GpuStatus::from_str("nonsense"), GpuStatus::CpuFallback);
    }

    #[test]
    fn uniform_maps_viewport_corners_to_ndc() {
        let u = ViewportUniform::from_viewport(&Viewport::default());
        let (x1, y1) = u.world_to_ndc(10.0, 10.0);
        let (x0, y0) = u.world_to_ndc(-10.0, -10.0);
        let (cx, cy) = u.world_to_ndc(0.0, 0.0);
        assert!((x1 - 1.0).abs() < 1e-6);
        assert!((y1 - 1.0).abs() < 1e-6);
        assert!((x0 + 1.0).abs() < 1e-6);
        assert!((y0 + 1.0).abs() < 1e-6);
        assert!(cx.abs() < 1e-6 && cy.abs() < 1e-6);
    }

    #[test]
    fn uniform_is_sixteen_bytes() {
        assert_eq!(mem::size_of::<ViewportUniform>(), 16);
    }

    #[test]
    fn vertex_layout_matches_shader_input() {
        let layout = curve_vertex_layout();
        assert_eq!(layout.array_stride, 8);
        assert_eq!(layout.attributes.len(), 1);
        assert_eq!(layout.attributes[0].format, wgpu::VertexFormat::Float32x2);
    }

    #[test]
    fn shaders_parse_as_wgsl() {
        let curve = naga::front::wgsl::parse_str(CURVE_SHADER);
        assert!(curve.is_ok(), "curve.wgsl: {curve:?}");
        let grid = naga::front::wgsl::parse_str(GRID_SHADER);
        assert!(grid.is_ok(), "grid.wgsl: {grid:?}");
    }

    #[test]
    fn frame_stats_reports_fps_and_budget() {
        let mut stats = FrameStats::new(120);
        assert!(stats.avg_ms().is_none());
        assert!(!stats.meets_budget());
        for _ in 0..60 {
            stats.push(16.0);
        }
        assert!((stats.avg_ms().unwrap() - 16.0).abs() < 1e-6);
        assert!((stats.fps().unwrap() - 62.5).abs() < 1e-3);
        assert!(stats.meets_budget());
        stats.push(100.0);
        assert_eq!(stats.worst_ms(), Some(100.0));
        assert!(!stats.meets_budget());
    }

    #[test]
    fn frame_stats_ignores_garbage_deltas() {
        let mut stats = FrameStats::default();
        stats.push(f32::NAN);
        stats.push(-5.0);
        stats.push(f32::INFINITY);
        assert!(stats.is_empty());
    }

    #[test]
    fn update_viewport_path_refreshes_uniform_and_revision() {
        let mut r = Renderer::new(Viewport::default());
        assert_eq!(r.status(), GpuStatus::CpuFallback);
        assert_eq!(r.revision(), 0);
        let next = Viewport::new(-5.0, 5.0, -2.0, 2.0);
        assert!(r.update_viewport(next));
        assert_eq!(r.viewport(), next);
        assert_eq!(r.revision(), 1);
        let (x1, _) = r.uniform().world_to_ndc(5.0, 0.0);
        assert!((x1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn update_viewport_rejects_degenerate_ranges() {
        let mut r = Renderer::new(Viewport::default());
        assert!(!r.update_viewport(Viewport::new(5.0, -5.0, 0.0, 1.0)));
        assert!(!r.update_viewport(Viewport::new(0.0, 1.0, 1.0, 1.0)));
        assert!(!r.update_viewport(Viewport::new(
            f64::NAN,
            1.0,
            0.0,
            1.0
        )));
        assert_eq!(r.viewport(), Viewport::default());
        assert_eq!(r.revision(), 0);
    }

    #[test]
    fn resize_ignores_zero_extents() {
        let mut r = Renderer::new(Viewport::default());
        r.resize(1280, 720);
        assert_eq!(r.canvas_px(), (1280, 720));
        r.resize(0, 720);
        assert_eq!(r.canvas_px(), (1280, 720));
    }

    #[test]
    fn headless_init_degrades_silently_without_gpu() {
        // GPU-less CI/container: both probes miss, renderer stays usable.
        // On a GPU host the same call upgrades to WebGpu/WebGl2; either way
        // it must not panic and the viewport path must keep working.
        let probes = pollster::block_on(probe_backends());
        assert_eq!(probes.len(), 2);
        assert_eq!(probes[0].backends, wgpu::Backends::PRIMARY);
        assert_eq!(probes[1].backends, wgpu::Backends::GL);

        let mut r = Renderer::new(Viewport::default());
        r.init_gpu_blocking();
        assert!(matches!(
            r.status(),
            GpuStatus::CpuFallback | GpuStatus::WebGpu | GpuStatus::WebGl2
        ));
        if r.status() == GpuStatus::CpuFallback {
            assert!(!r.has_device());
            assert!(!probes.iter().any(|p| p.adapter_found));
        } else {
            assert!(r.has_device());
        }
        assert!(r.update_viewport(Viewport::new(-4.0, 4.0, -4.0, 4.0)));
    }

    #[test]
    fn per_frame_cpu_work_stays_far_under_budget() {
        // The loop's CPU share per frame: viewport validate + uniform build.
        // 600 back-to-back updates must cost a fraction of one 16 ms frame.
        let mut r = Renderer::new(Viewport::default());
        let start = std::time::Instant::now();
        for i in 0..600 {
            let w = 20.0 + (i as f64) * 0.001;
            r.update_viewport(Viewport::new(-w / 2.0, w / 2.0, -10.0, 10.0));
        }
        assert!(start.elapsed().as_millis() < 16, "viewport path too slow");
        assert_eq!(r.revision(), 600);
    }
}
