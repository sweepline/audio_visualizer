use std::num::NonZeroU32;

use std::{iter, path};

use wgpu::util::DeviceExt;

use crate::audio_processor::AudioProcessor;
use crate::fft_buffer;
use crate::shaders::{self, INDICES, VERTICES};
use crate::state::State;
use crate::ui::Ui;

#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UtilUniform {
    pub time: f32,
    pub res_width: f32,
    pub res_height: f32,
}

pub struct Renderer {
    surface: wgpu::Surface,
    pub device: wgpu::Device,
    queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,

    pub size: winit::dpi::PhysicalSize<u32>,
    util_buffer: wgpu::Buffer,
    util_bind_group: wgpu::BindGroup,

    render_pipeline_layout: wgpu::PipelineLayout,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    fft_buffer: fft_buffer::FFTBuffer,
    fft_bind_group: wgpu::BindGroup,
}

impl Renderer {
    pub async fn new(state: &State) -> Self {
        let State {
            window,
            fft_dimensions,
            ..
        } = state;
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });
        let surface = unsafe { instance.create_surface(window) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        // Pick a non SRGB surface such that we don't have to convert in-shader.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| !f.describe().srgb)
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let fft_buffer =
            fft_buffer::FFTBuffer::from_buffer(&device, &queue, "fft_buffer", fft_dimensions)
                .unwrap();

        let fft_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
                label: Some("fft_bind_group_layout"),
            });

        let fft_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &fft_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fft_buffer.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&fft_buffer.sampler),
                },
            ],
            label: Some("fft_bind_group"),
        });

        // Init Utils

        let util_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Utils Buffer"),
            contents: bytemuck::cast_slice(&[UtilUniform {
                time: 0.0,
                res_width: size.width as f32,
                res_height: size.height as f32,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let util_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("util_bind_group_layout"),
            });

        let util_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &util_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: util_buffer.as_entire_binding(),
            }],
            label: Some("util_bind_group"),
        });

        let shaders =
            crate::shaders::list_shaders().expect("Some shaders available at initial load");
        let shader_src = &shaders[0];

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&util_bind_group_layout, &fft_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = shaders::make_pipeline(
            &device,
            &render_pipeline_layout,
            surface_format,
            &shader_src,
        );

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        Self {
            surface,
            device,
            queue,
            surface_config: config,
            size,
            render_pipeline_layout,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            fft_buffer,
            fft_bind_group,
            util_buffer,
            util_bind_group,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub fn change_shader(&mut self, shader: &path::PathBuf) {
        self.render_pipeline = crate::shaders::make_pipeline(
            &self.device,
            &self.render_pipeline_layout,
            self.surface_config.format,
            shader,
        );
    }

    pub fn update(&mut self, ap: &AudioProcessor, state: &mut State) {
        let util_uniform = [UtilUniform {
            time: state.get_elapsed_time().as_secs_f32(),
            res_width: self.size.width as f32,
            res_height: self.size.height as f32,
        }];
        let data: &[u8] = bytemuck::cast_slice(&util_uniform);
        self.queue.write_buffer(&self.util_buffer, 0, data);

        let fft = &mut self.fft_buffer;
        // We might not have gotten the lock, so just leave the data the same.
        let fft_texture = ap.fft_texture();
        if let Ok(fft_texture) = fft_texture.try_lock() {
            fft.buffer.copy_from_slice(fft_texture.as_slice());
        }
        drop(fft_texture);

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &fft.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &fft_buffer::to_byte_slice(&fft.buffer),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(4 * fft.size.width),
                rows_per_image: NonZeroU32::new(fft.size.height),
            },
            fft.size,
        );
    }

    pub fn render(&mut self, state: &State, ui: &mut Ui) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.util_bind_group, &[]);
            render_pass.set_bind_group(1, &self.fft_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        let _ok = ui.render(
            &mut encoder,
            &view,
            &state.window,
            &self.device,
            &self.queue,
            &self.surface_config,
        );

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
