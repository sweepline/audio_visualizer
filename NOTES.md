# Notes

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
