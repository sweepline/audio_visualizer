use core::f32::consts::PI;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SupportedBufferSize,
};
use fft_buffer::FFTDimensions;
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
mod audio_processor;
mod egui_integration;
mod fft_buffer;
mod shaders;
mod state;
// mod texture;
mod ui;

pub type TextureHandle = Arc<Mutex<Vec<f32>>>;

#[tokio::main]
async fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_transparent(true)
        .build(&event_loop)
        .unwrap();

    let fft_dim = FFTDimensions::default();

    let audio_processor = audio_processor::AudioProcessor::new(fft_dim);
    let mut state = state::State::new(&window, fft_dim).await;

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

                state.update(audio_processor.fft_texture());
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
