use std::num::NonZeroU32;

use anyhow::*;

use crate::{TEXTURE_HEIGHT, TEXTURE_SIZE, TEXTURE_WIDTH};

pub fn to_byte_slice<'a>(floats: &'a [f32]) -> &'a [u8] {
    unsafe { std::slice::from_raw_parts(floats.as_ptr() as *const _, floats.len() * 4) }
}

pub struct FFTBuffer {
    pub buffer: [f32; TEXTURE_SIZE],
    pub size: wgpu::Extent3d,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl FFTBuffer {
    pub fn from_buffer(device: &wgpu::Device, queue: &wgpu::Queue, label: &str) -> Result<Self> {
        let size = wgpu::Extent3d {
            width: TEXTURE_WIDTH as u32,
            height: TEXTURE_HEIGHT as u32,
            depth_or_array_layers: 1,
        };

        let buf: [f32; TEXTURE_SIZE] = [0.; TEXTURE_SIZE];
        let buf_data = to_byte_slice(&buf);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &buf_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(4 * TEXTURE_WIDTH as u32),
                rows_per_image: NonZeroU32::new(TEXTURE_HEIGHT as u32),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            buffer: buf,
            size,
            texture,
            view,
            sampler,
        })
    }
}
