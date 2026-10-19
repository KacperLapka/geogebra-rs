//! `wasm_bridge`: session state and the named-surface WASM ABI (AD-5).
//!
//! Rust owns all live math, GPU state, and viewport state. Types cross the
//! boundary as plain JSON strings (`String` in, `String` out), which keeps
//! every export natively testable with `cargo test` and importable from
//! Svelte after `wasm-pack build`.
//!
//! Exported surface:
//! `parse_and_validate`, `set_function`, `set_function_style`,
//! `set_function_visibility`, `update_param`, `update_viewport`,
//! `query_hover` (AD-5) plus `serialize_workspace` (workspace JSON owned
//! shape for Epic 5) and tracer helpers `sample_points`,
//! `sample_points_for` (4.3 per-id sampler), `set_canvas_size`,
//! `gpu_status`, `set_gpu_status`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use core_math::{find_extrema, find_inflections, find_poles, find_roots, parse, Expr, MarkerKind, Span};
use render::{
    clamp_width, default_style, for_function_index, grid_ticks, hex_to_linear, sample_curve,
    tangent_segment, vertical_asymptote_segment, FunctionStyle, GpuStatus, Viewport, POLE_BREAK,
};
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
    /// Function id -> render-owned style (E4-STYLE-STRUCT). Auto-created on
    /// `set_function`; `visible=false` skips submission, never deletes.
    styles: HashMap<String, FunctionStyle>,
    /// First-seen style order for the 8-colour cycle: `f` pinned to index 0,
    /// other ids take first-seen order 1, 2, ... (`% 8` wraps).
    style_order: Vec<String>,
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
            styles: HashMap::new(),
            style_order: Vec::new(),
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

/// Calculus marker overlay for `sample_points` (2.6, additive-only).
///
/// `kind` is one of `root|min|max|inflection`; `x`/`y` are math coords.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarkerDto {
    pub kind: String,
    pub x: f64,
    pub y: f64,
}

/// Tangent overlay for `sample_points` (2.6, additive-only).
///
/// `from`/`to` are viewport-spanning segment endpoints on
/// `y = y0 + slope * (x - x0)`; `null` when `x0` is missing/non-finite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TangentDto {
    pub x0: f64,
    pub y0: f64,
    pub slope: f64,
    pub from: [f64; 2],
    pub to: [f64; 2],
}

/// Vertical asymptote overlay for `sample_points` (3.1, additive-only).
///
/// `x` is the pole-x from the solver; `from`/`to` are viewport-spanning
/// vertical endpoints `[x,y_min] -> [x,y_max]`. Carries stories 2-4
/// Horizontal/Slant without a second ABI addition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AsymptoteDto {
    pub x: f64,
    pub from: [f64; 2],
    pub to: [f64; 2],
}

fn marker_kind_str(kind: MarkerKind) -> &'static str {
    match kind {
        MarkerKind::Root => "root",
        MarkerKind::LocalMin => "min",
        MarkerKind::LocalMax => "max",
        MarkerKind::Inflection => "inflection",
    }
}

/// Display id for a marker: kind plus coords at 3 decimals, shown verbatim
/// in the tooltip suffix (e.g. `root (1.000, 0.000)`).
fn marker_id(kind: &str, x: f64, y: f64) -> String {
    format!("{kind} ({x:.3}, {y:.3})")
}

/// Viewport-scoped markers via the solver pipeline (~200 scan steps to
/// protect the frame budget). Pole jumps never emit (AD-3); `1/x` yields no
/// markers.
fn collect_markers(
    expr: &Expr,
    params: &HashMap<String, f64>,
    viewport: &Viewport,
) -> Vec<MarkerDto> {
    const STEPS: usize = 200;
    const TOL: f64 = 1e-12;
    let mut out = Vec::new();
    for m in find_roots(expr, params, viewport.x_min, viewport.x_max, STEPS, TOL) {
        out.push(MarkerDto {
            kind: marker_kind_str(m.kind).to_string(),
            x: m.x,
            y: m.y,
        });
    }
    for m in find_extrema(expr, params, viewport.x_min, viewport.x_max, STEPS, TOL) {
        out.push(MarkerDto {
            kind: marker_kind_str(m.kind).to_string(),
            x: m.x,
            y: m.y,
        });
    }
    for m in find_inflections(expr, params, viewport.x_min, viewport.x_max, STEPS, TOL) {
        out.push(MarkerDto {
            kind: marker_kind_str(m.kind).to_string(),
            x: m.x,
            y: m.y,
        });
    }
    out.retain(|d| d.x.is_finite() && d.y.is_finite());
    out
}

