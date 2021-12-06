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
    res_width: f32;
    res_height: f32;
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
};

[[stage(vertex)]]
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// camera attributes
// cameraDirection and cameraUp MUST be normalized
// (ie. their length must be equal to 1)
let cameraPosition = vec3<f32>(0.0, 0.0, 10.0);
let cameraDirection = vec3<f32>(0.0, 0.0, -1.0);
let cameraUp = vec3<f32>(0.0, 1.0, 0.0);

// ray computation vars
let PI: f32 = 3.14159265359;
let fov: f32 = 50.0;

fn distanceToNearestSurface(p: vec3<f32>) -> f32 {
    return length(p) - 1.0 * (sin(util.time) + 1.0);
}

fn intersectsWithWorld(p: vec3<f32>, dir: vec3<f32>) -> bool {
  	var dist = 0.0;
    var nearest = 0.0;
    var hit = false;
    for(var i: i32 = 0; i < 20; i = i + 1){
        var nearest = distanceToNearestSurface(p + dir*dist);
        if(nearest < 0.01){
            hit = true;
            break;
        }
        dist = dist + nearest;
    }
    return hit;
}

// Fragment shader

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var resolution = vec2<f32>(util.res_width, util.res_height);
    var uv: vec2<f32> = in.clip_position.xy / resolution;
    uv.y = 1.0 - uv.y;
    // generate the ray for this pixel
    var fovx: f32 = PI * fov / 360.0;
    var fovy = fovx * util.res_height / util.res_width;
    var ulen = tan(fovx);
	var vlen = tan(fovy);
    var camUV: vec2<f32> = uv * 2.0 - vec2<f32>(1.0, 1.0);
    var nright: vec3<f32> = normalize(cross(cameraUp, cameraDirection));
    var pixel: vec3<f32> = cameraPosition + cameraDirection + nright * camUV.x * ulen + cameraUp * camUV.y * vlen;
    var rayDirection: vec3<f32> = normalize(pixel - cameraPosition);
    
    var collidedWithWorld = 0.0;
    if(intersectsWithWorld(cameraPosition, rayDirection)) {
        collidedWithWorld = 1.0;
    }
	var color = vec3<f32>(collidedWithWorld, 0.0, 0.0);

    var srgb = pow(color, vec3<f32>(2.2));
    return vec4<f32>(srgb, 1.0);
}