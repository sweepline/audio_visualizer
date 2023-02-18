use std::time::Duration;
use std::time::Instant;

use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use crate::fft_buffer::FFTDimensions;

pub struct State {
    pub window: Window,
    pub fft_dimensions: FFTDimensions,
    time: Instant,
    frame_timer: Instant,

    // FPS Meter
    fps_timer: Instant,
    pub delayed_fps: u64,
}

impl State {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let window = WindowBuilder::new()
            .with_transparent(false)
            .build(event_loop)
            .unwrap();
        let time = Instant::now();
        let fft_dimensions = FFTDimensions::default();
        let fps_timer = Instant::now();
        let frame_timer = Instant::now();

        Self {
            time,
            fft_dimensions,
            window,
            frame_timer,
            fps_timer,
            delayed_fps: 60,
        }
    }

    pub fn get_elapsed_time(&self) -> Duration {
        self.time.elapsed()
    }

    pub fn update(&mut self) {
        const FPS_UPDATE_RATE_US: u128 = 500 * 1_000; // MS * TO_uS
        if self.fps_timer.elapsed().as_micros() > FPS_UPDATE_RATE_US {
            self.fps_timer = Instant::now();
            self.delayed_fps = self.get_fps();
        }
        self.frame_timer = Instant::now();
    }

    pub fn get_fps(&self) -> u64 {
        let fps = 1_000_000 / self.frame_timer.elapsed().as_micros();
        // let mul = 2; // Make fps a multiple of mul;
        // let fps = ((fps + mul - 1) / mul) * mul;
        fps as u64
    }
}
