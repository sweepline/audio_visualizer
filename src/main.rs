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

pub fn hann_window(samples: &[f32]) -> Vec<f32> {
    let mut windowed_samples = Vec::with_capacity(samples.len());
    let samples_len_f32 = samples.len() as f32;
    for (i, sample) in samples.iter().enumerate() {
        let two_pi_i = 2.0 * PI * i as f32;
        let idontknowthename = f32::cos(two_pi_i / samples_len_f32);
        let multiplier = 0.5 * (1.0 - idontknowthename);
        windowed_samples.push(multiplier * sample)
    }
    windowed_samples
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

    // let fft: Radix4<f32> = Radix4::new(4096, FftDirection::Forward);
    let mut planner = FftPlanner::new();

    let mut now = Instant::now();
    fn err_fn(err: cpal::StreamError) {
        eprintln!("an error occurred on stream: {}", err);
    }
    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        println!("{:?}", producer.remaining());
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
                const FFT_SAMPLING_RATE: u128 = 60 * 1000; // 100 ms as µs

                // Try to run the FFT for the frequency response every 30 ms.
                if elapsed > FFT_SAMPLING_RATE {
                    let frame_samples = elapsed as f32 * sample_rate / 1_000_000.0;
                    let fft_samples = frame_samples as usize * channels;
                    println!(
                        "consuming {:?} samples, for {:?} ms",
                        fft_samples,
                        elapsed / 1_000
                    );
                    now2 = Instant::now();

                    let mut input_fell_behind = false;
                    let mut fft_buf: Vec<Complex32> = Vec::new();
                    for _ in 0..fft_samples {
                        let x = match consumer.pop() {
                            Some(s) => s,
                            None => {
                                input_fell_behind = true;
                                0.0
                            }
                        };
                        fft_buf.push(Complex32::new(x, 0.0))
                    }
                    if input_fell_behind {
                        eprintln!("input stream fell behind: try increasing latency");
                    }
                    // TODO: Probably bad to run this before every process.
                    let fft = planner.plan_fft_forward(fft_samples);

                    fft.process(&mut fft_buf);
                }
                // END FFT

                state.update();
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
            _ => {}
        }
    });
}
