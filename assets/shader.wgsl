// Vertex shader

@group(0) @binding(0)
var<uniform> time: f32;

struct Mouse {
    pos: vec2<f32>,
    state: u32,
}

@group(1) @binding(0)
var<uniform> mouse: Mouse;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

const OUTSIDE_COLOR: vec3<f32> = vec3<f32>(0., 1., 1.);
const INSIDE_COLOR: vec3<f32> = vec3<f32>(1., 0., 1.);

const LINE_DISTANCE: f32 = .25;
const LINE_THICKNESS: f32 = 0.005;

const SUB_LINES: u32 = 4;
const SUB_LINE_THICKNESS: f32 = 0.0025;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

//% include "lib/sdf"
//% include "generated/mouse_state"
//% include "lib/noise/perlin"

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;

    // --- SDFs
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

    var color: vec3<f32>;
    switch mouse.state {
        case Idle, default: {
            color = lines * select(OUTSIDE_COLOR, INSIDE_COLOR, dist < 0.);
        }
        case Clicked: {
            color = lines * select(INSIDE_COLOR, OUTSIDE_COLOR, dist < 0.);
        }
        case Held: {
            color = f32(dist < 0) * vec3<f32>(1.);
        }
    }
    return vec4<f32>(color, 1.);

    // --- Noise
    // let noiseScale = length(mouse.pos) * 10.;
    // var col = 0.;
    // for (var i: i32 = 1; i < 5; i++) {
    //     let offset = (uv + (time / 10.) * vec2f(f32(i), f32(i))) * f32(10 * i);
    //     col -= perlinNoise2(offset) / f32(i);
    // }
    // return vec4<f32>(abs(floor(col)), abs(floor(col)), abs(ceil(col)), 1.);
    
    // --- Use the provided fmod instead of % if you come from glsl.
    // let m = vec2f(fmod(uv.x - .5, .1), fmod(uv.y - .5, .1));
    // return vec4f(m.x, m.y, 0., 1.);
}
