use anyhow::Error;
use core::f32::consts::PI;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SupportedBufferSize,
};
use ringbuf::RingBuffer;
use rustfft::{num_complex::Complex32, FftPlanner};
use std::time::Instant;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod camera;
mod fft_buffer;
mod state;
mod texture;

pub const FREQ_RANGE: (f32, f32) = (10., 10000.);
pub const TEXTURE_WIDTH: u32 = 512;
pub const FFT_SIZE: usize = 1024;
pub const SMOOTHING: f32 = 0.7;

pub fn hann_window(samples: &mut [f32]) -> Result<(), Error> {
    let samples_len_f32 = samples.len() as f32;
    for (i, sample) in samples.iter_mut().enumerate() {
        let two_pi_i = 2. * PI * i as f32;
        let idontknowthename = f32::cos(two_pi_i / samples_len_f32);
        let multiplier = 0.5 * (1. - idontknowthename);
        let windowed = multiplier * *sample;
        *sample = windowed;
    }
    Ok(())
}

pub fn hann_single(sample: f32, i: usize, samples_len: usize) -> f32 {
    let samples_len_f32 = samples_len as f32;
    let two_pi_i = 2. * PI * i as f32;
    let cosine = f32::cos(two_pi_i / samples_len_f32);
    let multiplier = 0.5 * (1. - cosine);
    multiplier * sample
}

#[tokio::main]
async fn main() {
    env_logger::init();
    println!("WOW");
    let event_loop = EventLoop::new();
    println!("WOW");
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // FFT STUFF MOVE INTO STRUCT FOR STATE.
    // ALSO MAKE RENDER STATE AND PROGRAM STATE SEPERATE.
    let host = cpal::default_host();
    let device = host.default_input_device().unwrap();
    println!("WOW");

    let mut config = device
        .default_input_config()
        .expect("Failed to get default input config")
        .config();
    let sample_rate = config.sample_rate.0 as f32;

    // let measure_duration = BLOCK_LENGTH as f32 * bandwidth;
    // let freq_resolution = sample_rate / BLOCK_LENGTH as f32;
    let supported = device.default_input_config().unwrap();

    // TODO: Somethingsfucky about the buffersize
    let _data_size = supported.sample_format().sample_size();

    let sr_ms = sample_rate / 1_000.;
    let sr_us = sr_ms / 1_000.;
    let fft_delay_us = (FFT_SIZE as f32 / sr_us).round() as u128;
    println!(
        "FFT_SIZE: {:?}, FFT_DELAY_MS: {:?}",
        FFT_SIZE,
        fft_delay_us / 1000
    );

    let bz = if let SupportedBufferSize::Range { .. } = supported.buffer_size() {
        BufferSize::Fixed(FFT_SIZE as u32)
    } else {
        BufferSize::Default
    };
    // TODO: 2 Channels, the data will be interleaved [L, R, L, R]
    config.channels = 1;
    config.buffer_size = bz;
    // config.buffer_size = BufferSize::Fixed(FrameCount)
    let channels = config.channels as usize;

    println!("NAME: {:?}", device.name());
    println!("SIZE: {:?}", config.buffer_size);
    println!("RATE: {:?}", sample_rate);
    println!(
        "SAMPLE_FORMAT: {:?}",
        supported.sample_format().sample_size()
    );
    println!("CHANNELS: {:?}", channels);

    // The buffer to share samples make it twice the needed length
    let ring_size = FFT_SIZE * 4;
    let ring = RingBuffer::<f32>::new(ring_size);
    let (mut producer, mut consumer) = ring.split();

    // Fill the samples with 0. equal to the length of the delay.
    for _ in 0..ring_size {
        producer.push(0.).unwrap();
    }

    let mut planner = FftPlanner::new();

    let mut now = Instant::now();
    fn err_fn(err: cpal::StreamError) {
        eprintln!("an error occurred on stream: {}", err);
    }
    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        println!(
            "creating {:?} samples, for {:?} ms",
            data.len(),
            now.elapsed().as_millis()
        );
        now = Instant::now();
        let mut output_fell_behind = false;
        for &sample in data {
            if producer.push(sample).is_err() {
                output_fell_behind = true;
            }
        }
        if output_fell_behind {
            eprintln!("output stream fell behind: try increasing latency");
        }
    };
    // END FFT

    let mut state = state::State::new(&window).await;

    // START FFT
    let stream = device
        .build_input_stream(&config.into(), input_data_fn, err_fn)
        .unwrap();
    stream.play().unwrap();

    let mut texture: [f32; TEXTURE_WIDTH as usize] = [0.; TEXTURE_WIDTH as usize];
    let mut amplitudes: [f32; FFT_SIZE as usize / 2] = [0.; FFT_SIZE as usize / 2];
    let mut fft_buf: [Complex32; FFT_SIZE as usize] = [Complex32::default(); FFT_SIZE as usize];
    // TODO: Probably bad to run this before every process.
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let mut now2 = Instant::now();
    // END FFT.
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &mut so w have to dereference it twice
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::MainEventsCleared => {
                // START FFT
                let elapsed = now2.elapsed().as_micros();
                if elapsed > fft_delay_us as u128 {
                    // TODO: do something about time drift (fix your timestep).
                    now2 = Instant::now();
                    // Time elapsed in microseconds * samples per microseconds
                    let exact_samples = elapsed as f32 * sr_us;
                    println!(
                        "time-drift: {:?} ms",
                        (elapsed - fft_delay_us) as f32 / 1000.
                    );
                    println!(
                        "consuming {:?} samples, for {:?} ms",
                        exact_samples,
                        elapsed / 1_000
                    );
                    // println!("{:?}", texture);

                    let mut input_fell_behind = false;
                    for i in 0..FFT_SIZE {
                        let x = match consumer.pop() {
                            Some(s) => s,
                            None => {
                                input_fell_behind = true;
                                0.
                            }
                        };
                        fft_buf[i] = Complex32::new(hann_single(x, i, FFT_SIZE), 0.);
                    }

                    if input_fell_behind {
                        eprintln!("input stream fell behind: try increasing latency");
                    }

                    // TODO: optimize this call to with_scratch
                    fft.process(&mut fft_buf);

                    // println!(
                    //     "bin size: {:?}, buckets: {}",
                    //     sample_rate / fft_samples as f32,
                    //     bucketed.len()
                    // );

                    // Something about getting half the samples as you put in.

                    let bin_freq = sample_rate / FFT_SIZE as f32;

                    texture.fill(0.); // CLEAR THE TEXTURE
                    let freq_amp = fft_buf.into_iter().take(FFT_SIZE / 2).enumerate();
                    for (i, amp) in freq_amp {
                        let amp = amp / FFT_SIZE as f32;
                        let amp_prev = amplitudes[i];
                        let amp = SMOOTHING * amp_prev + (1. - SMOOTHING) * amp.norm();
                        amplitudes[i] = amp;

                        let freq = i as f32 * bin_freq;
                        // if freq < FREQ_RANGE.0 || freq > FREQ_RANGE.1 {
                        //     continue;
                        // }

                        // Map one range to another
                        // int input_range = input_end - input_start;
                        // int output_range = output_end - output_start;
                        // output = (input - input_start)*output_range / input_range + output_start;
                        let input_range = (FREQ_RANGE.1 - FREQ_RANGE.0) as usize;
                        let bin_n = (freq - FREQ_RANGE.0) as usize * TEXTURE_WIDTH as usize
                            / input_range
                            + 0;

                        if i < texture.len() {
                            texture[i] += amp;
                        }
                    }
                    for amp in texture.iter_mut() {
                        let db = 20. * f32::log10(*amp);
                        let db = db.clamp(-100., -20.);
                        let normalized = (db - -100.) / (-30. - -100.);
                        *amp = normalized;
                    }

                    // let max_peak = freq_amp.iter().max_by_key(|&(_, c)| *c as u32);
                    // let min_peak = freq_amp.iter().min_by_key(|&(_, c)| *c as u32);
                    // let min_freq = freq_amp.iter().min_by_key(|&(f, _)| *f as u32).unwrap();
                    // let max_freq = freq_amp.iter().max_by_key(|&(f, _)| *f as u32).unwrap();
                    // // println!("Min freq {}, max freq {}", min_freq.0, max_freq.0);
                    // if let Some((freq, amp)) = max_peak {
                    //     println!("Max peak was {}, with amplitude {}", freq, amp);
                    // }
                    // if let Some((freq, amp)) = min_peak {
                    //     println!("Min peak was {}, with amplitude {}", freq, amp);
                    // }
                }
                // END FFT

                state.update(&texture);
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::Suspended => {
                println!("SUSPENDED");
            }
            Event::Resumed => {
                println!("RESUMED");
            }
            _ => {}
        }
    });
}

