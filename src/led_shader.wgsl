// Vertex shader

struct CameraUniform {
    view_proj: mat4x4<f32>;
};
[[group(2), binding(0)]]
var<uniform> camera: CameraUniform;

struct UtilUniform {
    time: f32;
    res_width: f32;
    res_height: f32;
};
[[group(1), binding(0)]]
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

[[group(3), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(3), binding(1)]]
var s_diffuse: sampler;

[[group(0), binding(0)]]
var fft_buffer: texture_2d<f32>;
[[group(0), binding(1)]]
var fft_sampler: sampler;

let PI = 3.14159265359;

let bands = 50.0;
let segs = 30.0;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	// Texture coords have 0,0 in top left and 1,1 in bottom right
	// UV has 0,0 in bottom left and 1,1 in top right
    var uv = in.tex_coords;
    uv.y = 1.0 - uv.y;

   // quantize coordinates
    var p = vec2<f32>(0.0);
    p.x = floor(uv.x*bands)/bands;
    p.y = floor(uv.y*segs)/segs;

	//TODO: This method has gaps. If we have 30 bands and a 256 texture, then we should sample 8 values.

    var tex_sample = textureSample(t_diffuse, s_diffuse, in.tex_coords);
	var fft_dimensions = vec2<f32>(textureDimensions(fft_buffer));
    var fft_sample = textureSample(fft_buffer, fft_sampler, vec2<f32>(p.x, 0.25)).r;

    // led color
    var color = mix(vec3<f32>(0.0, 2.0, 0.0), vec3<f32>(2.0, 0.0, 0.0), sqrt(uv.y));

    // mask for bar graph
    var mask = select(1.0, 0.1, p.y > fft_sample);

    // led shape
    var d = fract((uv - p) *vec2<f32>(bands, segs)) - 0.5;
    var led = smoothStep(0.5, 0.35, abs(d.x)) * smoothStep(0.5, 0.35, abs(d.y));
    var ledColor = led * color * mask;

    var srgb = pow(ledColor, vec3<f32>(2.2));
    // output final color
    return vec4<f32>(srgb, 1.0);
}
