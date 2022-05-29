# Notes

I dont understand the holes in the logarithmic scale. They are in the linear one too, i think its the buckets being smaller than the FFT Bins?
Amplitude needs to be made linear?
Maybe let the values fade over time instead of replacing to accenuate small time notes more.

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
