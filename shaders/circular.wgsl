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

const PI = 3.14159265359;

const RAYS: f32 = 128.0; //number of rays //Please, decrease this value if shader is working too slow
const RADIUS: f32 = 0.5; //max circle radius
const RAY_LENGTH: f32 = 0.5; //ray's max length //increased by 0.1

fn fs_user(uv: vec2<f32>) -> vec4<f32> {
    //Prepare UV and background
    let aspect = util.res_width / util.res_height;
    var coord = uv;
    coord.x *= aspect;
    var color = mix(vec4<f32>(0.0, 1.0, 0.8, 1.0), vec4<f32>(0.0, 0.3, 0.25, 1.0), distance(vec2<f32>(aspect/2.0, 0.5), coord));

    color = rays(vec4<f32>(1.0), color, vec2<f32>(aspect/2.0, 1.0/2.0), RADIUS, RAYS, RAY_LENGTH, coord);

    return color;
}

fn rays(
	color: vec4<f32>,
	bg: vec4<f32>,
	position: vec2<f32>,
	radius: f32,
	rays: f32,
	ray_length: f32,
	uv: vec2<f32>)
-> vec4<f32> {
	var background = bg;
    let inside = (1.0 - ray_length) * radius; //empty part of circle
    let outside = radius - inside; //rest of circle
    let circle = 2.0*PI*inside; //circle lenght
    for(var i: i32 = 1; f32(i) <= rays; i++)
    {
        let len = outside * textureSample(fft_buffer, fft_sampler, vec2<f32>(f32(i)/rays, 0.0)).x; //length of actual ray
        background = bar(color, background, vec2<f32>(position.x, position.y+inside), vec2<f32>(circle/(rays*2.0), len), rotate(uv, position, 360.0/rays*f32(i))); //Added capsules
    }
    return background; //output
}

fn bar(color: vec4<f32>, background: vec4<f32>, position: vec2<f32>, diemensions: vec2<f32>, uv: vec2<f32>) -> vec4<f32>
{
    return capsule(color, background, vec4<f32>(position.x, position.y+diemensions.y/2.0, diemensions.x/2.0, diemensions.y/2.0), uv); //Just transform rectangle a little
}

fn capsule(color: vec4<f32>, background: vec4<f32>, region: vec4<f32>, uv: vec2<f32>) -> vec4<f32>
{
    if(uv.x > (region.x-region.z) && uv.x < (region.x+region.z) &&
       uv.y > (region.y-region.w) && uv.y < (region.y+region.w) ||
       distance(uv, region.xy - vec2<f32>(0.0, region.w)) < region.z ||
       distance(uv, region.xy + vec2<f32>(0.0, region.w)) < region.z) {
        return color;
	}
    return background;
}

fn rotate(p: vec2<f32>, center: vec2<f32>, angle: f32) -> vec2<f32> //rotating point around the center
{
	var point = p;
    let s = sin(radians(angle));
    let c = cos(radians(angle));

    point.x -= center.x;
    point.y -= center.y;

    let x = point.x * c - point.y * s;
    let y = point.x * s + point.y * c;

    point.x = x + center.x;
    point.y = y + center.y;

    return point;
}