#[allow(dead_code)]
fn enumerate_devices() -> Result<(), anyhow::Error> {
    println!("Supported hosts:\n  {:?}", cpal::ALL_HOSTS);
    let available_hosts = cpal::available_hosts();
    println!("Available hosts:\n  {:?}", available_hosts);

    for host_id in available_hosts {
        println!("{}", host_id.name());
        let host = cpal::host_from_id(host_id)?;

        let default_in = host.default_input_device().map(|e| e.name().unwrap());
        let default_out = host.default_output_device().map(|e| e.name().unwrap());
        println!("  Default Input Device:\n    {:?}", default_in);
        println!("  Default Output Device:\n    {:?}", default_out);

        let devices = host.devices()?;
        println!("  Devices: ");
        for (device_index, device) in devices.enumerate() {
            println!("  {}. \"{}\"", device_index + 1, device.name()?);

            // Input configs
            if let Ok(conf) = device.default_input_config() {
                println!("    Default input stream config:\n      {:?}", conf);
            }
            let input_configs = match device.supported_input_configs() {
                Ok(f) => f.collect(),
                Err(e) => {
                    println!("    Error getting supported input configs: {:?}", e);
                    Vec::new()
                }
            };
            if !input_configs.is_empty() {
                println!("    All supported input stream configs:");
                for (config_index, config) in input_configs.into_iter().enumerate() {
                    println!(
                        "      {}.{}. {:?}",
                        device_index + 1,
                        config_index + 1,
                        config
                    );
                }
            }

            // Output configs
            if let Ok(conf) = device.default_output_config() {
                println!("    Default output stream config:\n      {:?}", conf);
            }
            let output_configs = match device.supported_output_configs() {
                Ok(f) => f.collect(),
                Err(e) => {
                    println!("    Error getting supported output configs: {:?}", e);
                    Vec::new()
                }
            };
            if !output_configs.is_empty() {
                println!("    All supported output stream configs:");
                for (config_index, config) in output_configs.into_iter().enumerate() {
                    println!(
                        "      {}.{}. {:?}",
                        device_index + 1,
                        config_index + 1,
                        config
                    );
                }
            }
        }
    }

    Ok(())
}
