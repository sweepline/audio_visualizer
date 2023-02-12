use std::{num::NonZeroU32, time::Instant};

use std::iter;

use wgpu::util::DeviceExt;
use winit::{event::*, window::Window};

use crate::{fft_buffer, ui::UiState, TextureHandle};

#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UtilUniform {
    pub time: f32,
    pub res_width: f32,
    pub res_height: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: glam::Vec3,
    tex_coords: glam::Vec2,
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Texture coordinates
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub const VERTICES: &[Vertex] = &[
    Vertex {
        position: glam::vec3(-1.0, 1.0, 0.0),
        tex_coords: glam::vec2(0.0, 0.0),
    }, // Top Left
    Vertex {
        position: glam::vec3(1.0, 1.0, 0.0),
        tex_coords: glam::vec2(1.0, 0.0),
    }, // Top Right
    Vertex {
        position: glam::vec3(-1.0, -1.0, 0.0),
        tex_coords: glam::vec2(0.0, 1.0),
    }, // Bot Left
    Vertex {
        position: glam::vec3(1.0, -1.0, 0.0),
        tex_coords: glam::vec2(1.0, 1.0),
    }, // Bot Right
];

pub const INDICES: &[u16] = &[0, 2, 1, 1, 2, 3];

pub struct State {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    pub size: winit::dpi::PhysicalSize<u32>,
    pub time: Instant,
    util_buffer: wgpu::Buffer,
    util_bind_group: wgpu::BindGroup,

    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    fft_buffer: fft_buffer::FFTBuffer,
    fft_bind_group: wgpu::BindGroup,

    pub ui: UiState,
}

impl State {
    pub async fn new(window: &Window) -> Self {
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
        println!("SURFACE TextureFormat: {:?}", surface_format);

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

        let fft_buffer = fft_buffer::FFTBuffer::from_buffer(&device, &queue, "fft_buffer").unwrap();

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

        let time = Instant::now();

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

        let shader = device.create_shader_module(wgpu::include_wgsl!("led_shader.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&util_bind_group_layout, &fft_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLAMPING
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

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

        let ui = UiState::new(&window, &device, surface_format);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            fft_buffer,
            fft_bind_group,
            time,
            util_buffer,
            util_bind_group,
            ui,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.ui.input(event)
    }

    fn get_elapsed_time(&self) -> f32 {
        self.time.elapsed().as_secs_f32()
    }

    pub fn update(&mut self, fft_texture: TextureHandle) {
        let x = [UtilUniform {
            time: self.get_elapsed_time(),
            res_width: self.size.width as f32,
            res_height: self.size.height as f32,
        }];
        let data: &[u8] = bytemuck::cast_slice(&x);
        self.queue.write_buffer(&self.util_buffer, 0, data);

        let fft = &mut self.fft_buffer;
        // We might not have gotten the lock, so just leave the data the same.
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

        self.ui.update(&self.time);
    }

    pub fn render(&mut self, window: &Window) -> Result<(), wgpu::SurfaceError> {
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

        let _ok = self.ui.render(
            &mut encoder,
            &view,
            window,
            &self.device,
            &self.queue,
            &self.config,
        );

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
