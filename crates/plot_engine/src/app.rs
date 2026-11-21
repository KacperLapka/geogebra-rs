//! Winit application and window-event handling.

use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[cfg(target_arch = "wasm32")]
use winit::event_loop::EventLoop;

use crate::renderer::Renderer;

/// Event-loop handler that owns the renderer once the window exists.
pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<Renderer>>,
    state: Option<Renderer>,
}

impl App {
    /// Creates an app; on the web it also prepares the proxy that delivers the renderer.
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<Renderer>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());

        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationHandler<Renderer> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_arch = "wasm32")]
        let window_attributes = {
            use wasm_bindgen::{JsCast, UnwrapThrowExt};
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            Window::default_attributes().with_canvas(Some(html_canvas_element))
        };

        #[cfg(not(target_arch = "wasm32"))]
        let window_attributes = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            // Native targets can block while the GPU resources are created.
            self.state = Some(pollster::block_on(Renderer::new(window)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Web targets must create the renderer asynchronously.
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(
                                Renderer::new(window)
                                    .await
                                    .expect("Unable to create canvas!!!")
                            )
                            .is_ok()
                    )
                });
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: Renderer) {
        // This is where `proxy.send_event()` ends up on the web.
        #[cfg(target_arch = "wasm32")]
        let event = {
            let mut event = event;
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
            event
        };

        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(renderer) = &mut self.state else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => renderer.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                renderer.update();
                if let Err(error) = renderer.render() {
                    log::error!("{error}");
                    event_loop.exit();
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }
}

/// Exits the application when Escape is pressed.
fn handle_key(event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
    if code == KeyCode::Escape && is_pressed {
        event_loop.exit();
    }
}
