//! Winit application and window-event handling.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[cfg(target_arch = "wasm32")]
use winit::event_loop::EventLoopProxy;

use crate::graph_state::GraphState;
use crate::renderer::Renderer;

/// Events sent from the JS-facing `PlotEngine` into the event loop.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) enum UserEvent {
    RendererReady(Box<Renderer>),
    Redraw,
}

/// Event-loop handler that owns the renderer once the window exists.
pub(crate) struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<EventLoopProxy<UserEvent>>,
    #[cfg(target_arch = "wasm32")]
    canvas: HtmlCanvasElement,
    state: Option<Renderer>,
    graph_state: Rc<RefCell<GraphState>>,
}

impl App {
    /// Creates an app with a shared graph state for the JS instance.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn new(
        proxy: EventLoopProxy<UserEvent>,
        graph_state: Rc<RefCell<GraphState>>,
        canvas: HtmlCanvasElement,
    ) -> Self {
        Self {
            proxy: Some(proxy),
            canvas,
            state: None,
            graph_state,
        }
    }

    /// Creates a native app with its own graph state.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn new() -> Self {
        Self {
            state: None,
            graph_state: Rc::new(RefCell::new(GraphState::new())),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_arch = "wasm32")]
        let window_attributes = {
            use winit::platform::web::WindowAttributesExtWebSys;

            Window::default_attributes().with_canvas(Some(self.canvas.clone()))
        };

        #[cfg(not(target_arch = "wasm32"))]
        let window_attributes = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            // Native targets can block while the GPU resources are created.
            match pollster::block_on(Renderer::new(window)) {
                Ok(renderer) => self.state = Some(renderer),
                Err(error) => log::error!("failed to create the renderer: {error}"),
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Web targets must create the renderer asynchronously, then hand it
            // back to the event loop through the proxy.
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    match Renderer::new(window).await {
                        Ok(renderer) => {
                            if proxy
                                .send_event(UserEvent::RendererReady(Box::new(renderer)))
                                .is_err()
                            {
                                log::error!("event loop closed before the renderer was ready");
                            }
                        }
                        Err(error) => log::error!("failed to create the renderer: {error}"),
                    }
                });
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::RendererReady(renderer) => {
                let mut renderer = *renderer;
                renderer.window.request_redraw();
                renderer.resize(
                    renderer.window.inner_size().width,
                    renderer.window.inner_size().height,
                );
                self.state = Some(renderer);
            }
            UserEvent::Redraw => {
                if let Some(renderer) = &self.state {
                    renderer.window.request_redraw();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.state {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.state {
                    match self.graph_state.try_borrow_mut() {
                        Ok(mut graph_state) => renderer.update(&mut graph_state),
                        Err(_) => {
                            log::error!("graph state is already borrowed; skipping update")
                        }
                    }

                    if let Err(error) = renderer.render() {
                        log::error!("{error}");
                        event_loop.exit();
                    }
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
