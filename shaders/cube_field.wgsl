// Based on
// https://www.shadertoy.com/view/ll2SRy


// Cheap vec3<f32> to vec3<f32> hash. Works well enough, but there are other ways.
fn hash33(p: vec3<f32>) -> vec3<f32> {

    let n = sin(dot(p, vec3<f32>(7., 157., 113.)));
    return fract(vec3<f32>(2097152., 262144., 32768.)*n);
}

fn map_range(value: f32, min_in: f32, max_in: f32, min_out: f32, max_out: f32) -> f32 {
  return min_out + (value - min_in) * (max_out - min_out) / (max_in - min_in);
}

fn map(pp: vec3<f32>) -> f32 {
	var p = pp;
	// Creating the repeat cubes, with slightly convex faces. Standard,
    // flat faced cubes don't capture the light quite as well.

    // Cube center offset, to create a bit of disorder, which breaks the
    // space up a little.
    let o = hash33(floor(p))*0.2;

    // 3D space repetition.
    p = fract(p + o) - 0.5;

    // A bit of roundness. Used to give the cube faces a touch of convexity.
    let r = dot(p, p) - 0.21;

    // Max of abs(x), abs(y) and abs(z) minus a constant gives a cube.
    // Adding a little bit of "r," above, rounds off the surfaces a bit.
    p = abs(p);
	return max(max(p.x, p.y), p.z)*.95 + r*0.05 - map_range(fft_sample(0.2,0), 0., 1., 0.0, 0.5);


    // Alternative. Egg shapes... kind of.
	//p = pp;
    //let perturb = sin(p.x*10.)*sin(p.y*10.)*sin(p.z*10.);
	//p += hash33(floor(p))*.2;
	//return length(fract(p)-(.5))-(0.25) + perturb*0.05;

}


fn fs_user(coord: vec2<f32>) -> vec3<f32> {
    let aspect = util.res_width / util.res_height;
    var uv = (coord - 0.5) * vec2<f32>(aspect, 1.);

    // Unit direction ray. The last term is one of many ways to fish-lens the camera.
    // For a regular view, set "rd.z" to something like "0.5."
	let fish = (1. - dot(uv, uv)*.5)*.5;
    var rd = normalize(vec3<f32>(
		uv,
		fish
	)); // Fish lens, for that 1337, but tryhardish, demo look. :)

    // there are a few ways to hide artifacts and inconsistencies. making things go fast is one of them. :)
    // ray origin, scene color, and surface postion vector.
    let ro = vec3<f32>(0., 0., util.time * 3.);
	var col = vec3<f32>(0.0);
	var sp = vec3<f32>(0.0);

    // Swivel the unit ray to look around the scene.
	let cs = cos( util.time * 0.375 );
	let si = sin( util.time * 0.375 );
	let rdxz = mat2x2<f32>(cs, si,-si, cs)*rd.xz;
	rd.x = rdxz.x;
	rd.z = rdxz.y;
	let rdxy = mat2x2<f32>(cs, si,-si, cs)*rd.xy;
	rd.x = rdxy.x;
	rd.y = rdxy.y;

    // Unit ray jitter is another way to hide artifacts. It can also trick the viewer into believing
    // something hard core, like global illumination, is happening. :)
    rd *= 0.985 + hash33(rd) * 0.03;


	// Ray distance, bail out layer number, surface distance and normalized accumulated distance.
	var t: f32 = 0.;
	var layers: i32 = 0;
	var d = 0.;
	var aD = 0.;

    // Surface distance threshold. Smaller numbers give a sharper object. I deliberately
    // wanted some blur, so bumped it up slightly.
    var thD = .035; // + smoothstep(-0.2, 0.2, sin(iTime*0.75 - 3.14159*0.4))*0.025;

    // Only a few iterations seemed to be enough. Obviously, more looks better, but is slower.
	for(var i=0; i<56; i++)	{

        // Break conditions. Anything that can help you bail early usually increases frame rate.
        if(layers > 15 || col.x>1. || t>10.) {
			break;
		}

        // Current ray postion. Slightly redundant here, but sometimes you may wish to reuse
        // it during the accumulation stage.
        sp = ro + rd * t;

        d = map(sp); // Distance to nearest point in the cube field.

        // If we get within a certain distance of the surface, accumulate some surface values.
        // Values further away have less influence on the total.
        //
        // aD - Accumulated distance. I interpolated aD on a whim (see below), because it seemed
        // to look nicer.
        //
        // 1/.(1. + t*t*.25) - Basic distance attenuation. Feel free to substitute your own.

         // Normalized distance from the surface threshold value to our current isosurface value.
		aD = (thD-abs(d)*15./16.)/thD;


        // If we're within the surface threshold, accumulate some color.
        // Two "if" statements in a shader loop makes me nervous. I don't suspect there'll be any
        // problems, but if there are, let us know.
        if(aD>0.) {
            // Smoothly interpolate the accumulated surface distance value, then apply some
            // basic falloff (fog, if you prefer) using the camera to surface distance, "t."
			col += aD*aD*(3. - 2.*aD)/(1. + t*t*.25)*.2;
            layers++;
        }


        // Kind of weird the way this works. I think not allowing the ray to hone in properly is
        // the very thing that gives an even spread of values. The figures are based on a bit of
        // knowledge versus trial and error. If you have a faster computer, feel free to tweak
        // them a bit.
        t += max( abs(d) * 0.7, thD * 1.5);
	}

    // I'm virtually positive "col" doesn't drop below zero, but just to be safe...
    col = max(col, vec3<f32>(0.));

    // Mixing the greytone color and some firey orange with a sinusoidal pattern that
    // was completely made up on the spot.
    col = mix(col, vec3<f32>(min(col.x*1.5, 1.), pow(col.x, 2.5), pow(col.x, 12.)),
              dot(sin(rd.yzx*8. + sin(rd.zxy*8.)), vec3<f32>(.1666))+0.4);


	// Doing the same again, but this time mixing in some green. I might have gone overboard
    // applying this step. Commenting it out probably looks more sophisticated.
    /* col = mix(col, vec3<f32>(col.x*col.x*.85, col.x, col.x*col.x*.3), */
    /*          dot(sin(rd.yzx*4. + sin(rd.zxy*4.)), vec3<f32>(.1666)) + .25); */


	// Presenting the color to the screen -- Note that there isn't any gamma correction. That
    // was a style choice.
	/* return vec3<f32>(rd); */
	return clamp(col, vec3<f32>(0.), vec3<f32>(1.));
 }

