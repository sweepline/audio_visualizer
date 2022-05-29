use anyhow::Error;
use core::f32::consts::PI;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::RingBuffer;
use rustfft::{algorithm::Radix4, num_complex::Complex32, Fft, FftDirection, FftPlanner};
use std::time::{Instant, SystemTime};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod camera;
mod fft_buffer;
mod state;
mod texture;

pub const BUCKETS: usize = 20;

pub fn hann_window(samples: &mut [f32]) -> Result<(), Error> {
    let samples_len_f32 = samples.len() as f32;
    for (i, sample) in samples.iter_mut().enumerate() {
        let two_pi_i = 2.0 * PI * i as f32;
        let idontknowthename = f32::cos(two_pi_i / samples_len_f32);
        let multiplier = 0.5 * (1.0 - idontknowthename);
        let windowed = multiplier * *sample;
        *sample = windowed;
    }
    Ok(())
}

pub fn hann_single(sample: f32, i: usize, samples_len: usize) -> f32 {
    let samples_len_f32 = samples_len as f32;
    let two_pi_i = 2.0 * PI * i as f32;
    let cosine = f32::cos(two_pi_i / samples_len_f32);
    let multiplier = 0.5 * (1.0 - cosine);
    multiplier * sample
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // FFT STUFF MOVE INTO STRUCT FOR STATE.
    // ALSO MAKE RENDER STATE AND PROGRAM STATE SEPERATE.
    let host = cpal::default_host();
    let device = host.default_input_device().unwrap();
    let mut config = device
        .default_input_config()
        .expect("Failed to get default input config")
        .config();
    // TODO: 2 Channels, the data will be interleaved [L, R, L, R]
    config.channels = 1;
    let channels = config.channels as usize;
    let sample_rate = config.sample_rate.0 as f32;

    println!("NAME: {:?}", device.name());
    println!("SIZE: {:?}", config.buffer_size);
    println!("RATE: {:?}", sample_rate);
    println!("CHANNELS: {:?}", config.channels);

    // Create a delay in case the input and output devices aren't synced.
    let latency = 80.0; // ms
    let latency_frames = (latency / 1_000.0) * sample_rate;
    let latency_samples = latency_frames as usize * channels;
    println!(
        "LATENCY: {:?}, LATENCY_FRAMES: {:?}, LATENCY_SAMPLES: {:?}",
        latency, latency_frames, latency_samples
    );

    // The buffer to share samples
    let ring = RingBuffer::<f32>::new(latency_samples * 2);
    let (mut producer, mut consumer) = ring.split();

    // Fill the samples with 0.0 equal to the length of the delay.
    for _ in 0..latency_samples {
        // The ring buffer has twice as much space as necessary to add latency here,
        // so this should never fail
        producer.push(0.0).unwrap();
    }

    let mut planner = FftPlanner::new();

    let mut now = Instant::now();
    fn err_fn(err: cpal::StreamError) {
        eprintln!("an error occurred on stream: {}", err);
    }
    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        // println!(
        //     "creating {:?} samples, for {:?} ms",
        //     data.len(),
        //     now.elapsed().as_millis()
        // );
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

    let mut now2 = Instant::now();
    // END FFT.
    event_loop.run(move |event, _, control_flow| {
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
            Event::RedrawRequested(_) => {
                // START FFT
                let elapsed = now2.elapsed().as_micros();
                const FFT_SAMPLING_RATE: u128 = 200 * 1000;

                let mut bucketed_opt: Option<Vec<f32>> = None;
                // Try to run the FFT for the frequency response every 30 ms.
                if elapsed > FFT_SAMPLING_RATE {
                    let frame_samples = elapsed as f32 * sample_rate / 1_000_000.0;
                    let fft_samples = frame_samples as usize * channels;
                    // println!(
                    //     "consuming {:?} samples, for {:?} ms",
                    //     fft_samples,
                    //     elapsed / 1_000
                    // );
                    now2 = Instant::now();

                    let mut input_fell_behind = false;
                    let mut fft_buf: Vec<Complex32> = Vec::new();
                    for i in 0..fft_samples {
                        let x = match consumer.pop() {
                            Some(s) => s,
                            None => {
                                input_fell_behind = true;
                                0.0
                            }
                        };
                        fft_buf.push(Complex32::new(hann_single(x, i, fft_samples), 0.0))
                    }

                    if input_fell_behind {
                        eprintln!("input stream fell behind: try increasing latency");
                    }
                    // TODO: Probably bad to run this before every process.
                    let fft = planner.plan_fft_forward(fft_samples);

                    fft.process(&mut fft_buf);

                    let freq_range = (15.0, 2000.0);
                    let mut bucketed: Vec<f32> = vec![0.0; BUCKETS];
                    println!(
                        "bin size: {:?}, buckets: {}",
                        sample_rate / fft_samples as f32,
                        bucketed.len()
                    );

                    let freq_amp: Vec<(f32, f32)> = fft_buf
                        .into_iter()
                        .take(fft_samples / 2)
                        .enumerate()
                        .map(|(i, c)| {
                            let bin_size = sample_rate / fft_samples as f32;
                            (i as f32 * bin_size, c.norm())
                        })
                        .filter(|(freq, _)| freq > &freq_range.0 && freq < &freq_range.1)
                        .collect();

                    let freq_range_log = (f32::log10(freq_range.0), f32::log10(freq_range.1));
                    let freq_to_bucket_log = |&freq| {
                        let scaled_01 = (f32::log10(freq) - freq_range_log.0)
                            / (freq_range_log.1 - freq_range_log.0);
                        let scaled_buckets = scaled_01 * BUCKETS as f32;
                        scaled_buckets as usize
                    };
                    let freq_to_bucket_lin = |&freq| {
                        let scaled_01 = (freq - freq_range.0) / (freq_range.1 - freq_range.0);
                        let scaled_buckets = scaled_01 * BUCKETS as f32;
                        scaled_buckets as usize
                    };
                    for (freq, amp) in &freq_amp {
                        // let bucket_index = freq_to_bucket_log(freq);
                        let bucket_index = freq_to_bucket_lin(freq);
                        bucketed[bucket_index] += amp;
                    }

                    // println!("{:#?}", bucketed);
                    let max_peak = freq_amp.iter().max_by_key(|&(_, c)| *c as u32);
                    let min_freq = freq_amp.iter().min_by_key(|&(f, _)| *f as u32).unwrap();
                    let max_freq = freq_amp.iter().max_by_key(|&(f, _)| *f as u32).unwrap();
                    println!("Min freq {}, max freq {}", min_freq.0, max_freq.0);
                    if let Some((freq, amp)) = max_peak {
                        println!("Max peak was {}, with amplitude {}", freq, amp);
                    }
                    bucketed_opt = Some(bucketed);
                }
                // END FFT

                state.update(bucketed_opt.as_deref());
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
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
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
