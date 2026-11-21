//! GPU resources, surface configuration, and frame rendering.

use std::sync::Arc;

use lyon_tessellation::VertexBuffers;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::graph;
use crate::graph_state::GRAPH_STATE;
use crate::grid;
use crate::pipelines::{self, Vertex};

/// Largest surface dimension accepted by [`Renderer::resize`].
const MAX_SURFACE_DIMENSION: u32 = 2048;

/// Returns the clamped surface size, or `None` when either dimension is zero.
fn surface_dimensions(width: u32, height: u32) -> Option<(u32, u32)> {
    if width == 0 || height == 0 {
        return None;
    }

    Some((
        width.min(MAX_SURFACE_DIMENSION),
        height.min(MAX_SURFACE_DIMENSION),
    ))
}

/// GPU buffers for one tessellated curve.
struct GraphBuffers {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl GraphBuffers {
    /// Uploads tessellated geometry, or returns `None` when there is nothing to draw.
    fn upload(device: &wgpu::Device, geometry: &VertexBuffers<Vertex, u16>) -> Option<Self> {
        if geometry.vertices.is_empty() || geometry.indices.is_empty() {
            return None;
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Graph Vertex Buffer"),
            contents: bytemuck::cast_slice(&geometry.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Graph Index Buffer"),
            contents: bytemuck::cast_slice(&geometry.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Some(Self {
            vertex_buffer,
            index_buffer,
            index_count: geometry.indices.len() as u32,
        })
    }
}

/// Owns the window surface, render pipelines, and uploadable graph buffers.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    /// Window driving the event loop; public so `App` can request redraws.
    pub window: Arc<Window>,
    grid_pipeline: wgpu::RenderPipeline,
    grid_vertex_buffer: wgpu::Buffer,
    grid_vertex_count: u32,
    graph_pipeline: wgpu::RenderPipeline,
    graph_buffers: Option<GraphBuffers>,
}

impl Renderer {
    /// Creates a renderer for `window` with an empty plot (grid only).
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let instance = Self::create_instance();
        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: true,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                // WebGL doesn't support all of wgpu's features.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let (width, height) = surface_dimensions(size.width, size.height).unwrap_or((1, 1));
        let config = Self::surface_configuration(&surface, &adapter, width, height);

        let grid_pipeline = pipelines::create_pipeline(
            &device,
            config.format,
            wgpu::PrimitiveTopology::LineList,
            "Grid Pipeline",
        );
        let graph_pipeline = pipelines::create_pipeline(
            &device,
            config.format,
            wgpu::PrimitiveTopology::TriangleList,
            "Graph Pipeline",
        );

        let grid_vertices = grid::build_grid_vertices();
        let grid_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Vertex Buffer"),
            contents: bytemuck::cast_slice(&grid_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            grid_pipeline,
            grid_vertex_buffer,
            grid_vertex_count: grid_vertices.len() as u32,
            graph_pipeline,
            graph_buffers: None,
        })
    }

    /// Creates the wgpu instance with the backend supported by the target.
    fn create_instance() -> wgpu::Instance {
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        })
    }

    /// Chooses an sRGB surface format and the matching configuration.
    fn surface_configuration(
        surface: &wgpu::Surface<'_>,
        adapter: &wgpu::Adapter,
        width: u32,
        height: u32,
    ) -> wgpu::SurfaceConfiguration {
        let capabilities = surface.get_capabilities(adapter);
        let format = capabilities
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(capabilities.formats[0]);

        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: capabilities.present_modes[0],
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        }
    }

    /// Reconfigures the surface after a non-zero window resize.
    pub fn resize(&mut self, width: u32, height: u32) {
        let Some((width, height)) = surface_dimensions(width, height) else {
            return;
        };

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.is_surface_configured = true;
    }

    /// Rebuilds the curve buffers when the graph state has a pending change.
    pub fn update(&mut self) {
        let changed = GRAPH_STATE.with_borrow(|state| state.has_changed());
        if !changed {
            return;
        }

        let points = GRAPH_STATE.with_borrow(|state| state.curve_points());
        self.graph_buffers = graph::points_raw_to_path(&points)
            .map(|path| graph::tessellate_curve(&path))
            .and_then(|geometry| GraphBuffers::upload(&self.device, &geometry));

        GRAPH_STATE.with_borrow_mut(|state| state.clear_changed());
    }

    /// Draws one frame and presents it to the surface.
    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame.
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // Recreating all GPU resources is out of scope; bail instead.
                anyhow::bail!("Lost device");
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            self.draw(&mut render_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(output);

        Ok(())
    }

    /// Draws the grid first, then the curve when one has been uploaded.
    fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.grid_pipeline);
        render_pass.set_vertex_buffer(0, self.grid_vertex_buffer.slice(..));
        render_pass.draw(0..self.grid_vertex_count, 0..1);

        if let Some(graph) = &self.graph_buffers {
            render_pass.set_pipeline(&self.graph_pipeline);
            render_pass.set_vertex_buffer(0, graph.vertex_buffer.slice(..));
            render_pass.set_index_buffer(graph.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..graph.index_count, 0, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_dimensions_are_rejected() {
        assert_eq!(surface_dimensions(0, 0), None);
        assert_eq!(surface_dimensions(0, 600), None);
        assert_eq!(surface_dimensions(800, 0), None);
    }

    #[test]
    fn dimensions_are_clamped_to_max() {
        assert_eq!(surface_dimensions(800, 600), Some((800, 600)));
        assert_eq!(
            surface_dimensions(MAX_SURFACE_DIMENSION + 1, 600),
            Some((MAX_SURFACE_DIMENSION, 600))
        );
        assert_eq!(
            surface_dimensions(800, MAX_SURFACE_DIMENSION + 1),
            Some((800, MAX_SURFACE_DIMENSION))
        );
        assert_eq!(
            surface_dimensions(u32::MAX, u32::MAX),
            Some((MAX_SURFACE_DIMENSION, MAX_SURFACE_DIMENSION))
        );
    }
}
