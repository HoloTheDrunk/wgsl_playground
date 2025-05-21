// Vertex shader



struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@group(1) @binding(0)
var<uniform> time: f32;

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
    @builtin(vertex_index) in_vertex_index: u32,
    @builtin(instance_index) in_instance_index: u32,
) -> VertexOutput {
    var model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var angle = time / f32(2 * (in_instance_index + 1));
    var time_rot_mat = mat4x4<f32>(
        vec4f(cos(angle), -sin(angle), 0., 0.),
        vec4f(sin(angle), cos(angle), 0., 0.),
        vec4f(0., 0., 1., 0.),
        vec4f(0., 0., 0., 1.),
    );

    var out: VertexOutput;

    out.color = instance.color;

    var pos = model.position + vec3f(f32(in_vertex_index), 0., 0.);
    out.clip_position = model_matrix * time_rot_mat * vec4<f32>(model.position / 10., 1.0);

    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
