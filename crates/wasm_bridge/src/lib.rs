//! `wasm_bridge`: session state and the named-surface WASM ABI (AD-5).
//!
//! Rust owns all live math, GPU state, and viewport state. Types cross the
//! boundary as plain JSON strings (`String` in, `String` out), which keeps
//! every export natively testable with `cargo test` and importable from
//! Svelte after `wasm-pack build`.
//!
//! Exported surface:
//! `parse_and_validate`, `set_function`, `update_param`, `update_viewport`,
//! `query_hover` (AD-5) plus `serialize_workspace` (workspace JSON owned
//! shape for Epic 5) and tracer helpers `sample_points`,
//! `set_canvas_size`, `gpu_status`, `set_gpu_status`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use core_math::{Expr, Span, parse};
use render::{GpuStatus, POLE_BREAK, Viewport, grid_ticks, sample_curve};
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
    /// Bumped by every accepted `update_viewport`; the canvas loop skips
    /// uniform re-upload while this is unchanged.
    viewport_revision: u64,
    /// Backend the canvas loop actually draws with (story 1.2 status path).
    gpu: GpuStatus,
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
            viewport_revision: 0,
            gpu: GpuStatus::CpuFallback,
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
    /// Schema version. Always `1` in this epic; Epic 5 forwards it as
    /// `SaveBody.version` (AD-6) and rejects `0` / missing.
    pub version: u32,
    /// Function id -> last accepted formula text (e.g. `{"f": "a*x^2"}`).
    pub functions: HashMap<String, String>,
    /// Slider-bound parameters (e.g. `{"a": 2.0}`). Superset of names used
    /// by `functions`; unknown names evaluate to `NaN`, never panic.
    pub params: HashMap<String, f64>,
    /// Viewport owned by the session; Svelte never holds its own copy (AD-5).
    pub viewport: ViewportDto,
}

/// Workspace JSON budget owned by this epic for Epic 5 (R11): a tracer
/// workspace with one function, a handful of params, and a viewport must
/// serialize to well under this limit. Epic 5 POSTs the same document.
pub const MAX_WORKSPACE_JSON_BYTES: usize = 10 * 1024;

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
    session()
        .lock()
        .unwrap()
        .params
        .insert(key.to_string(), value);
}

/// Move the visible window. Ignores degenerate ranges.
///
/// Accepted updates bump the viewport revision so the canvas loop can skip
/// redundant uniform uploads; rejected input leaves revision untouched.
///
/// The negated comparison is deliberate NaN-safe ordering: NaN bounds must
/// reject, and `!(a < b)` rejects NaN where `a >= b` would not.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
#[wasm_bindgen]
pub fn update_viewport(x_min: f64, x_max: f64, y_min: f64, y_max: f64) {
    if !(x_min < x_max) || !(y_min < y_max) {
        return;
    }
    if ![x_min, x_max, y_min, y_max].iter().all(|v| v.is_finite()) {
        return;
    }
    let mut s = session().lock().unwrap();
    s.viewport = Viewport::new(x_min, x_max, y_min, y_max);
    s.viewport_revision += 1;
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

/// Hover query core: pixel in, math out (AD-4). Pure function over explicit
/// inputs so `cargo test` covers the exact tooltip path without WASM.
///
/// Maps the pixel through `viewport.screen_to_world` using the stored canvas
/// size, then evaluates the active expression at `math_x`. Diverging or
/// non-finite samples (beyond [`POLE_BREAK`]) become `math_y: None` so the
/// tooltip shows a gap marker instead of a spike value. Degenerate canvas
/// sizes or non-finite pixels yield `math_x: NaN` with `math_y: None`.
///
/// Svelte never performs this transform itself; it calls [`query_hover`] and
/// displays the returned coordinates verbatim.
///
/// Negated comparisons below are deliberate NaN-safe ordering: non-finite
/// canvas sizes and pixels must take the degenerate path.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn query_hover_core(
    viewport: &Viewport,
    canvas_w: f64,
    canvas_h: f64,
    px_x: f64,
    px_y: f64,
    expr: Option<&Expr>,
    params: &HashMap<String, f64>,
) -> HoverResult {
    if !(canvas_w > 0.0)
        || !(canvas_h > 0.0)
        || !canvas_w.is_finite()
        || !canvas_h.is_finite()
        || !px_x.is_finite()
        || !px_y.is_finite()
    {
        return HoverResult {
            math_x: f64::NAN,
            math_y: None,
            marker: None,
        };
    }
    let (math_x, _) = viewport.screen_to_world(px_x, px_y, canvas_w, canvas_h);
    let math_y = expr
        .map(|e| e.eval(math_x, params))
        .filter(|y| y.is_finite() && y.abs() <= POLE_BREAK);
    HoverResult {
        math_x,
        math_y,
        marker: None,
    }
}

