struct FgVertexInput {
    @location(0) position: vec2f,
    @location(1) tex_coords: vec2f,
    @location(2) color: vec3f,
}

struct FgVertexOutput {
    @builtin(position) position: vec4f,
    @location(0) tex_coords: vec2f,
    @location(1) color: vec3f,
}

@group(0) @binding(0)
var glyph_texture: texture_2d<f32>;

@group(0) @binding(1)
var glyph_sampler: sampler;

@vertex
fn vs_main(in: FgVertexInput) -> FgVertexOutput {
    var out: FgVertexOutput;
    out.position = vec4f(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: FgVertexOutput) -> @location(0) vec4f {
    let alpha = textureSample(glyph_texture, glyph_sampler, in.tex_coords).r;
    return vec4f(in.color, alpha);
}
