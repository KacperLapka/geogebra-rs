// Grid shader: axis lines in the same viewport-uniform space as the curve.
//
// Kept as a separate module so grid styling (dashed subdivisions, axis
// emphasis) can evolve without touching the curve pipeline. WGSL only.

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
    // Neutral grid gray at low alpha; Svelte draws tick labels as DOM text.
    return vec4<f32>(0.533, 0.533, 0.533, 0.35);
}