/// Tangent overlay from the ordinary `x0` session param. Missing/non-finite
/// `x0` (or a non-finite tangent) yields `None` so the caller emits
/// `tangent: null` with curve + markers untouched.
fn tangent_for(
    expr: &Expr,
    params: &HashMap<String, f64>,
    viewport: &Viewport,
) -> Option<TangentDto> {
    let x0 = *params.get("x0")?;
    if !x0.is_finite() {
        return None;
    }
    let seg = tangent_segment(expr, params, x0, viewport)?;
    Some(TangentDto {
        x0: seg.x0,
        y0: seg.y0,
        slope: seg.slope,
        from: seg.from,
        to: seg.to,
    })
}

/// Vertical asymptote overlay from solver pole-x detection (3.1).
///
/// Mirrors [`collect_markers`]: viewport-scoped [`find_poles`] (~200 scan
/// steps) mapped through [`vertical_asymptote_segment`]. Non-finite pole-x
/// and degenerate viewports yield nothing; the caller then emits an empty
/// list with curve + markers untouched. Hidden curves never reach here.
fn collect_verticals(
    expr: &Expr,
    params: &HashMap<String, f64>,
    viewport: &Viewport,
) -> Vec<AsymptoteDto> {
    const STEPS: usize = 200;
    const TOL: f64 = 1e-12;
    let mut out = Vec::new();
    for x in find_poles(expr, params, viewport.x_min, viewport.x_max, STEPS, TOL) {
        if !x.is_finite() {
            continue;
        }
        if x < viewport.x_min || x > viewport.x_max {
            continue;
        }
        if let Some(seg) = vertical_asymptote_segment(x, viewport) {
            if !seg.from.iter().all(|v| v.is_finite())
                || !seg.to.iter().all(|v| v.is_finite())
            {
                continue;
            }
            out.push(AsymptoteDto {
                x: seg.x,
                from: seg.from,
                to: seg.to,
            });
        }
    }
    out.retain(|d| {
        d.x.is_finite()
            && d.from.iter().all(|v| v.is_finite())
            && d.to.iter().all(|v| v.is_finite())
    });
    out
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
    /// Optional per-id style map (4.3, additive-only). Serialized alongside
    /// when non-empty; missing (or null) reads as no custom style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<HashMap<String, StyleEntry>>,
}

/// JSON wire shape for one stored style in the workspace document.
/// Hex stays `#RRGGBB` so the style setters can reapply it verbatim.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StyleEntry {
    pub color_hex: String,
    pub width_px: f32,
    pub dashed: bool,
    pub visible: bool,
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
    // Ident gate before order tracking: only valid ids join the cycle.
    if is_ident(id) {
        ensure_style_order(&mut s, id);
    }
    // Auto-default the render-owned style so the curve draws immediately.
    let style = default_style_for_ordered(&s, id);
    s.styles.entry(id.to_string()).or_insert(style);
    Ok(ok)
}

/// Record first-seen order for the palette cycle. `"f"` pins to index 0 and
/// never joins the order; callers validate with `is_ident` before calling.
/// No ident check here — callers validate with `is_ident` before calling.
fn ensure_style_order(s: &mut Session, id: &str) {
    if id == "f" {
        return;
    }
    if !s.style_order.iter().any(|e| e == id) {
        s.style_order.push(id.to_string());
    }
}

/// Default style per id: the tracer curve `"f"` pins to palette index 0;
/// other ids take first-seen order 1, 2, ...; unknown ids fall back to 0.
fn default_style_for_ordered(s: &Session, id: &str) -> FunctionStyle {
    if id == "f" {
        return for_function_index(0);
    }
    match s.style_order.iter().position(|e| e == id) {
        Some(pos) => for_function_index(pos + 1),
        None => for_function_index(0),
    }
}

/// JSON wire shape for a stored style. Always returned by the style setters
/// so the frontend can sync clamped widths without a second round-trip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StyleDto {
    pub color: [f32; 4],
    pub width_px: f32,
    pub dashed: bool,
    pub visible: bool,
}

