use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
};

mod audio_processor;
mod egui_integration;
mod fft_buffer;
mod renderer;
mod shaders;
mod state;
mod ui;

pub type TextureHandle = Arc<Mutex<Vec<f32>>>;

#[tokio::main]
async fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();

    let mut state = state::State::new(&event_loop);
    let audio_processor = audio_processor::AudioProcessor::new(&state);
    let mut renderer = renderer::Renderer::new(&state).await;
    let mut ui = ui::Ui::new(&state, &renderer);

    // END FFT.
    event_loop.run(move |event, _, control_flow| {
        ui.handle_event(&event);
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => {
                ui.update(&state, &mut renderer);
                renderer.update(&audio_processor, &mut state);
                //audio_processor.update().... needs to update thread.
                state.update();

                match renderer.render(&state, &mut ui) {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window.id() => {
                // If input didnt capture the keybind, do this.
                if !ui.input(event, &mut state) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            renderer.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &mut so w have to dereference it twice
                            renderer.resize(**new_inner_size);
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
