#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

//! Typed errors returned by the plot engine.
//!
//! The graph and engine layers use this enum internally. The WASM boundary
//! converts it into a JavaScript `Error`, using [`PlotError::code`] as the
//! error's `name` and [`PlotError::to_string`] as its `message`.

use std::fmt;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

/// Something the plot engine refused or failed to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlotError {
    /// The function expression was empty or whitespace-only.
    EmptyFunction,
    /// The requested x range was not finite or not ascending.
    InvalidRange,
    /// The zoom factor was not finite or not positive.
    InvalidZoom,
    /// An offset or zoom was requested before a function was loaded.
    NoFunctionLoaded,
    /// `math_engine` failed to differentiate or sample the function.
    Math(String),
    /// The graph state was already mutably borrowed.
    StateBusy,
    /// The browser window could not be accessed.
    WindowUnavailable,
    /// The browser document could not be accessed.
    DocumentUnavailable,
    /// No element existed with the requested canvas id.
    CanvasNotFound(String),
    /// The element with the requested id was not a canvas.
    NotACanvas,
    /// The winit event loop could not be created.
    EventLoop(String),
    /// Too many functions
    TooManyFunctions,
}

impl PlotError {
    /// Stable identifier exposed as `error.name` in JavaScript.
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyFunction => "EmptyFunction",
            Self::InvalidRange => "InvalidRange",
            Self::InvalidZoom => "InvalidZoom",
            Self::NoFunctionLoaded => "NoFunctionLoaded",
            Self::Math(_) => "MathError",
            Self::StateBusy => "StateBusy",
            Self::WindowUnavailable => "WindowUnavailable",
            Self::DocumentUnavailable => "DocumentUnavailable",
            Self::CanvasNotFound(_) => "CanvasNotFound",
            Self::NotACanvas => "NotACanvas",
            Self::EventLoop(_) => "EventLoopError",
            Self::TooManyFunctions => "TooManyFunctions",
        }
    }
}

impl fmt::Display for PlotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFunction => write!(formatter, "function expression is empty"),
            Self::InvalidRange => {
                write!(
                    formatter,
                    "range must be finite and start must be less than end"
                )
            }
            Self::InvalidZoom => {
                write!(
                    formatter,
                    "zoom factor must be finite and greater than zero"
                )
            }
            Self::NoFunctionLoaded => write!(formatter, "no function has been loaded yet"),
            Self::Math(message) => write!(formatter, "math engine error: {message}"),
            Self::StateBusy => write!(formatter, "graph state is already borrowed"),
            Self::WindowUnavailable => write!(formatter, "failed to get the browser window"),
            Self::DocumentUnavailable => write!(formatter, "failed to get the browser document"),
            Self::CanvasNotFound(id) => write!(formatter, "failed to find canvas with id `{id}`"),
            Self::NotACanvas => write!(
                formatter,
                "the element with the requested id is not a canvas"
            ),
            Self::EventLoop(message) => {
                write!(formatter, "failed to create the event loop: {message}")
            }
            Self::TooManyFunctions => write!(formatter, "too many functions"),
        }
    }
}

impl std::error::Error for PlotError {}

#[cfg(target_arch = "wasm32")]
impl From<PlotError> for JsValue {
    fn from(error: PlotError) -> Self {
        let js_error = js_sys::Error::new(&error.to_string());
        js_error.set_name(error.code());
        js_error.into()
    }
}
