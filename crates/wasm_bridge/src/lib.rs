//! `wasm_bridge`: session state and the named-surface WASM ABI (AD-5).
//!
//! Rust owns all live math, GPU state, and viewport state. Types cross the
//! boundary as plain JSON strings (`String` in, `String` out), which keeps
//! every export natively testable with `cargo test` and importable from
//! Svelte after `wasm-pack build`.
//!
//! Exported surface:
//! `parse_and_validate`, `set_function`, `update_param`, `update_viewport`,
//! `query_hover` (AD-5) plus tracer helpers `sample_points`,
//! `serialize_workspace`, `set_canvas_size` (owned by follow-up work).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use core_math::{Expr, Span, parse};
use render::{Viewport, grid_ticks, sample_curve};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Session state
// ---------------------------------------------------------------------------

struct Session {
    /// Function id -> parsed expression. The tracer drives id `"f"`.
    functions: HashMap<String, Expr>,
    /// Function id -> last accepted formula text (for workspace JSON).
    formulas: HashMap<String, String>,
    /// Slider-bound parameters.
    params: HashMap<String, f64>,
    viewport: Viewport,
    canvas_w: f64,
    canvas_h: f64,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            functions: HashMap::new(),
            formulas: HashMap::new(),
            params: HashMap::new(),
            viewport: Viewport::default(),
            canvas_w: 800.0,
            canvas_h: 600.0,
        }
    }
}

fn session() -> &'static Mutex<Session> {
    static SESSION: OnceLock<Mutex<Session>> = OnceLock::new();
    SESSION.get_or_init(|| Mutex::new(Session::default()))
}

/// Reset session state. Exported for tests and the dev harness only.
#[wasm_bindgen]
pub fn reset_session() {
    *session().lock().unwrap() = Session::default();
}

// ---------------------------------------------------------------------------
// JSON shapes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseOk {
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseErr {
    pub span: [usize; 2],
    pub message: String,
}

