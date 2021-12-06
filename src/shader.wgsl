// Vertex shader

[[block]]
struct CameraUniform {
    view_proj: mat4x4<f32>;
};
[[group(1), binding(0)]]
var<uniform> camera: CameraUniform;

[[block]]
struct UtilUniform {
    time: f32;
    resolution: vec2<u32>;
};
[[group(2), binding(0)]]
var<uniform> util: UtilUniform;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
    [[location(1)]] position: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    out.position = vec3<f32>(model.position);
    return out;
}

// Fragment shader

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var uv = in.tex_coords;
    uv.y = 1.0 - uv.y;
    var col = 0.5 + 0.5 * cos(vec3<f32>(util.time) + uv.xyx + vec3<f32>(0.0, 2.0, 4.0));
    // var col = vec3<f32>(uv, 0.0);

    var tex_sample = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    var srgb = pow(col, vec3<f32>(2.2));
    return vec4<f32>(srgb, 1.0);
    // return tex_sample;
}