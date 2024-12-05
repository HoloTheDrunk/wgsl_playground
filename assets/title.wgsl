// Vertex shader
//% include "lib/utils/gen_triangle_vs"

// Fragment shader

//% include "lib/sdf"
//% include "generated/mouse_state"
//% include "lib/noise/perlin"

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

// SDF example constants
const OUTSIDE_COLOR: vec3<f32> = vec3<f32>(0., 0., 0.);
const INSIDE_COLOR: vec3<f32> = vec3<f32>(.886, .875, .824);

const LINE_DISTANCE: f32 = .25;
const LINE_THICKNESS: f32 = 0.005;

const SUB_LINES: u32 = 4;
const SUB_LINE_THICKNESS: f32 = 0.0025;

fn sdf_example(uv: vec2f) -> vec4f {
    let box_left = vec2<f32>(.5 - .15, .5);
    var shapes = array<f32, 3>(
        disc(uv, mouse.pos, .1),
        disc(uv, box_left + vec2<f32>(.3, .1), .1),
        rectangle(uv, box_left + vec2<f32>(.15, 0), vec2<f32>(.15, .1)),
    );

    var dist: f32 = shapes[0];
    for (var i: i32 = 1; i < 3; i++) {
        dist = round_merge(dist, shapes[i], .05);
    }

    let distance_change = fwidth(dist) * 0.5;

    let major_line_distance = abs(fract(dist / LINE_DISTANCE + .5) - .5) * LINE_DISTANCE;
    let major_lines = smoothstep(LINE_THICKNESS - distance_change, LINE_THICKNESS + distance_change, major_line_distance);

    let distance_between_sub_lines = LINE_DISTANCE / f32(SUB_LINES);
    let sub_line_distance = abs(fract(dist / distance_between_sub_lines + .5) - 0.5) * distance_between_sub_lines;
    let sub_lines = smoothstep(SUB_LINE_THICKNESS - distance_change, SUB_LINE_THICKNESS + distance_change, sub_line_distance);

    let lines = min(major_lines, sub_lines);

    var color = select(
        lines * select(OUTSIDE_COLOR, INSIDE_COLOR, dist < 0.), 
        2. * (0.5 - lines) * INSIDE_COLOR,
        dist > LINE_THICKNESS && abs(lines) <= 0.5,
    );
  
    return vec4<f32>(color, 1.);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = vec2f(in.tex_coords.x, in.tex_coords.y);

    return sdf_example(uv);
}
