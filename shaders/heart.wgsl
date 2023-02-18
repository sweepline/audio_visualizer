// Derived from https://www.shadertoy.com/view/WllyzM
fn GetGraphDist(pp: vec3<f32>) -> f32 {
    // Switch Y and Z
    let p = pp.xzy;

    // Equation: (x^2 + 9/4 * y^2 + z^2 - 1)^3 - x^2 * z^3 - 9/80 * y^2 * z^3 = 0
    // -3 <= x,y,z <= 3
    let m = (p.x*p.x) + (9.0/4.0 * p.y*p.y) + (p.z*p.z) - 1.0;
    let d = m*m*m -
        (p.x*p.x * p.z*p.z*p.z) -
        (9.0/80.0 * p.y*p.y * p.z*p.z*p.z);

    return d;
}

const arrowThickness: f32 = 0.3;
fn Arrow(
	rayOrigin: vec3<f32>,
	rayDirection: vec3<f32>,
	newP_in: vec3<f32>,
	offset: f32,
	invTip:  f32
) -> vec3<f32> {
	var newP = newP_in;
    // Move arrow position
	newP.z += 2.3 + offset;

    // Center position
    newP.x -= 0.025;

    // See if intersection point lays within the arrow
    if(abs(newP.x / newP.z) <= arrowThickness &&
       newP.z * 8.0/3.0 * invTip <= 1.0 &&
       newP.z * invTip >= 0.0)
    {
        return vec3<f32>(0.0);
    }

    return vec3<f32>(1.0);
}

fn AllArrows(rayOrigin: vec3<f32>, rayDirection: vec3<f32>) -> vec3<f32>
{
	var col = vec3<f32>(1.0);

    // 2 arrows in xz-plane
	var t = (0.0 - rayOrigin.y) / rayDirection.y;
    var newP = rayOrigin + t * rayDirection;
    col *= Arrow(rayOrigin, rayDirection, newP.xyz, 0.3, 1.0);
    col *= Arrow(rayOrigin, rayDirection, newP.zyx, 0.0, 1.0);

    // 1 arrow in yz-plane
	t = (0.0 - rayOrigin.z) / rayDirection.z;
    newP = rayOrigin + t * rayDirection;
    col *= Arrow(rayOrigin, rayDirection, newP.xzy, -4.6, -1.0);

    return col;
}

fn AllAxes(rayOrigin: vec3<f32>, rayDirection: vec3<f32>) -> vec3<f32>
{
    let thickness = 0.05;

    // X-axis
    var ty = (0.0 - rayOrigin.y) / rayDirection.y;
    var newPy = rayOrigin + ty * rayDirection;
    if(newPy.x <= 2.0 && newPy.x >= -2.0 && newPy.z <= 0.0 && newPy.z >= -thickness)
    {
        return vec3<f32>(0.0);
    }

    // Y-axis
    let tz = (0.0 - rayOrigin.z) / rayDirection.z;
    let newPz = rayOrigin + tz * rayDirection;
    if(newPz.x <= thickness && newPz.x >= 0.0 && newPz.y <= 2.0 && newPz.y >= -2.0)
    {
        return vec3<f32>(0.0);
    }

    // Z-axis
    ty = (0.0 - rayOrigin.y) / rayDirection.y;
    newPy = rayOrigin + ty * rayDirection;
    if(newPy.x <= thickness && newPy.x >= 0.0 && newPy.z <= 2.0 && newPy.z >= -2.5)
    {
        return vec3<f32>(0.0);
    }

    return vec3<f32>(1.0);
}

// Extreme values for a good quality
//const f32 STEP_SIZE = 0.001;
//const f32 MAX_NUM_STEPS = 4400.0;
//const f32 HIT_DIST = 0.000001;
const STEP_SIZE = 0.003;
const MAX_NUM_STEPS = 2200.0;
const HIT_DIST = 0.000001;
fn fs_user(uvv: vec2<f32>) -> vec3<f32> {
    let aspect = util.res_width / util.res_height;
    var uv = uvv;
	uv -= vec2<f32>(0.5, 0.5);
    uv.x *= aspect;

    // Camera setup
    let camOrigin = vec3<f32>(1.0 + sin(util.time)*0.1, 2.0, -4.0) * 0.95;
    let camLookAt = vec3<f32>(0.0, 0.4, 0.0);

    let camForward = normalize(camLookAt - camOrigin);
    let camRight = normalize(cross(camForward, vec3<f32>(0.0, 1.0, 0.0)));
    let camUp = normalize(cross(camRight, camForward));

    let zoom = 1.0;
    var rayOrigin = camOrigin + camForward * zoom + camRight * uv.x + camUp * uv.y;
    let rayDirection = normalize(rayOrigin - camOrigin);


    var col = vec3<f32>(1.0);
    var p = rayOrigin;

    // Start ray marching
    for(var i = 0.0; i <= MAX_NUM_STEPS; i += 1.0)
    {
        let currDist = GetGraphDist(p);

        if(currDist <= HIT_DIST)
        {
            // Shade
            let n = normalize(p);
            let l = normalize(vec3<f32>(-0.4, -1.0, 3.0));
            col = vec3<f32>(1.0, 0.0, 0.0) * max(dot(n, -l), 0.2);

            // Lines on the heart
            let rps = 6.0;
            p.z += 0.03;
            let rp = round(p*rps);
            let diff = abs(p*rps-rp);
            let size = 0.03;

            if(diff.x < size || diff.y < size || diff.z < size) {
                col = vec3<f32>(0.0);
			}

            break;
        }

        // March
        p += rayDirection * STEP_SIZE;


        // Make sure the point is within the bounding box
        if(abs(p.x) > 3.0 || abs(p.y) > 3.0 || abs(p.z) > 3.0) {
            break;
		}
    }

    // Move axes back a bit
    rayOrigin += vec3<f32>(0.0, 0.0, -0.4);

    // Render axes and arrows
    col *= AllAxes(rayOrigin, rayDirection);
    col *= AllArrows(rayOrigin, rayDirection);

    return col;
}
