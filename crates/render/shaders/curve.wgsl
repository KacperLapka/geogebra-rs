// Curve shader: math-space polyline -> clip space via the viewport uniform.
//
// Vertex input is one `vec2<f32>` math point per vertex, drawn as a
// `LineStrip`. Grid and curve share the uniform layout; only the fragment
// color differs. WGSL only per the architecture conventions: wgpu compiles
// to SPIR-V / GL internally, including the WebGL2 fallback path.

struct ViewportUniform {
    center: vec2<f32>,
    scale: vec2<f32>,
};

@group(0) @binding(0) var<uniform> viewport: ViewportUniform;

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs_main(@location(0) world: vec2<f32>) -> VertexOut {
    var out: VertexOut;
    let ndc = (world - viewport.center) * viewport.scale;
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Tracer blue (#2563eb), matching the 2D fallback stroke.
    return vec4<f32>(0.149, 0.388, 0.922, 1.0);
}
