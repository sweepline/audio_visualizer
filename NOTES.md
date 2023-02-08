# Notes

Need a better system for the buckets.
Log scale for amplitude and frequency is definitely right.
I think i need to compensate that amplitude for lower buckets are smaller as they have fewer samples per bucket.
Which will probaly also help with the holes.

Maybe let the values fade over time instead of replacing to accenuate small time notes more.
This will also need seperation of the FFT and the rendering, which we should have.

Maybe use a rendering library for an easier time? (probably not).

Maybe use storage buffers or dynamic uniforms for fft data, as the data should be pretty small.

```rust
FFTData {
	amp_lim: vec2<f32>,
	bands: usize,
	data: vec<f32>
}
```

something something...

Take some input and to FFT on it and send the data to the shader.

Have two shaders running and blend them when switching? Maybe something more elaborate with morphing them.

I should probably make the visualizations codable with webasm so you don't have to recompile the program?