/// Hover query: pixel in, math out. Never performs spatial math in Svelte.
#[wasm_bindgen]
pub fn query_hover(px_x: f64, px_y: f64) -> String {
    let s = session().lock().unwrap();
    let expr = active_expr(&s);
    let out = query_hover_core(
        &s.viewport,
        s.canvas_w,
        s.canvas_h,
        px_x,
        px_y,
        expr.as_ref(),
        &s.params,
    );
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

/// Pipeline status for the Svelte banner and loop backend selection.
///
/// Returns `{"status":"webgpu|webgl2|cpu-fallback","viewport_revision":N}`.
/// Tracer helper owned by story 1.2; the canvas loop polls the revision to
/// skip redundant uniform uploads.
#[wasm_bindgen]
pub fn gpu_status() -> String {
    let s = session().lock().unwrap();
    serde_json::to_string(&serde_json::json!({
        "status": s.gpu.as_str(),
        "viewport_revision": s.viewport_revision,
    }))
    .unwrap()
}

/// Record which backend the canvas loop actually acquired.
///
/// Accepts the [`GpuStatus`] wire names; unknown input falls back to
/// `cpu-fallback` so the loop can never observe an invalid backend.
#[wasm_bindgen]
pub fn set_gpu_status(status: &str) {
    session().lock().unwrap().gpu = GpuStatus::from_wire(status);
}

/// Workspace snapshot owned by this epic for Epic 5 (`version: 1`).
///
/// Owned JSON shape: `{version:1, functions:{id:formula}, params:{name:value},
/// viewport:{x_min,x_max,y_min,y_max}}`. Svelte reads it back only through
/// this function and restores it through the existing ABI
/// (`set_function` + `update_param` + `update_viewport`) — no new ABI
/// needed (AD-5). Epic 5 POSTs this document verbatim; `version` travels as
/// `SaveBody.version` (AD-6). Typical tracer payloads are a few hundred
/// bytes, far under [`MAX_WORKSPACE_JSON_BYTES`].
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
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
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
    fn hover_reports_none_near_pole_break() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "1/x"));
        set_canvas_size(800.0, 600.0);
        update_viewport(-1.0, 1.0, -10.0, 10.0);
        // center pixel maps to math x = 0, where 1/x diverges.
        let hover: HoverResult = parse_json(&query_hover(400.0, 300.0));
        assert!(hover.math_x.abs() < 1e-9);
        assert_eq!(hover.math_y, None);
        assert_eq!(hover.marker, None);
    }

    #[test]
    fn hover_without_function_returns_math_x_only() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        set_canvas_size(800.0, 600.0);
        update_viewport(-10.0, 10.0, -10.0, 10.0);
        let hover: HoverResult = parse_json(&query_hover(600.0, 150.0));
        // math_x is still the exact viewport transform: -10 + 0.75 * 20 = 5.
        assert!((hover.math_x - 5.0).abs() < 1e-9);
        assert_eq!(hover.math_y, None);
    }

    #[test]
    fn hover_core_rejects_degenerate_input() {
        let vp = Viewport::default();
        let params = HashMap::new();
        let bad_canvas = query_hover_core(&vp, 0.0, 600.0, 400.0, 300.0, None, &params);
        assert!(bad_canvas.math_x.is_nan());
        assert_eq!(bad_canvas.math_y, None);
        let bad_px = query_hover_core(&vp, 800.0, 600.0, f64::NAN, 300.0, None, &params);
        assert!(bad_px.math_x.is_nan());
        assert_eq!(bad_px.math_y, None);
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

    #[test]
    fn gpu_status_tracks_backend_and_viewport_revision() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let first: serde_json::Value = parse_json(&gpu_status());
        assert_eq!(first["status"], "cpu-fallback");
        assert_eq!(first["viewport_revision"], 0);

        update_viewport(-5.0, 5.0, -5.0, 5.0);
        update_viewport(-4.0, 4.0, -4.0, 4.0);
        update_viewport(4.0, -4.0, -4.0, 4.0); // rejected: no bump
        let moved: serde_json::Value = parse_json(&gpu_status());
        assert_eq!(moved["viewport_revision"], 2);

        set_gpu_status("webgpu");
        let gpu: serde_json::Value = parse_json(&gpu_status());
        assert_eq!(gpu["status"], "webgpu");
        set_gpu_status("webgl2");
        let gl: serde_json::Value = parse_json(&gpu_status());
        assert_eq!(gl["status"], "webgl2");
        set_gpu_status("nonsense");
        let fallback: serde_json::Value = parse_json(&gpu_status());
        assert_eq!(fallback["status"], "cpu-fallback");
    }

    #[test]
    fn slider_sweep_updates_curve_in_real_time() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let ok: ParseOk = parse_json(&set_function("f", "a*x^2"));
        assert_eq!(ok.params, vec!["a".to_string()]);
        update_viewport(-10.0, 10.0, -10.0, 10.0);
        // Sweep slider `a`: y at x=2 must track a*x^2 exactly, no re-parse.
        for (a, want) in [(1.0, 4.0), (2.0, 8.0), (0.5, 2.0), (-1.0, -4.0)] {
            update_param("a", a);
            let sample: serde_json::Value = parse_json(&sample_points(5));
            let pts = sample["points"].as_array().unwrap();
            // 5 points across [-10,10]: index 3 is x=5? index mapping:
            // x_i = -10 + 20*i/4, so check x=10 corner scales with `a`.
            let corner = pts[4][1].as_f64().unwrap();
            assert!((corner - a * 100.0).abs() < 1e-9, "a={a}: {corner}");
            // Direct eval path the sampler reads: HashMap params, no globals.
            let s = session().lock().unwrap();
            let expr = active_expr(&s).unwrap();
            assert!((expr.eval(2.0, &s.params) - want).abs() < 1e-12);
        }
    }

    #[test]
    fn slider_rejects_non_finite_and_bad_idents() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "a*x^2"));
        update_param("a", 2.0);
        update_param("a", f64::NAN);
        update_param("a", f64::INFINITY);
        update_param("", 5.0);
        update_param("9lives", 5.0);
        update_param("x", 5.0);
        let ws: Workspace = parse_json(&serialize_workspace());
        assert_eq!(ws.params.get("a").unwrap(), &2.0);
        assert!(!ws.params.contains_key(""));
        assert!(!ws.params.contains_key("9lives"));
        assert!(!ws.params.contains_key("x"));
    }

    #[test]
    fn workspace_json_shape_version_1_under_10kb() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "a*x^2"));
        update_param("a", 2.5);
        update_viewport(-10.0, 10.0, -10.0, 10.0);
        let raw = serialize_workspace();
        assert!(raw.len() < MAX_WORKSPACE_JSON_BYTES, "len={}", raw.len());
        let v: serde_json::Value = parse_json(&raw);
        assert_eq!(v["version"], 1);
        assert_eq!(v["functions"]["f"], "a*x^2");
        assert_eq!(v["params"]["a"], 2.5);
        for key in ["x_min", "x_max", "y_min", "y_max"] {
            assert!(v["viewport"][key].is_number(), "missing {key}");
        }
        let ws: Workspace = parse_json(&raw);
        assert_eq!(ws.version, 1);
        assert_eq!(ws.functions.get("f").unwrap(), "a*x^2");
    }

    #[test]
    fn workspace_round_trip_restores_identical_session() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "a*x^2"));
        update_param("a", 2.0);
        update_viewport(-5.0, 5.0, -5.0, 5.0);
        let raw = serialize_workspace();
        let ws: Workspace = parse_json(&raw);
        // Restore through the existing ABI only (AD-5): the Epic 5 recipe.
        reset_session();
        for (id, formula) in &ws.functions {
            let ok: ParseOk = parse_json(&set_function(id, formula));
            assert!(ok.params.contains(&"a".to_string()));
        }
        for (k, v) in &ws.params {
            update_param(k, *v);
        }
        update_viewport(
            ws.viewport.x_min,
            ws.viewport.x_max,
            ws.viewport.y_min,
            ws.viewport.y_max,
        );
        let again: Workspace = parse_json(&serialize_workspace());
        assert_eq!(ws, again);
    }
}
