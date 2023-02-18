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

// Here we do log scaling on the frequency axis.

fn fs_user(uvv: vec2<f32>) -> vec4<f32> {
	var uv = uvv;
    var xPos: f32;
    var fft: f32;

    if (uv.y > 0.5){

        //linear sampling
        xPos = uv.x;
        fft = getLevel(xPos);

    }else{

        //crop bottom and top of range
        uv.x = mix(0.3,0.7, uv.x);

        //logarithmic sampling
        xPos = toLog(uv.x, 0.01, 1.0);

        fft = getLevel(xPos);

        //boost contrast
        fft = pow(fft,3.0);

        //boost gain
        fft *= 1.5;

        //contrast / brightness
        let contrast = 1.4;
        let brightness = 0.;
        fft = (fft - 0.5) * contrast + 0.5 + brightness;

    }

    let color = vec4<f32>(vec3<f32>(fft), 1.0);
    return color;
}
/*

	Linear vs Logarithmic FFT

	some good test songs:

	https://soundcloud.com/kraddy/winning
	https://soundcloud.com/grey-houston/soothing-piano-melody
	https://soundcloud.com/pointpoint/life-in-gr

*/

//from https://stackoverflow.com/questions/35799286
fn toLog(value: f32, min: f32, max: f32) -> f32{
	let exp = (value-min) / (max-min);
	return min * pow(max/min, exp);
}

fn getLevel(samplePos: f32) -> f32{
    return textureSample(fft_buffer, fft_sampler, vec2<f32>(samplePos, 0.25)).r;
}
