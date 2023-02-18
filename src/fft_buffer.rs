use std::num::NonZeroU32;

use anyhow::*;

pub fn to_byte_slice<'a>(floats: &'a [f32]) -> &'a [u8] {
    unsafe { std::slice::from_raw_parts(floats.as_ptr() as *const _, floats.len() * 4) }
}

#[derive(Clone, Copy, Debug)]
pub struct FFTDimensions {
    pub fft_size: usize,
    time_slices: usize,
    pub smoothing: f32,
    ring_factor: usize,
    // TODO: make dependent on the sample rate.
}

impl FFTDimensions {
    pub fn new(fft_size: usize, time_slices: usize, smoothing: f32, ring_factor: usize) -> Self {
        if f32::log2(fft_size as f32).fract() != 0.0 {
            eprintln!("FFT Size should be power of two, but it was {}", fft_size);
        }
        Self {
            fft_size,
            time_slices,
            smoothing,
            ring_factor,
        }
    }
    /// 1/4 size of FFT_SIZE for 0-10kHz assuming a 44.1kHz Source.
    /// Width must be less than or equal to FFT_SIZE
    pub fn texture_width(&self) -> u32 {
        (self.fft_size / 4) as u32
    }
    pub fn texture_height(&self) -> u32 {
        self.time_slices as u32
    }
    pub fn texture_size(&self) -> usize {
        (self.texture_width() * self.texture_height()) as usize
    }
    pub fn ring_size(&self) -> usize {
        self.fft_size * self.ring_factor
    }
}

impl Default for FFTDimensions {
    fn default() -> Self {
        Self::new(1024, 100, 0.8, 4)
    }
}

/// This is a texture that we can write the fft_data into and send to the GPU.

pub struct FFTBuffer {
    pub buffer: Vec<f32>,
    pub size: wgpu::Extent3d,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl FFTBuffer {
    pub fn from_buffer(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        fft_dimensions: &FFTDimensions,
    ) -> Result<Self> {
        let size = wgpu::Extent3d {
            width: fft_dimensions.texture_width(),
            height: fft_dimensions.texture_height(),
            depth_or_array_layers: 1,
        };

        let buf: Vec<f32> = vec![0.; fft_dimensions.texture_size()];
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
                bytes_per_row: NonZeroU32::new(4 * fft_dimensions.texture_width()),
                rows_per_image: NonZeroU32::new(fft_dimensions.texture_height()),
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
