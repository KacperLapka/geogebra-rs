// Curve shader: quad-ribbon with the E4-SHADER-CONTRACT solid + dashed paths.
//
// Vertex input per contract (6 attributes, 44-byte stride):
//   location 0: position_math: vec2<f32> (math-space anchor)
//   location 1: normal_dir: vec2<f32> (unit screen-space normal, +/- per side)
//   location 2: color: vec4<f32> (linear-sRGB)
//   location 3: width_px: f32 (CSS px, clamped in-shader to [2.5, 6.0])
//   location 4: dash_flag: f32 (0.0 solid; 1.0 dashed; 2.0 epic 3 only)
//   location 5: arclength_px: f32 (screen-space px at 1x; scaled by dpr in-shader)
//
// Binding 0 keeps the shared ViewportUniform layout; binding 1 carries the
// canvas size (physical px) plus dpr so widths stay correct on hidpi.
// WGSL only, GL-translatable subset: f32 math, no subgroups, no textures.

struct ViewportUniform {
    center: vec2<f32>,
    scale: vec2<f32>,
};

struct CanvasUniform {
    canvas_px: vec2<f32>,
    dpr: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> viewport: ViewportUniform;
@group(0) @binding(1) var<uniform> canvas: CanvasUniform;

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    // Signed across-ribbon offset in device px (+half on even verts, -half
    // on odd verts). Requires the CPU emission order: pairs of opposite
    // verts per sample, in sample order (see expand_ribbon).
    @location(1) u_px: f32,
    @location(2) half_px: f32,
    // Dash flag (0.0 solid, 1.0 dashed, 2.0 reserved) and DPR-scaled
    // arclength in device px (CSS px * dpr) for the screen-space dash.
    @location(3) dash_flag: f32,
    @location(4) arc_dev: f32,
};

@vertex
fn vs_main(
    @location(0) position_math: vec2<f32>,
    @location(1) normal_dir: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) width_px: f32,
    @location(4) dash_flag: f32,
    @location(5) arclength_px: f32,
    @builtin(vertex_index) vid: u32,
) -> VertexOut {
    var out: VertexOut;
    // Width clamps twice (ABI + shader) per E4 conventions.
    let clamped = clamp(width_px, 2.5, 6.0);
    let dpr = max(canvas.dpr, 1.0);
    let half_device = clamped * 0.5 * dpr;
    let safe_px = max(canvas.canvas_px, vec2<f32>(1.0, 1.0));
    // Anchor in NDC, then offset across the ribbon in screen space.
    let ndc = (position_math - viewport.center) * viewport.scale;
    let offset_ndc = normal_dir * half_device * 2.0 / safe_px;
    out.pos = vec4<f32>(ndc + offset_ndc, 0.0, 1.0);
    out.color = color;
    let side = select(-1.0, 1.0, (vid % 2u) == 0u);
    out.u_px = side * half_device;
    out.half_px = half_device;
    // CPU keeps arclength in CSS px so the same buffer is correct at any
    // DPR; scale here so the fragment works in device px.
    out.dash_flag = dash_flag;
    out.arc_dev = arclength_px * dpr;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    // 1px edge antialiasing: full coverage inside, smooth fade at the rim.
    let dist = abs(in.u_px);
    let edge_alpha = 1.0 - smoothstep(max(in.half_px - 1.0, 0.0), in.half_px, dist);
    var dash_alpha = 1.0;
    // <0.5 solid, <1.5 dashed (8 on / 6 off CSS px), >=1.5 reserved
    // (epic 3 owns 2.0): render solid so the reserved path never collides.
    if (in.dash_flag >= 0.5 && in.dash_flag < 1.5) {
        let dpr = max(canvas.dpr, 1.0);
        let period = 14.0 * dpr;
        let on = 8.0 * dpr;
        let fade = 1.0 * dpr;
        let t = fract(in.arc_dev / period) * period;
        if (t < on) {
            dash_alpha = 1.0;
        } else if (t < on + fade) {
            dash_alpha = 1.0 - smoothstep(on, on + fade, t);
        } else if (t < period - fade) {
            discard;
        } else {
            dash_alpha = smoothstep(period - fade, period, t);
        }
    }
    let alpha = edge_alpha * dash_alpha;
    if (alpha <= 0.001) {
        discard;
    }
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