impl From<&FunctionStyle> for StyleDto {
    fn from(s: &FunctionStyle) -> Self {
        Self {
            color: s.color,
            width_px: s.width_px,
            dashed: s.dashed,
            visible: s.visible,
        }
    }
}

fn style_dto(s: &FunctionStyle) -> String {
    serde_json::to_string(&StyleDto::from(s)).unwrap()
}

/// Style update core: hex validation + width clamp. Bad hex or bad id keeps
/// the stored style untouched. Visibility is always preserved here.
fn set_function_style_core(
    id: &str,
    color_hex: &str,
    width_px: f64,
    dashed: bool,
) -> FunctionStyle {
    if !is_ident(id) {
        return default_style();
    }
    let Some(rgb) = hex_to_linear(color_hex) else {
        let s = session().lock().unwrap();
        let fallback = default_style_for_ordered(&s, id);
        return s.styles.get(id).copied().unwrap_or(fallback);
    };
    let mut s = session().lock().unwrap();
    ensure_style_order(&mut s, id);
    let fallback = default_style_for_ordered(&s, id);
    let entry = s.styles.entry(id.to_string()).or_insert(fallback);
    entry.color = rgb;
    entry.width_px = clamp_width(width_px as f32);
    entry.dashed = dashed;
    *entry
}

/// Visibility core: retains the entry, flips only the flag.
fn set_function_visibility_core(id: &str, visible: bool) -> FunctionStyle {
    if !is_ident(id) {
        return default_style();
    }
    let mut s = session().lock().unwrap();
    ensure_style_order(&mut s, id);
    let fallback = default_style_for_ordered(&s, id);
    let entry = s.styles.entry(id.to_string()).or_insert(fallback);
    entry.visible = visible;
    *entry
}

