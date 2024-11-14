// Vertex shader
//% include "lib/utils/gen_triangle_vs"

// Fragment shader

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
    let col = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let pair = col.g + col.b;
    let b_prio = col.b / (pair + col.r);
    let res = vec4f((pair + col.r) / 3., pair / 2., pair / 2., col.a);

    return res;
}
