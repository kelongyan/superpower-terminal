struct BgVertexInput {
    @location(0) position: vec2f,
    @location(1) color: vec3f,
}

struct BgVertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec3f,
}

@vertex
fn vs_main(in: BgVertexInput) -> BgVertexOutput {
    var out: BgVertexOutput;
    out.position = vec4f(in.position, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: BgVertexOutput) -> @location(0) vec4f {
    return vec4f(in.color, 1.0);
}