/// Visible flag of the active function's style; a missing entry counts as
/// visible so curves drawn before any style call keep rendering.
fn active_visible(s: &Session) -> bool {
    let id = if s.functions.contains_key("f") {
        Some("f")
    } else if s.functions.len() == 1 {
        s.functions.keys().next().map(String::as_str)
    } else {
        None
    };
    id.and_then(|id| s.styles.get(id))
        .map(|st| st.visible)
        .unwrap_or(true)
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

/// Convert linear-sRGB RGBA back to `#RRGGBB` for the workspace style map.
fn linear_to_hex(c: [f32; 4]) -> String {
    fn to_srgb(v: f32) -> u8 {
        let s = if v <= 0.0031308 {
            12.92 * v
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (s.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    format!(
        "#{:02X}{:02X}{:02X}",
        to_srgb(c[0]),
        to_srgb(c[1]),
        to_srgb(c[2])
    )
}

fn style_entry(s: &FunctionStyle) -> StyleEntry {
    StyleEntry {
        color_hex: linear_to_hex(s.color),
        width_px: s.width_px,
        dashed: s.dashed,
        visible: s.visible,
    }
}

/// Per-id sample core: samples one function by id with its own style.
/// Returns `None` for unknown ids; hidden styles yield no points but keep
/// the entry. NaN y values serialize as `null` so the frontend breaks the
/// polyline instead of drawing a vertical spike.
fn sample_points_for_core(id: &str, n: usize) -> Option<Vec<(f64, Option<f64>)>> {
    let s = session().lock().unwrap();
    let expr = s.functions.get(id)?.clone();
    if s.styles.get(id).map(|st| st.visible).unwrap_or(true) {
        let n = n.clamp(2, 4000);
        Some(
            sample_curve(&expr, &s.params, &s.viewport, n)
                .into_iter()
                .map(|[x, y]| (x, if y.is_nan() { None } else { Some(y) }))
                .collect(),
        )
    } else {
        Some(Vec::new())
    }
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

/// Install or restyle a curve: `set_function_style("f", "#2563eb", 3.5, false)`.
///
/// Width clamps to [2.5, 6.0]; malformed hex keeps the stored style.
/// Always returns the stored style JSON
/// `{"color":[r,g,b,a],"width_px":w,"dashed":b,"visible":b}`.
#[wasm_bindgen]
pub fn set_function_style(id: &str, color_hex: &str, width_px: f64, dashed: bool) -> String {
    style_dto(&set_function_style_core(id, color_hex, width_px, dashed))
}

/// Show or hide a curve: `set_function_visibility("f", false)`.
///
/// The style entry is retained so toggling back restores the ribbon without
/// re-entering the formula. Returns the stored style JSON (see
/// [`set_function_style`]).
#[wasm_bindgen]
pub fn set_function_visibility(id: &str, visible: bool) -> String {
    style_dto(&set_function_visibility_core(id, visible))
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
/// When the pixel lands within ~12px of a calculus marker (viewport-scoped
/// roots/extrema/inflections), `marker` names it (`kind (x, y)` at 3
/// decimals) and the coords snap to the marker. Svelte never performs this
/// transform itself; it calls [`query_hover`] and displays the returned
/// coordinates verbatim.
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
    // Nearest-marker snap within ~12px (viewport-only, pole jumps excluded
    // by the solver). Snapped coords equal the marker within 1e-12.
    if let Some(e) = expr {
        let markers = collect_markers(e, params, viewport);
        let mut best: Option<(&MarkerDto, f64)> = None;
        for m in &markers {
            let (mx, my) = viewport.world_to_screen(m.x, m.y, canvas_w, canvas_h);
            if !mx.is_finite() || !my.is_finite() {
                continue;
            }
            let dx = mx - px_x;
            let dy = my - px_y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= 12.0 && best.map(|(_, d)| dist < d).unwrap_or(true) {
                best = Some((m, dist));
            }
        }
        if let Some((m, _)) = best {
            return HoverResult {
                math_x: m.x,
                math_y: Some(m.y),
                marker: Some(marker_id(&m.kind, m.x, m.y)),
            };
        }
    }
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
/// Returns `{"points":[[x,y|null],...],"params":[...],"markers":[{kind,x,y},...],
/// "tangent":{x0,y0,slope,from,to}|null,"asymptotes":[{x,from,to},...]}`.
/// `markers`/`tangent`/`asymptotes` are additive:
/// existing `points`/`params` fields are byte-identical. Superseded by the
/// wgpu vertex-buffer path in stories 1.2/1.4 for curves, but the overlay
/// fields feed the live 2D canvas.
#[wasm_bindgen]
pub fn sample_points(n: usize) -> String {
    let s = session().lock().unwrap();
    let expr = active_expr(&s);
    let visible = active_visible(&s);
    let params_map = s.params.clone();
    let viewport = s.viewport;
    let points: Vec<(f64, Option<f64>)> = match expr.clone() {
        Some(expr) if visible => {
            let n = n.clamp(2, 4000);
            sample_curve(&expr, &s.params, &s.viewport, n)
                .into_iter()
                .map(|[x, y]| (x, if y.is_nan() { None } else { Some(y) }))
                .collect()
        }
        Some(_) | None => Vec::new(),
    };
    let (markers, tangent, asymptotes): (
        Vec<MarkerDto>,
        Option<TangentDto>,
        Vec<AsymptoteDto>,
    ) = match expr.as_ref() {
        Some(e) if visible => (
            collect_markers(e, &params_map, &viewport),
            tangent_for(e, &params_map, &viewport),
            collect_verticals(e, &params_map, &viewport),
        ),
        _ => (Vec::new(), None, Vec::new()),
    };
    let params: Vec<String> = s.params.keys().cloned().collect();
    serde_json::to_string(&serde_json::json!({
        "points": points,
        "params": params,
        "markers": markers,
        "tangent": tangent,
        "asymptotes": asymptotes,
    }))
    .unwrap()
}

/// Per-id sampler (4.3, additive): sample one function by id with its own
/// style. Shape matches [`sample_points`]: `{"points":[[x,y|null],...],
/// "params":[...]}`. Unknown ids yield empty `points`; hidden styles yield
/// empty `points` with the style entry retained.
#[wasm_bindgen]
pub fn sample_points_for(id: &str, n: usize) -> String {
    let s = session().lock().unwrap();
    let params: Vec<String> = s.params.keys().cloned().collect();
    drop(s);
    let points: Vec<(f64, Option<f64>)> = sample_points_for_core(id, n).unwrap_or_default();
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
    let style = if s.styles.is_empty() {
        None
    } else {
        Some(
            s.styles
                .iter()
                .map(|(id, st)| (id.clone(), style_entry(st)))
                .collect(),
        )
    };
    let ws = Workspace {
        version: 1,
        functions: s.formulas.clone(),
        params: s.params.clone(),
        viewport: ViewportDto::from(&s.viewport),
        style,
    };
    serde_json::to_string(&ws).unwrap()
}

fn is_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !s.chars().next().unwrap().is_ascii_digit()
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
    fn style_auto_default_on_set_function() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^2"));
        let style: StyleDto = parse_json(&set_function_visibility("f", true));
        assert!(style.visible);
        assert!((style.width_px - 3.5).abs() < 1e-6);
        assert!(!style.dashed);
        let expect = render::hex_to_linear("#2563eb").unwrap();
        for (a, b) in style.color.iter().zip(expect.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn style_width_clamps_both_ends() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^2"));
        let narrow: StyleDto = parse_json(&set_function_style("f", "#2563eb", 1.0, false));
        assert!((narrow.width_px - 2.5).abs() < 1e-6);
        let wide: StyleDto = parse_json(&set_function_style("f", "#2563eb", 10.0, false));
        assert!((wide.width_px - 6.0).abs() < 1e-6);
    }

    #[test]
    fn style_bad_hex_keeps_previous() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^2"));
        let before: StyleDto = parse_json(&set_function_style("f", "#2563eb", 3.5, false));
        let after: StyleDto = parse_json(&set_function_style("f", "not-a-color", 3.5, false));
        assert_eq!(before, after);
    }

    #[test]
    fn visibility_retains_entry_and_restores() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^2"));
        let _: StyleDto = parse_json(&set_function_style("f", "#2563eb", 4.0, false));
        let hidden: StyleDto = parse_json(&set_function_visibility("f", false));
        assert!(!hidden.visible);
        assert!((hidden.width_px - 4.0).abs() < 1e-6);
        // Style entry retained while hidden: width and colour survive.
        let restored: StyleDto = parse_json(&set_function_visibility("f", true));
        assert!(restored.visible);
        assert!((restored.width_px - 4.0).abs() < 1e-6);
        assert_eq!(restored.color, hidden.color);
    }

    #[test]
    fn hidden_curve_samples_no_points() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^2"));
        let full: serde_json::Value = parse_json(&sample_points(16));
        assert!(!full["points"].as_array().unwrap().is_empty());
        let _: StyleDto = parse_json(&set_function_visibility("f", false));
        let empty: serde_json::Value = parse_json(&sample_points(16));
        assert!(empty["points"].as_array().unwrap().is_empty());
    }

    #[test]
    fn visibility_without_prior_function_defaults() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let style: StyleDto = parse_json(&set_function_visibility("g", false));
        assert!(!style.visible);
        assert!((style.width_px - 3.5).abs() < 1e-6);
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

    #[test]
    fn two_function_session_under_10kb_round_trips() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "a*x^2"));
        let _: ParseOk = parse_json(&set_function("g", "b*sin(x)"));
        update_param("a", 2.0);
        update_param("b", 0.5);
        update_viewport(-10.0, 10.0, -10.0, 10.0);
        let raw = serialize_workspace();
        assert!(raw.len() < MAX_WORKSPACE_JSON_BYTES, "len={}", raw.len());
        let v: serde_json::Value = parse_json(&raw);
        assert_eq!(v["version"], 1);
        assert_eq!(v["functions"]["f"], "a*x^2");
        assert_eq!(v["functions"]["g"], "b*sin(x)");
        let ws: Workspace = parse_json(&raw);
        reset_session();
        for (id, formula) in &ws.functions {
            let _: ParseOk = parse_json(&set_function(id, formula));
        }
        for (k, val) in &ws.params {
            update_param(k, *val);
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

    #[test]
    fn cycle_assigns_f_g_h_in_order() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^2"));
        let _: ParseOk = parse_json(&set_function("g", "x^3"));
        let _: ParseOk = parse_json(&set_function("h", "sin(x)"));
        let s = session().lock().unwrap();
        let f = s.styles.get("f").copied().unwrap();
        let g = s.styles.get("g").copied().unwrap();
        let h = s.styles.get("h").copied().unwrap();
        drop(s);
        assert_eq!(f, for_function_index(0));
        assert_eq!(g, for_function_index(1));
        assert_eq!(h, for_function_index(2));
    }

    #[test]
    fn ninth_function_wraps_to_entry_zero() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let ids = ["f", "a", "b", "c", "d", "e", "g", "h", "i"];
        for id in ids {
            let _: ParseOk = parse_json(&set_function(id, "x"));
        }
        let s = session().lock().unwrap();
        let ninth = s.styles.get("i").copied().unwrap();
        drop(s);
        // `f` pins 0; other ids take first-seen slots 1..8, so the ninth id
        // wraps (`8 % 8 == 0`).
        assert_eq!(ninth, for_function_index(8));
        assert_eq!(ninth, for_function_index(0));
    }

    #[test]
    fn invalid_id_never_joins_cycle_order() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("9bad", "x"));
        let _: ParseOk = parse_json(&set_function("g", "x^2"));
        let s = session().lock().unwrap();
        assert!(!s.style_order.iter().any(|e| e == "9bad"));
        assert_eq!(s.styles.get("g").copied().unwrap(), for_function_index(1));
    }

    #[test]
    fn per_id_sampler_keeps_shape_and_respects_visibility() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^2"));
        let _: ParseOk = parse_json(&set_function("g", "x^3"));
        // Shape matches `sample_points` exactly: `{points, params}` only.
        let one: serde_json::Value = parse_json(&sample_points_for("f", 8));
        assert_eq!(one.as_object().unwrap().len(), 2);
        assert!(one.get("points").is_some() && one.get("params").is_some());
        assert!(one.get("markers").is_none() && one.get("tangent").is_none());
        assert_eq!(one["points"].as_array().unwrap().len(), 8);
        let other: serde_json::Value = parse_json(&sample_points_for("g", 8));
        assert_eq!(other["points"].as_array().unwrap().len(), 8);
        // Unknown id yields empty points, never an error.
        let missing: serde_json::Value = parse_json(&sample_points_for("zzz", 8));
        assert!(missing["points"].as_array().unwrap().is_empty());
        // Hidden curve samples nothing but retains its entry.
        let _: StyleDto = parse_json(&set_function_visibility("g", false));
        let hidden: serde_json::Value = parse_json(&sample_points_for("g", 8));
        assert!(hidden["points"].as_array().unwrap().is_empty());
        let shown: serde_json::Value = parse_json(&sample_points_for("f", 8));
        assert!(!shown["points"].as_array().unwrap().is_empty());
        let restored: StyleDto = parse_json(&set_function_visibility("g", true));
        assert!(restored.visible);
        let back: serde_json::Value = parse_json(&sample_points_for("g", 8));
        assert!(!back["points"].as_array().unwrap().is_empty());
    }

    #[test]
    fn per_id_sampler_preserves_pole_breaks() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^2"));
        let _: ParseOk = parse_json(&set_function("g", "1/x"));
        update_viewport(-1.0, 1.0, -10.0, 10.0);
        // Odd count puts a sample exactly at x=0 where 1/x diverges.
        let sample: serde_json::Value = parse_json(&sample_points_for("g", 65));
        let pts = sample["points"].as_array().unwrap();
        // NaN serializes as null: the pole break survives per-id sampling.
        assert!(pts.iter().any(|p| p[1].is_null()));
        assert!(pts.iter().all(|p| {
            p[1].is_null()
                || p[1]
                    .as_f64()
                    .map(|y| y.abs() <= POLE_BREAK)
                    .unwrap_or(false)
        }));
    }

    #[test]
    fn style_map_omitted_when_empty_and_round_trips() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let raw = serialize_workspace();
        let v: serde_json::Value = parse_json(&raw);
        assert!(v.get("style").is_none(), "empty map must omit `style`");
        let ws: Workspace = parse_json(&raw);
        assert_eq!(ws.style, None);

        let _: ParseOk = parse_json(&set_function("f", "x^2"));
        let _: StyleDto = parse_json(&set_function_style("f", "#E69F00", 4.0, true));
        let raw = serialize_workspace();
        let v: serde_json::Value = parse_json(&raw);
        assert_eq!(v["style"]["f"]["color_hex"], "#E69F00");
        assert_eq!(v["style"]["f"]["width_px"], 4.0);
        assert_eq!(v["style"]["f"]["dashed"], true);
        // Missing map reads as no custom style; explicit null too.
        let bare = r#"{"version":1,"functions":{"f":"x^2"},"params":{},"viewport":{"x_min":-10.0,"x_max":10.0,"y_min":-10.0,"y_max":10.0}}"#;
        let ws: Workspace = parse_json(bare);
        assert_eq!(ws.style, None);
        let nulled = r#"{"version":1,"functions":{"f":"x^2"},"params":{},"viewport":{"x_min":-10.0,"x_max":10.0,"y_min":-10.0,"y_max":10.0},"style":null}"#;
        let ws: Workspace = parse_json(nulled);
        assert_eq!(ws.style, None);
        // Style entries reapply through the existing setters verbatim.
        let styled: Workspace = parse_json(&raw);
        reset_session();
        for (id, formula) in &styled.functions {
            let _: ParseOk = parse_json(&set_function(id, formula));
        }
        for (id, entry) in styled.style.clone().unwrap() {
            let back: StyleDto = parse_json(&set_function_style(
                &id,
                &entry.color_hex,
                entry.width_px as f64,
                entry.dashed,
            ));
            assert!((back.width_px - entry.width_px).abs() < 1e-6);
            assert_eq!(back.dashed, entry.dashed);
            let _: StyleDto = parse_json(&set_function_visibility(&id, entry.visible));
        }
        let again: Workspace = parse_json(&serialize_workspace());
        assert_eq!(styled, again);
    }

    #[test]
    fn hover_near_root_snaps_to_marker() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^2-1"));
        set_canvas_size(800.0, 600.0);
        update_viewport(-10.0, 10.0, -10.0, 10.0);
        // Root at (1, 0) maps to pixel (440, 300) on this canvas.
        let hover: HoverResult = parse_json(&query_hover(440.0, 300.0));
        let marker = hover.marker.expect("expected root marker");
        assert!(marker.contains("root"), "marker: {marker}");
        assert!((hover.math_x - 1.0).abs() < 1e-12, "x: {}", hover.math_x);
        assert!(
            (hover.math_y.unwrap() - 0.0).abs() < 1e-12,
            "y: {:?}",
            hover.math_y
        );
    }

    #[test]
    fn hover_far_from_markers_returns_coords_only() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^2-1"));
        set_canvas_size(800.0, 600.0);
        update_viewport(-10.0, 10.0, -10.0, 10.0);
        // Mid-gap at x=0: 40px from either root, outside the 12px snap.
        let hover: HoverResult = parse_json(&query_hover(400.0, 300.0));
        assert_eq!(hover.marker, None);
        assert!((hover.math_x - 0.0).abs() < 1e-9);
        assert!((hover.math_y.unwrap() + 1.0).abs() < 1e-9);
    }

    #[test]
    fn samples_carry_max_min_inflection_markers() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^3-3*x"));
        update_viewport(-3.0, 3.0, -3.0, 3.0);
        let sample: serde_json::Value = parse_json(&sample_points(240));
        let markers = sample["markers"].as_array().unwrap();
        let find = |kind: &str| {
            markers
                .iter()
                .find(|m| m["kind"] == kind)
                .unwrap_or_else(|| panic!("missing {kind}: {markers:?}"))
                .clone()
        };
        let max = find("max");
        let min = find("min");
        let infl = find("inflection");
        assert!((max["x"].as_f64().unwrap() + 1.0).abs() < 1e-12);
        assert!((max["y"].as_f64().unwrap() - 2.0).abs() < 1e-12);
        assert!((min["x"].as_f64().unwrap() - 1.0).abs() < 1e-12);
        assert!((min["y"].as_f64().unwrap() + 2.0).abs() < 1e-12);
        assert!(infl["x"].as_f64().unwrap().abs() < 1e-12);
        assert!(infl["y"].as_f64().unwrap().abs() < 1e-12);
    }

    #[test]
    fn samples_carry_tangent_on_the_line() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^2"));
        update_param("x0", 1.0);
        let sample: serde_json::Value = parse_json(&sample_points(240));
        let tangent = &sample["tangent"];
        assert!(tangent.is_object(), "tangent: {tangent}");
        assert!((tangent["x0"].as_f64().unwrap() - 1.0).abs() < 1e-12);
        assert!((tangent["y0"].as_f64().unwrap() - 1.0).abs() < 1e-12);
        assert!((tangent["slope"].as_f64().unwrap() - 2.0).abs() < 1e-12);
        for end in [tangent["from"].clone(), tangent["to"].clone()] {
            let x = end[0].as_f64().unwrap();
            let y = end[1].as_f64().unwrap();
            let want = 1.0 + 2.0 * (x - 1.0);
            assert!((y - want).abs() < 1e-12, "x={x}: got {y} want {want}");
        }
    }

    #[test]
    fn missing_x0_leaves_curve_and_markers_untouched() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^2"));
        // No `x0` param yet: tangent null, curve + markers unaffected.
        let sample: serde_json::Value = parse_json(&sample_points(240));
        assert!(
            sample["tangent"].is_null(),
            "tangent: {}",
            sample["tangent"]
        );
        assert!(!sample["points"].as_array().unwrap().is_empty());
        assert!(!sample["markers"].as_array().unwrap().is_empty());
        // Non-finite `x0` is rejected by `update_param`: still null.
        update_param("x0", f64::NAN);
        let again: serde_json::Value = parse_json(&sample_points(240));
        assert!(again["tangent"].is_null());
        assert!(!again["points"].as_array().unwrap().is_empty());
    }

    #[test]
    fn pole_form_has_no_markers_but_keeps_tangent() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "1/x"));
        update_viewport(-10.0, 10.0, -10.0, 10.0);
        update_param("x0", 1.0);
        let sample: serde_json::Value = parse_json(&sample_points(240));
        assert!(sample["markers"].as_array().unwrap().is_empty());
        let tangent = &sample["tangent"];
        assert!(tangent.is_object(), "tangent: {tangent}");
        assert!((tangent["y0"].as_f64().unwrap() - 1.0).abs() < 1e-12);
        assert!((tangent["slope"].as_f64().unwrap() + 1.0).abs() < 1e-12);
    }

    #[test]
    fn reciprocal_carries_single_vertical_asymptote() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "1/x"));
        update_viewport(-1.0, 1.0, -10.0, 10.0);
        let sample: serde_json::Value = parse_json(&sample_points(257));
        // Curve splits: at least one null (NaN break), finite within 1e6.
        let pts = sample["points"].as_array().unwrap();
        assert!(pts.iter().any(|p| p[1].is_null()));
        assert!(pts.iter().all(|p| {
            p[1].is_null() || p[1].as_f64().map(|y| y.abs() <= POLE_BREAK).unwrap_or(false)
        }));
        // One vertical asymptote at x≈0 spanning the viewport.
        let asym = sample["asymptotes"].as_array().unwrap();
        assert_eq!(asym.len(), 1, "asymptotes: {asym:?}");
        let x = asym[0]["x"].as_f64().unwrap();
        assert!(x.abs() < 1e-9, "x: {x}");
        let from = asym[0]["from"].as_array().unwrap();
        let to = asym[0]["to"].as_array().unwrap();
        assert!((from[0].as_f64().unwrap() - x).abs() < 1e-12);
        assert!((to[0].as_f64().unwrap() - x).abs() < 1e-12);
        assert!((from[1].as_f64().unwrap() + 10.0).abs() < 1e-12);
        assert!((to[1].as_f64().unwrap() - 10.0).abs() < 1e-12);
        // Existing fields unchanged: points + markers + tangent present.
        assert!(sample.get("points").is_some());
        assert!(sample.get("markers").is_some());
        assert!(sample.get("tangent").is_some());
    }

    #[test]
    fn parabola_and_hidden_curve_emit_no_asymptotes() {
        let _guard = test_lock().lock().unwrap();
        reset_session();
        let _: ParseOk = parse_json(&set_function("f", "x^2"));
        update_viewport(-1.0, 1.0, -10.0, 10.0);
        let sample: serde_json::Value = parse_json(&sample_points(64));
        assert!(sample["asymptotes"].as_array().unwrap().is_empty());
        assert!(!sample["points"].as_array().unwrap().is_empty());
        // Hidden curve: empty points and empty overlay, entry retained.
        let _: ParseOk = parse_json(&set_function("f", "1/x"));
        let _: StyleDto = parse_json(&set_function_visibility("f", false));
        let hidden: serde_json::Value = parse_json(&sample_points(64));
        assert!(hidden["points"].as_array().unwrap().is_empty());
        assert!(hidden["asymptotes"].as_array().unwrap().is_empty());
    }
}
