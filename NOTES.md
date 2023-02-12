# Notes

## TODO

- Sound wave texture
- Beat detection + texture
- Make all 3 textures (freq, wave, beat) Width x 10 or more textures and include previous steps.
- Immediate mode UI
  - Debug UI
- Change parameters at runtime.
- Start/stop control.
- Selecting different shaders in UI at runtime
- Transition between shaders
- Maybe allow webasm code to control phases before shader.

## Seperate rendering

This will also need seperation of the FFT and the rendering, which we should have.
We could run the Smoothing at frame rate instead of fft rate?

https://gist.github.com/soulthreads/2efe50da4be1fb5f7ab60ff14ca434b8 IS VERY USEFUL

Maybe use a rendering library for an easier time? (probably not).

## Storage buffer

Maybe use storage buffers or dynamic uniforms for fft data, as the data should be pretty small.

```rust
FFTData {
	amp_lim: vec2<f32>,
	bands: usize,
	data: vec<f32>
}
```

something something...

## Blending

Have two shaders running and blend them when switching? Maybe something more elaborate with morphing them.

## Modular

I should probably make the visualizations codable with webasm so you don't have to recompile the program?

## Map one range to another

```
int input_range = input_end - input_start;
int output_range = output_end - output_start;
output = (input - input_start)*output_range / input_range + output_start;
```

## Shaders

https://www.shadertoy.com/view/ssycDd
https://www.shadertoy.com/view/ldtBRN
https://www.shadertoy.com/view/XlVGWh
https://www.shadertoy.com/view/3dSyRK
https://www.shadertoy.com/view/Xsj3zy
