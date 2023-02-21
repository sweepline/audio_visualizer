// Based on
// https://www.shadertoy.com/view/4ddSDS
// Created by Stephane Cuillerdier - @Aiekick/2016

fn getRotZMat(a: f32) -> mat3x3<f32> {
	return mat3x3<f32>(
		cos(a), -sin(a), 0.,
		sin(a),  cos(a), 0.,
		0.,      0.,     1.
	);
}

fn modu(x: f32, y: f32) -> f32 {
	return x - y * floor(x/y);
}

fn map(pp: vec3<f32>) -> f32 {
	var p: vec3<f32> = pp;
	p.x += sin(p.z*1.8);
    p.y += cos(p.z*.2) * sin(p.x*.8);
	p *= getRotZMat(p.z*0.8+sin(p.x)+cos(p.y));
    p.x = modu(p.x, 0.3) - 0.15;
    p.y = modu(p.y, 0.3) - 0.15;
	return length(p.xy);
}

fn fs_user(coord: vec2<f32>) -> vec3<f32> {
    let aspect = util.res_width / util.res_height;
	// Make uv -0.5 to 0.5
    var uv = (coord - 0.5) * vec2<f32>(aspect, 1.);

	var dstepf: f32 = 0.0;

    // Unit direction ray. The last term is one of many ways to fish-lens the camera.
    // For a regular view, set "rd.z" to something like "0.5."
	let fish = (1. - dot(uv, uv)*.5)*.5;
    var rd = normalize(vec3<f32>(
		uv,
		fish
	)); // Fish lens, for that 1337, but tryhardish, demo look. :)

    // there are a few ways to hide artifacts and inconsistencies. making things go fast is one of them. :)
    // ray origin, scene color, and surface postion vector.
    let ro = vec3<f32>(0., 0., util.time * 1.26);
	var col = vec3<f32>(0.0);
	var sp = vec3<f32>(0.0);

    // Swivel the unit ray to look around the scene.
	let cs = cos( util.time * 0.275 );
	let si = sin( util.time * 0.275 );
	let rdxz = mat2x2<f32>(cs, si,-si, cs)*rd.xz;
	rd.x = rdxz.x;
	rd.z = rdxz.y;

	var t: f32 = 0.06;
	var layers: i32 = 0;
	var d = 0.;
	var aD = 0.;

    let thD = 0.02;
	for(var i=0; i<256; i++) {
		if(layers > 15 || col.x > 1. || t>5.6) {
			break;
		}

        sp = ro + rd * t;
        d = map(sp);
		dstepf += 0.003;

        aD = (thD - abs(d)* 15./16.)/thD;

        if(aD>0.) {
			col += aD*aD*(3. - 2.*aD)/(1. + t*t*.25)*.2;
            layers++;
		}
        t += max( abs(d) * 0.7, thD * 1.5) * dstepf;
	}

    col = max(col, vec3<f32>(0.));
    col = mix(col, vec3<f32>(min(col.x*1.5, 1.), pow(col.x, 2.5), pow(col.x, 12.)),
              dot(sin(rd.yzx*8. + sin(rd.zxy*8.)), vec3<f32>(.1666))+0.4);
    col = mix(col, vec3<f32>(col.x*col.x*.85, col.x, col.x*col.x*0.3),
             dot(sin(rd.yzx*4. + sin(rd.zxy*4.)), vec3<f32>(.1666))+0.25);
	return clamp(col, vec3<f32>(0.), vec3<f32>(1.));
}