impl ParseErr {
    fn new(span: Span, message: String) -> Self {
        Self {
            span: [span.start, span.end],
            message,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoverResult {
    pub math_x: f64,
    pub math_y: Option<f64>,
    /// Marker id at the hovered point; `None` until calculus markers land.
    pub marker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    pub version: u32,
    pub functions: HashMap<String, String>,
    pub params: HashMap<String, f64>,
    pub viewport: ViewportDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewportDto {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

impl From<&Viewport> for ViewportDto {
    fn from(v: &Viewport) -> Self {
        Self {
            x_min: v.x_min,
            x_max: v.x_max,
            y_min: v.y_min,
            y_max: v.y_max,
        }
    }
}

// ---------------------------------------------------------------------------
// Plain-Rust core (natively tested)
// ---------------------------------------------------------------------------

fn parse_core(formula: &str) -> Result<ParseOk, ParseErr> {
    match parse(formula) {
        Ok(expr) => Ok(ParseOk {
            params: expr.params_used(),
        }),
        Err(e) => Err(ParseErr::new(e.span, e.message)),
    }
}

fn set_function_core(id: &str, formula: &str) -> Result<ParseOk, ParseErr> {
    let expr = parse(formula).map_err(|e| ParseErr::new(e.span, e.message))?;
    let ok = ParseOk {
        params: expr.params_used(),
    };
    let mut s = session().lock().unwrap();
    for name in &ok.params {
        s.params.entry(name.clone()).or_insert(1.0);
    }
    s.formulas.insert(id.to_string(), formula.to_string());
    s.functions.insert(id.to_string(), expr);
    Ok(ok)
}

/// Active expression for sampling/hover: id `"f"`, else any single function.
fn active_expr(s: &Session) -> Option<Expr> {
    if let Some(e) = s.functions.get("f") {
        return Some(e.clone());
    }
    if s.functions.len() == 1 {
        return s.functions.values().next().cloned();
    }
    None
}

// ---------------------------------------------------------------------------
// WASM ABI (AD-5): JSON strings across the boundary
// ---------------------------------------------------------------------------

/// Validate a formula. Returns `{"params":[...]}` or `{"span":[s,e],"message"}`.
#[wasm_bindgen]
pub fn parse_and_validate(formula: &str) -> String {
    match parse_core(formula) {
        Ok(ok) => serde_json::to_string(&ok).unwrap(),
        Err(e) => serde_json::to_string(&e).unwrap(),
    }
}

/// Install a function: `set_function("f", "a*x^2")`.
/// Returns `{"params":[...]}` or the span error JSON.
#[wasm_bindgen]
pub fn set_function(id: &str, formula: &str) -> String {
    match set_function_core(id, formula) {
        Ok(ok) => serde_json::to_string(&ok).unwrap(),
        Err(e) => serde_json::to_string(&e).unwrap(),
    }
}

/// Bind a slider value: `update_param("a", 2.5)`.
#[wasm_bindgen]
pub fn update_param(key: &str, value: f64) {
    if key == "x" || !is_ident(key) || !value.is_finite() {
        return;
    }
    session().lock().unwrap().params.insert(key.to_string(), value);
}

/// Move the visible window. Ignores degenerate ranges.
#[wasm_bindgen]
pub fn update_viewport(x_min: f64, x_max: f64, y_min: f64, y_max: f64) {
    if !(x_min < x_max) || !(y_min < y_max) {
        return;
    }
    if ![x_min, x_max, y_min, y_max].iter().all(|v| v.is_finite()) {
        return;
    }
    session().lock().unwrap().viewport = Viewport::new(x_min, x_max, y_min, y_max);
}

/// Tell the bridge the canvas pixel size so hover math is exact.
#[wasm_bindgen]
pub fn set_canvas_size(w: f64, h: f64) {
    if w > 0.0 && h > 0.0 && w.is_finite() && h.is_finite() {
        let mut s = session().lock().unwrap();
        s.canvas_w = w;
        s.canvas_h = h;
    }
}

/// Hover query: pixel in, math out. Never performs spatial math in Svelte.
#[wasm_bindgen]
pub fn query_hover(px_x: f64, px_y: f64) -> String {
    let s = session().lock().unwrap();
    let (math_x, _) = s.viewport.screen_to_world(px_x, px_y, s.canvas_w, s.canvas_h);
    let math_y = active_expr(&s).map(|e| {
        let y = e.eval(math_x, &s.params);
        if y.is_finite() && y.abs() <= 1e6 {
            Some(y)
        } else {
            None
        }
    });
    let out = HoverResult {
        math_x,
        math_y: math_y.flatten(),
        marker: None,
    };
    serde_json::to_string(&out).unwrap()
}

/// Tracer helper: sample the active function across the viewport.
///
/// Returns `{"points":[[x,y|null],...],"params":[...]}`. Superseded by the
/// wgpu vertex-buffer path in stories 1.2/1.4.
#[wasm_bindgen]
pub fn sample_points(n: usize) -> String {
    let s = session().lock().unwrap();
    let points: Vec<(f64, Option<f64>)> = match active_expr(&s) {
        Some(expr) => {
            let n = n.clamp(2, 4000);
            sample_curve(&expr, &s.params, &s.viewport, n)
                .into_iter()
                .map(|[x, y]| (x, if y.is_nan() { None } else { Some(y) }))
                .collect()
        }
        None => Vec::new(),
    };
    let params: Vec<String> = s.params.keys().cloned().collect();
    serde_json::to_string(&serde_json::json!({ "points": points, "params": params })).unwrap()
}

/// Tracer helper: grid ticks for the current viewport.
#[wasm_bindgen]
pub fn grid_lines() -> String {
    let s = session().lock().unwrap();
    let x = grid_ticks(s.viewport.x_min, s.viewport.x_max, 10);
    let y = grid_ticks(s.viewport.y_min, s.viewport.y_max, 10);
    serde_json::to_string(&serde_json::json!({ "x": x, "y": y })).unwrap()
}

/// Workspace snapshot stub owned by this epic for Epic 5 (`version: 1`).
#[wasm_bindgen]
pub fn serialize_workspace() -> String {
    let s = session().lock().unwrap();
    let ws = Workspace {
        version: 1,
        functions: s.formulas.clone(),
        params: s.params.clone(),
        viewport: ViewportDto::from(&s.viewport),
    };
    serde_json::to_string(&ws).unwrap()
}

fn is_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        && !s.chars().next().unwrap().is_numeric()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex as StdMutex, OnceLock as StdOnceLock};

    /// Serialize tests: the session is process-global.
    fn test_lock() -> &'static StdMutex<()> {
        static LOCK: StdOnceLock<StdMutex<()>> = StdOnceLock::new();
        LOCK.get_or_init(|| StdMutex::new(()))
    }

    fn parse_json<T: for<'de> Deserialize<'de>>(s: &str) -> T {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn validate_reports_params() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let ok: ParseOk = parse_json(&parse_and_validate("a*x^2+1"));
        assert_eq!(ok.params, vec!["a".to_string()]);
    }

    #[test]
    fn validate_reports_span_error() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let err: ParseErr = parse_json(&parse_and_validate("a**x^"));
        assert!(!err.message.is_empty());
        assert!(err.span[1] <= "a**x^".len());
    }

    #[test]
    fn set_sample_hover_slider_round_trip() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let ok: ParseOk = parse_json(&set_function("f", "a*x^2"));
        assert_eq!(ok.params, vec!["a".to_string()]);
        update_param("a", 2.0);
        set_canvas_size(800.0, 600.0);
        update_viewport(-10.0, 10.0, -10.0, 10.0);

        let sample: serde_json::Value = parse_json(&sample_points(5));
        let pts = sample["points"].as_array().unwrap();
        assert_eq!(pts.len(), 5);
        assert!((pts[0][1].as_f64().unwrap() - 200.0).abs() < 1e-9);

        // center pixel maps to math (0, 0)
        let hover: HoverResult = parse_json(&query_hover(400.0, 300.0));
        assert!(hover.math_x.abs() < 1e-9);
        assert!(hover.math_y.unwrap().abs() < 1e-9);

        let ws: Workspace = parse_json(&serialize_workspace());
        assert_eq!(ws.version, 1);
        assert_eq!(ws.functions.get("f").unwrap(), "a*x^2");
        assert_eq!(ws.params.get("a").unwrap(), &2.0);
    }

    #[test]
    fn rejects_bad_viewport_and_reserved_param() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        update_viewport(5.0, -5.0, 0.0, 1.0);
        update_param("x", 99.0);
        let ws: Workspace = parse_json(&serialize_workspace());
        assert_eq!(ws.viewport.x_min, -10.0);
        assert!(!ws.params.contains_key("x"));
    }
}
