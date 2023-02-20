// mi-ku/Altair
// https://www.shadertoy.com/view/Xsj3zy

const MULT = 10.0;
const BLUR_EPS = 0.001;

fn colorize(uv: vec2<f32>) -> vec3<f32>
{
	let FNS = time_steps();
	let FNSF = f32(FNS);
	let FNSFinv = (1.0/FNSF);
	for(var i = 0; i < FNS; i++) {
		let pt = uv;
		//let val = get_val(uv * vec2<f32>( FNSFinv, 0.0 ) + vec2<f32>( f32(i)/FNSF, 0.0 ), uv )
		let val = fft_sample(uv.x, i)
			* FNSFinv * MULT  + ( f32(i) + 0.2 )/FNSF;

		if ( val > pt.y ) {
			let colv = f32(i) / FNSF;
			var col = vec3<f32>( colv, 0.26, 0.4 );
			col += min( .14, max( .4 - abs( val - pt.y ) * 80.0, 0.0 ) );
			return col;
		}
	}
	return vec3<f32>( 1.0, 0.26, 0.4 );
}

fn fs_user(uvv: vec2<f32>) -> vec3<f32> {
    let aspect = util.res_width / util.res_height;
	let uv = uvv;// / vec2<f32>(util.res_width, util.res_height);

	let c1 = colorize(uv);
	return c1;
}
