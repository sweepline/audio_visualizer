// Vertex shader bindings

struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
    @builtin(position) position: vec4<f32>,
};

struct Locals {
    screen_size: vec2<f32>,
    padding: vec2<f32>
};
@group(0) @binding(0) var<uniform> r_locals: Locals;

@vertex
fn vs_main(
    @location(0) a_pos: vec2<f32>,
    @location(1) a_tex_coord: vec2<f32>,
    @location(2) a_color: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coord = a_tex_coord;

    // [u8; 4] RGB as u32 -> [r, g, b, a]
    let color = vec4<f32>(
        f32(a_color & 255u),
        f32((a_color >> 8u) & 255u),
        f32((a_color >> 16u) & 255u),
        f32((a_color >> 24u) & 255u),
    );
    out.color = vec4<f32>(color.rgba / 255.0);

    out.position = vec4<f32>(
        2.0 * a_pos.x / r_locals.screen_size.x - 1.0,
        1.0 - 2.0 * a_pos.y / r_locals.screen_size.y,
        0.0,
        1.0,
    );

    return out;
}

// Fragment shader bindings

@group(1) @binding(0) var r_tex_color: texture_2d<f32>;
@group(1) @binding(1) var r_tex_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color * textureSample(r_tex_color, r_tex_sampler, in.tex_coord);
}

