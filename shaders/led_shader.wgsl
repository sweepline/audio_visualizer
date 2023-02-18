const BANDS = 50.0;
const SEGS = 50.0;

fn fs_user(uv: vec2<f32>) -> vec3<f32> {
   // quantize coordinates
	let p = vec2<f32>(
		floor(uv.x*BANDS)/BANDS,
		floor(uv.y*SEGS)/SEGS
	);

	// textureDimensions gives actual dimensions (512x10) for example
	// But sampling is done in [0;1].
	let dim = textureDimensions(fft_buffer);
    let fft_sample: f32 = fft_sample(p.x, 0);

    // led color
    let color = mix(vec3<f32>(0.0, 2.0, 0.0), vec3<f32>(2.0, 0.0, 0.0), sqrt(uv.y));

    // mask for bar graph
    let mask = select(1.0, 0.1, p.y > fft_sample);

    // led shape
    let d = fract((uv - p) *vec2<f32>(BANDS, SEGS)) - 0.5;
    let led = smoothstep(0.5, 0.35, abs(d.x)) * smoothstep(0.5, 0.35, abs(d.y));
    let ledColor = led * color * mask;

    return ledColor;
}
