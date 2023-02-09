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

let BAR_COLOR = vec3<f32>(0.8, 0.2, 0.3);
let BAR_WIDTH = 0.75;
let PI = 3.14159265359;


[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	// Texture coords have 0,0 in top left and 1,1 in bottom right
	// UV has 0,0 in bottom left and 1,1 in top right
    var uv = in.tex_coords;
    uv.y = 1.0 - uv.y;

    var tex_sample = textureSample(t_diffuse, s_diffuse, in.tex_coords);
	var fft_dimensions = vec2<f32>(textureDimensions(fft_buffer));
    var fft_sample = textureSample(fft_buffer, fft_sampler, vec2<f32>(uv.x, 0.25)).r;

	// Bars
	var horizontal = smoothStep(1.0 - BAR_WIDTH, 1.0 - BAR_WIDTH, abs(sin(uv.x * PI * fft_dimensions.x)));
	var vertical = smoothStep(1.0 - fft_sample, 1.0 - fft_sample, 1.0 - uv.y);
	var col2 = vec3<f32>(BAR_COLOR * horizontal * vertical);

	// Basic white output
	var col = vec3<f32>(step(uv.y, fft_sample));

	col = pow(col, vec3<f32>(1.0/2.2));
	return vec4<f32>(col, 1.0);
}
