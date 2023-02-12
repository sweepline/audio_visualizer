use std::time::Instant;

use egui::FontDefinitions;
use egui_demo_lib::DemoWindows;

use wgpu::{CommandEncoder, Device, TextureFormat, TextureView};
use winit::{event::*, window::Window};

use crate::egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use crate::egui_winit_platform::{Platform, PlatformDescriptor};

pub struct UiState {
    platform: Platform,
    egui_rp: RenderPass,
    windows: DemoWindows,
}

impl UiState {
    pub fn new(window: &Window, device: &Device, surface_format: TextureFormat) -> Self {
        let size = window.inner_size();
        // We use the egui_winit_platform crate as the platform.
        let mut font_definitions = FontDefinitions::default();
        font_definitions
            .font_data
            .values_mut()
            .for_each(|x| x.tweak.scale = 1.0);
        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor: window.scale_factor(),
            font_definitions,
            style: Default::default(),
        });
        println!("SCALE: {:?}", window.scale_factor());

        // We use the egui_wgpu_backend crate as the render backend.
        let render_pass = RenderPass::new(&device, surface_format, 1);

        // Display the demo application that ships with egui.
        let demo_app = egui_demo_lib::DemoWindows::default();

        Self {
            platform,
            egui_rp: render_pass,
            windows: demo_app,
        }
    }

    pub fn update(&mut self, time: &Instant) {
        self.platform.update_time(time.elapsed().as_secs_f64());
    }

    pub fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) -> Result<(), wgpu::SurfaceError> {
        // Begin to draw the UI frame.
        self.platform.begin_frame();

        // Draw the demo application.
        self.windows.ui(&self.platform.context());

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let full_output = self.platform.end_frame(Some(&window));
        let paint_jobs = self.platform.context().tessellate(full_output.shapes);

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            physical_width: config.width,
            physical_height: config.height,
            scale_factor: window.scale_factor() as f32,
        };
        let tdelta: egui::TexturesDelta = full_output.textures_delta;
        self.egui_rp
            .add_textures(&device, &queue, &tdelta)
            .expect("add texture ok");
        self.egui_rp
            .update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

        // Record all render passes.
        self.egui_rp
            .execute(encoder, view, &paint_jobs, &screen_descriptor, None)
            .unwrap();

        self.egui_rp
            .remove_textures(tdelta)
            .expect("remove texture ok");
        Ok(())
    }

    pub fn handle_event(&mut self, event: &Event<()>) {
        self.platform.handle_event(event)
    }
}
