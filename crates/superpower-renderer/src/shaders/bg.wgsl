struct BgVertexInput {
    @location(0) position: vec2f,
    @location(1) rect_pos: vec2f,
    @location(2) rect_size: vec2f,
    @location(3) color: vec3f,
    @location(4) radius: f32,
}

struct BgVertexOutput {
    @builtin(position) position: vec4f,
    @location(0) rect_pos: vec2f,
    @location(1) rect_size: vec2f,
    @location(2) color: vec3f,
    @location(3) radius: f32,
}

@vertex
fn vs_main(in: BgVertexInput) -> BgVertexOutput {
    var out: BgVertexOutput;
    out.position = vec4f(in.position, 0.0, 1.0);
    out.rect_pos = in.rect_pos;
    out.rect_size = in.rect_size;
    out.color = in.color;
    out.radius = in.radius;
    return out;
}

fn rounded_rect_alpha(position: vec2f, rect_pos: vec2f, rect_size: vec2f, radius: f32) -> f32 {
    if radius <= 0.0 {
        return 1.0;
    }

    let center = rect_pos + rect_size * 0.5;
    let half_size = rect_size * 0.5 - vec2f(radius, radius);
    let q = abs(position - center) - half_size;
    let dist = length(max(q, vec2f(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
    return 1.0 - smoothstep(0.0, 1.25, dist);
}

@fragment
fn fs_main(in: BgVertexOutput) -> @location(0) vec4f {
    let alpha = rounded_rect_alpha(in.position.xy, in.rect_pos, in.rect_size, in.radius);
    return vec4f(in.color, alpha);
}
