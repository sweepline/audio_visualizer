# Notes

## TODO

- Sound wave texture
- Beat detection + texture
  - https://mziccard.me/2015/05/28/beats-detection-algorithms-1
- Make all 3 textures (freq, wave, beat) Width x 10 or more textures and include previous steps.
- Multi-level texture test
- Immediate mode UI
  - Debug UI
- Change parameters at runtime.
- Start/stop control.
- Selecting different shaders in UI at runtime
  - Seems pretty fast to just drop the old ones and recompile them, but be aware of it.
  - Gracefully handle compilation errors.
    - This also goes into the fading of shaders, because we can try and compile it and not fade then.
- Build the preface into the shader compilation so you only have to write the fs_user function. (Adding consts and functions outside should be available).
- Transition between shaders (have the main fragment control alpha and blend them).
- Maybe allow webasm code to control phases before shader.
- Some kind of prelude generation + docs for it.

## Seperate rendering

This will also need seperation of the FFT and the rendering, which we should have.
We could run the Smoothing at frame rate instead of fft rate?

https://gist.github.com/soulthreads/2efe50da4be1fb5f7ab60ff14ca434b8 IS VERY USEFUL

Maybe use a rendering library for an easier time? (probably not).

## Storage buffer

Maybe use storage buffers or dynamic uniforms for fft data, as the data should be pretty small.

FFT_SIZE x timesteps textures work pretty well..

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

