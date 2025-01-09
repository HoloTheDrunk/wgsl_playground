// Vertex shader
//% include "lib/utils/gen_triangle_vs"

// Fragment shader
//% include "lib/sdf"

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

struct FontAtlas {
    size: vec2<u32>,
}

struct Ui {
    data: array<UiElement, SIZE>,
}

struct UiElement {
    pos: vec2f,
}

@group(1) @binding(0)
var<uniform> atlas: FontAtlas;
@group(1) @binding(1)
var<uniform> ui: Ui;

@group(2) @binding(0)
var t_atlas: texture_2d<f32>;
@group(2) @binding(1)
var s_atlas: sampler;

fn @fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = vec2f(in.tex_coords.x, in.tex_coords.y);
    // var color = textureSample(t_diffuse, s_diffuse, uv);
    // color = select(
    //     color,
    //     mix(color, vec4f({r:?}, {g:?}, {b:?}, 1.), {a:?}),
    //     dist < 0.,
    // );
    return vec4f(uv.x, uv.y, 0., 1.);
}
