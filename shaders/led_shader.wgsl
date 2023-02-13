// Vertex shader

struct UtilUniform {
    time: f32,
    res_width: f32,
    res_height: f32,
};

@group(0) @binding(0)
var<uniform> util: UtilUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
	@location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) tex_coords: vec2<f32>,
	@location(1) position: vec3<f32>,
};

@vertex
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

@group(1) @binding(0)
var fft_buffer: texture_2d<f32>;
@group(1) @binding(1)
var fft_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	// Texture coords have 0,0 in top left and 1,1 in bottom right
	// UV has 0,0 in bottom left and 1,1 in top right

    let uv = vec2<f32>(in.tex_coords.x, 1.0 - in.tex_coords.y);
	return fs_user(uv);
}

const BANDS = 50.0;
const SEGS = 50.0;

fn fs_user(uv: vec2<f32>) -> vec4<f32> {
   // quantize coordinates
	var p = vec2<f32>(
		floor(uv.x*BANDS)/BANDS,
		floor(uv.y*SEGS)/SEGS
	);

	/* let fft_dimensions = vec2<f32>(textureDimensions(fft_buffer)); */
    let fft_sample = textureSample(fft_buffer, fft_sampler, vec2<f32>(p.x, 0.25)).r;

    // led color
    let color = mix(vec3<f32>(0.0, 2.0, 0.0), vec3<f32>(2.0, 0.0, 0.0), sqrt(uv.y));

    // mask for bar graph
    let mask = select(1.0, 0.1, p.y > fft_sample);

    // led shape
    let d = fract((uv - p) *vec2<f32>(BANDS, SEGS)) - 0.5;
    let led = smoothstep(0.5, 0.35, abs(d.x)) * smoothstep(0.5, 0.35, abs(d.y));
    let ledColor = led * color * mask;

    return vec4<f32>(ledColor, 1.0);
}
