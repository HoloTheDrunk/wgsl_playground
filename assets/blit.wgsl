// Vertex shader
//% include "lib/utils/gen_triangle_vs"

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let col = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let sum = col.r + col.g + col.b;
    let val = select(0., 1., sum != 0);

    return vec4f(val, val, val, 1.);
    // return vec4f(in.tex_coords.x, in.tex_coords.y, 0., 1.);
}
