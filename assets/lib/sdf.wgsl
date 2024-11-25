// SDF Shapes

fn disc(pos: vec2<f32>, center: vec2<f32>, radius: f32) -> f32 {
    let dist = distance(pos, center);
    return dist - radius;
}

fn rectangle(pos: vec2<f32>, center: vec2<f32>, half_size: vec2<f32>) -> f32 {
    let edge_distances = abs(pos - center) - half_size;
    let outside_distance = length(max(edge_distances, vec2<f32>(0, 0)));
    let inside_distance = min(max(edge_distances.x, edge_distances.y), 0.);
    return outside_distance + inside_distance;
}

// SDF Operations

fn merge(shape1: f32, shape2: f32) -> f32 {
    return min(shape1, shape2);
}

fn intersect(shape1: f32, shape2: f32) -> f32 {
    return max(shape1, shape2);
}

fn subtract(base: f32, shape: f32) -> f32 {
    return intersect(base, -shape);
}

fn interpolate(shape1: f32, shape2: f32, amount: f32) -> f32 {
    return mix(shape1, shape2, amount);
}

fn round_merge(shape1: f32, shape2: f32, radius: f32) -> f32 {
    let intersection_space = min(
        vec2<f32>(shape1 - radius, shape2 - radius),
        vec2<f32>(0., 0.)
    );
    
    let inside_distance = -length(intersection_space);
    let simple_union = merge(shape1, shape2);
    let outside_distance = max(simple_union, radius);

    return inside_distance + outside_distance;
}

fn round_intersect(shape1: f32, shape2: f32, radius: f32) -> f32 {
    let intersection_space = max(
        vec2f(shape1 + radius, shape2 + radius),
        vec2f(0., 0.),
    );

    let outside_distance = length(intersection_space);
    let simple_intersection = intersect(shape1, shape2);
    let inside_distance = min(simple_intersection, -radius);

    return outside_distance + inside_distance;
}

fn round_subtract(base: f32, shape: f32, radius: f32) -> f32 {
    return round_intersect(base, -shape, radius);
}
