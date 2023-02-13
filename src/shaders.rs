use std::{fs, io, path::PathBuf};

pub fn list_shaders() -> Result<Vec<PathBuf>, io::Error> {
    // let desc = wgpu::ShaderModuleDescriptor {
    //     label: Some("led_shader.wgsl"),
    //     source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/led_shader.wgsl").into()),
    // };
    let paths = fs::read_dir("./shaders")?;

    let mut files: Vec<PathBuf> = vec![];
    for path in paths {
        let p = path?.path();
        files.push(p);
    }
    Ok(files)
}

pub fn make_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    shader: &PathBuf,
) -> wgpu::RenderPipeline {
    let shader_src = fs::read_to_string(shader).expect("Should have been able to read the file");
    let desc = wgpu::ShaderModuleDescriptor {
        label: Some(
            shader
                .file_name()
                .expect("Shader file to have a filename")
                .to_str()
                .expect("Filename to be valid utf-8"),
        ),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    };
    let shader = device.create_shader_module(desc);

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[Vertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
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

    render_pipeline
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: glam::Vec3,
    tex_coords: glam::Vec2,
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
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
