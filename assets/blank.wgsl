// Vertex shader
//% include "lib/utils/gen_triangle_vs"

// Fragment shader

//% include "lib/sdf"
//% include "generated/mouse_state"

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@group(1) @binding(0)
var<uniform> time: f32;

struct Mouse {
    pos: vec2<f32>,
    state: u32,
}

@group(2) @binding(0)
var<uniform> mouse: Mouse;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = vec2f(in.tex_coords.x, in.tex_coords.y);
    return vec4f(uv, 1., 1.);
}
