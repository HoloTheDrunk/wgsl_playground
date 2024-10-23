fn rand11(f: f32) -> f32 { return f32(hash11(bitcast<u32>(f))) / f32(0xffffffff); }
fn rand22(f: vec2f) -> vec2f { return vec2f(hash22(bitcast<vec2u>(f))) / f32(0xffffffff); }
fn rand33(f: vec3f) -> vec3f { return vec3f(hash33(bitcast<vec3u>(f))) / f32(0xffffffff); }
fn rand44(f: vec4f) -> vec4f { return vec4f(hash44(bitcast<vec4u>(f))) / f32(0xffffffff); }

// On generating random numbers, with help of y= [(a+x)sin(bx)] mod 1", W.J.J. Rey, 22nd European Meeting of Statisticians 1998
fn rand11_sin(n: f32) -> f32 { return fract(sin(n) * 43758.5453123); }
fn rand22_sin(n: vec2f) -> f32 { return fract(sin(dot(n, vec2f(12.9898, 4.1414))) * 43758.5453); }
