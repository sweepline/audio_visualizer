use core::f32::consts::PI;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SupportedBufferSize,
};
use ringbuf::{Consumer, StaticRb};
use rustfft::{num_complex::Complex32, FftPlanner};
use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

// mod camera;
mod egui_integration;
mod fft_buffer;
mod shaders;
mod state;
// mod texture;
mod ui;

// Should be 2^n.
pub const FFT_SIZE: usize = 1024;
// 1/4 size of FFT_SIZE for 0-10kHz assuming a 44.1kHz Source.
// Width must be less than or equal to FFT_SIZE
pub const TEXTURE_WIDTH: usize = FFT_SIZE / 4;
pub const TEXTURE_HEIGHT: usize = 2;
pub const TEXTURE_SIZE: usize = TEXTURE_WIDTH * TEXTURE_HEIGHT;
// Smoothing for the visuals should be applied per time and not based on FFT SIZE
pub const SMOOTHING: f32 = 0.8;
pub const RING_SIZE: usize = FFT_SIZE * 4;

pub type TextureHandle = Arc<Mutex<[f32; TEXTURE_SIZE]>>;

pub fn blackman_single(sample: f32, n: f32, len: f32) -> f32 {
    let a0 = (1. - 0.16) / 2.;
    let a1 = 0.5;
    let a2 = 0.16 / 2.;
    let w = a0 - a1 * f32::cos((2. * PI * n) / len) + a2 * f32::cos((4. * PI * n) / len);
    sample * w
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
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_transparent(true)
        .build(&event_loop)
        .unwrap();

    let host = cpal::default_host();
    let device = host.default_input_device().unwrap();

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
    let ring = StaticRb::<f32, RING_SIZE>::default();
    let (mut producer, mut consumer) = ring.split();

    // Fill the samples with 0. equal to the length of the delay.
    for _ in 0..RING_SIZE {
        producer.push(0.).unwrap();
    }

    fn err_fn(err: cpal::StreamError) {
        eprintln!("an error occurred on stream: {}", err);
    }
    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let mut output_fell_behind = false;
        for &sample in data {
            if producer.push(sample).is_err() {
                output_fell_behind = true;
            }
        }
        if output_fell_behind {
            eprintln!("Output stream fell behind: try increasing latency");
        }
    };
    // END FFT

    let mut state = state::State::new(&window).await;

    // Better performance with Arc<[Atomic]>....
    let tex_handle: TextureHandle = Arc::new(Mutex::new([0.; TEXTURE_SIZE]));

    let fft_tex_handle = tex_handle.clone();
    thread::spawn(move || {
        fft_analysis(&mut consumer, fft_tex_handle, sample_rate);
    });

    // START FFT
    let stream = device
        .build_input_stream(&config.into(), input_data_fn, err_fn, None)
        .unwrap();
    stream.play().unwrap();

    let mut timer = Instant::now();

    // END FFT.
    event_loop.run(move |event, _, control_flow| {
        state.ui.handle_event(&event);
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => {
                let fps = 1_000_000 / timer.elapsed().as_micros();
                let mul = 2; // Make fps a multiple of mul;
                let fps = ((fps + mul - 1) / mul) * mul;
                timer = Instant::now();

                state.update(tex_handle.clone());
                match state.render(&window) {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
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

fn fft_analysis(
    consumer: &mut Consumer<f32, Arc<StaticRb<f32, RING_SIZE>>>,
    texture_handle: Arc<Mutex<[f32]>>,
    sample_rate: f32,
) {
    let sr_ms = sample_rate / 1_000.;
    let sr_us = sr_ms / 1_000.;
    let fft_delay_us = (FFT_SIZE as f32 / sr_us).round() as u128;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let mut scratch = vec![Complex32::default(); fft.get_inplace_scratch_len()];

    let mut amplitudes: [f32; FFT_SIZE as usize / 2] = [0.; FFT_SIZE as usize / 2];
    let mut fft_buf: [Complex32; FFT_SIZE as usize] = [Complex32::default(); FFT_SIZE as usize];
    let mut timer = Instant::now();

    loop {
        // START FFT
        let elapsed = timer.elapsed().as_micros();
        if elapsed > fft_delay_us as u128 {
            // TODO: do something about time drift (fix your timestep).
            timer = Instant::now();
            // Time elapsed in microseconds * samples per microseconds
            let exact_samples = elapsed as f32 * sr_us;
            let time_drift = elapsed - fft_delay_us;

            let mut input_fell_behind = false;
            for i in 0..FFT_SIZE {
                let x = match consumer.pop() {
                    Some(s) => s,
                    None => {
                        input_fell_behind = true;
                        0.
                    }
                };
                // Apply windowing function to the input
                fft_buf[i] = Complex32::new(blackman_single(x, i as f32, FFT_SIZE as f32), 0.);
            }

            if input_fell_behind {
                eprintln!("Input stream fell behind: try increasing latency");
            }

            fft.process_with_scratch(&mut fft_buf, &mut scratch);

            let _bin_freq = sample_rate / FFT_SIZE as f32;

            let Ok(mut texture) = texture_handle.lock() else {
                panic!("TEXTURE MUTEX FFT SIDE");
            };

            // TODO: Maybe move this out of this loop and to the main thread?
            // The buffer has the last TEXTURE_HEIGHT fft runs.
            // With the first TEXTURE_WIDTH elements being the newest run.
            // So rotate the elements back one TEXTURE_WIDTH and write the
            // new run to the buffer at the front.
            texture.rotate_right(TEXTURE_WIDTH);
            texture[..TEXTURE_WIDTH].copy_from_slice(&[0.; TEXTURE_WIDTH]);

            let freq_amp = fft_buf.into_iter().take(TEXTURE_WIDTH).enumerate();
            for (i, amp) in freq_amp {
                let amp = amp / FFT_SIZE as f32;
                let amp_prev = amplitudes[i];
                let amp = SMOOTHING * amp_prev + (1. - SMOOTHING) * amp.norm();
                amplitudes[i] = amp;

                if i < texture.len() {
                    texture[i] += amp;
                }
            }
            const DB_LO: f32 = -100.;
            const DB_HI: f32 = -10.;
            for amp in texture.iter_mut() {
                let db = 20. * f32::log10(*amp);
                let db = db.clamp(DB_LO, DB_HI);
                let normalized = (db - DB_LO) / (DB_HI - DB_LO);
                *amp = normalized;
            }

            // Done with the texture so drop it so the rendering can use it.
            drop(texture);

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
            const EARLY_WAKE_US: u128 = 2000; // Because we want to stop sleeping a little before.
            let remaining = fft_delay_us - timer.elapsed().as_micros() - EARLY_WAKE_US;
            spin_sleep::sleep(Duration::from_micros(remaining as u64))
        }
    }
}
