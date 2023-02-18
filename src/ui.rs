use std::path::PathBuf;

use wgpu::{CommandEncoder, TextureView};
use winit::{event::*, window::Window};

use crate::egui_integration::wgpu::{RenderPass, ScreenDescriptor};
use crate::egui_integration::winit::{Platform, PlatformDescriptor};
use crate::renderer::Renderer;
use crate::shaders;
use crate::state::State;

pub struct Ui {
    platform: Platform,
    egui_rp: RenderPass,
    visible: bool,
    pressed_last_frame: bool,
    shaders: Vec<PathBuf>,
}

impl Ui {
    pub fn new(state: &State, renderer: &Renderer) -> Self {
        let size = state.window.inner_size();

        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor: state.window.scale_factor(),
            font_definitions: egui::FontDefinitions::default(),
            style: Default::default(),
        });

        {
            // Change the fonts to be bigger;
            use egui::FontFamily::Proportional;
            use egui::FontId;
            use egui::TextStyle::*;

            let ctx = platform.context();
            let mut style = (*ctx.style()).clone();
            style.text_styles = [
                (Heading, FontId::new(30.0, Proportional)),
                (Body, FontId::new(18.0, Proportional)),
                (Monospace, FontId::new(14.0, Proportional)),
                (Button, FontId::new(14.0, Proportional)),
                (Small, FontId::new(10.0, Proportional)),
            ]
            .into();
            ctx.set_style(style);
        }

        // We use the egui_wgpu_backend crate as the render backend.
        let render_pass = RenderPass::new(&renderer.device, renderer.surface_config.format, 1);

        Self {
            platform,
            egui_rp: render_pass,
            visible: false,
            pressed_last_frame: false,
            shaders: shaders::list_shaders().unwrap_or(vec![]),
        }
    }

    pub fn input(&mut self, event: &WindowEvent, state: &mut State) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::F1 => {
                        if is_pressed && !self.pressed_last_frame {
                            self.visible = !self.visible;
                        }
                        self.pressed_last_frame = is_pressed;
                        true
                    }
                    VirtualKeyCode::F2 => {
                        if is_pressed && !self.pressed_last_frame {
                            self.visible = !self.visible;
                        }
                        self.pressed_last_frame = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update(&mut self, state: &State, renderer: &mut Renderer) {
        let time = state.get_elapsed_time();
        self.platform.update_time(time.as_secs_f64());

        if !self.visible {
            // Returning at this point pauses animations,
            // so if you want to have them continue in the background you have to
            // do something about letting the ui render but not take input.
            return;
        }

        // Begin to draw the UI frame.
        self.platform.begin_frame();

        // Draw the demo application.
        // self.windows.ui(&self.platform.context());
        let ctx = self.platform.context();

        // TODO: Move this into an app struct.
        let mut visuals = egui::Visuals::dark();
        let mut rgba = egui::Rgba::from(visuals.panel_fill);
        rgba[3] = 0.8;
        visuals.panel_fill = rgba.into();
        let style = egui::Style {
            visuals,
            ..Default::default()
        };
        egui::SidePanel::left("debug_panel")
            .default_width(300.0)
            .frame(egui::Frame::side_top_panel(&style))
            .show(&ctx, |ui| {
                ui.label("egui");
                ui.add_space(12.0);
                ui.separator();
                for p in &self.shaders {
                    if ui.link(p.file_name().unwrap().to_str().unwrap()).clicked() {
                        renderer.change_shader(p);
                    }
                }
                ui.separator();
                ui.label(format!("FPS: {}", state.delayed_fps));
            });
    }

    /// Rendering the UI, update MUST be called before this every frame.
    pub fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) -> Result<(), wgpu::SurfaceError> {
        if !self.visible {
            // Returning at this point pauses animations,
            // so if you want to have them continue in the background you have to
            // do something about letting the ui render but not take input.
            return Ok(());
        }

        // Update must have been called by this point as it starts the frame.

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
