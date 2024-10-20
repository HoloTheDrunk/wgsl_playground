// Vertex shader

@group(0) @binding(0)
var<uniform> time: f32;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

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

fn disc(point: vec2<f32>, center: vec2<f32>, radius: f32) -> f32 {
    let dist = distance(point, center);
    return (dist - radius) / radius;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = disc(in.tex_coords, vec2<f32>(0.5, 0.5), .4);

    let col = f32(abs(d) < 0.05) + abs(d) * f32(d < -0.05);
    return vec4<f32>(col, col, col, 1.);
}
