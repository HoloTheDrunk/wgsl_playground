// Floored modulo since '%' is a truncated modulo in wgsl...
// All my homies hate truncated modulo.
fn fmod(lhs: f32, rhs: f32) -> f32 {
    return ((lhs % rhs) + rhs) % rhs;
}

fn fmod2(lhs: vec2f, rhs: f32) -> vec2f {
    return vec2f(fmod(lhs.x, rhs), fmod(lhs.y, rhs));
}

fn fmod2v(lhs: vec2f, rhs: vec2f) -> vec2f {
    return vec2f(
        fmod(lhs.x, rhs.x),
        fmod(lhs.y, rhs.y),
    );
}

fn fmod3(lhs: vec3f, rhs: f32) -> vec3f {
    let v2 = fmod2(lhs.xy, rhs);
    return vec3f(v2.x, v2.y, fmod(lhs.z, rhs));
}

fn fmod3v(lhs: vec3f, rhs: vec3f) -> vec3f {
    let v2 = fmod2v(lhs.xy, rhs.xy);
    return vec3f(v2.x, v2.y, fmod(lhs.z, rhs.z));
}

fn fmod4(lhs: vec4f, rhs: f32) -> vec4f {
    let v3 = fmod3(lhs.xyz, rhs);
    return vec4f(v3.x, v3.y, v3.z, fmod(lhs.w, rhs));
}

fn fmod4v(lhs: vec4f, rhs: vec4f) -> vec4f {
    let v3 = fmod3v(lhs.xyz, rhs.xyz);
    return vec4f(v3.x, v3.y, v3.z, fmod(lhs.w, rhs.w));
}
