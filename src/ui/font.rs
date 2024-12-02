use std::{fs, iter::repeat, path::Path};

use bytemuck::{Pod, Zeroable};
use easy_signed_distance_field as sdf;
use wgpu::util::DeviceExt as _;

use crate::{GpuBuffer, GpuBufferData};

pub struct Font {
    pub data: FontData,
    pub uniform: FontUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl Font {
    pub fn load(device: &wgpu::Device, path: &Path) -> Self {
        let atlas = generate_atlas(path);
        let data = FontData { atlas };

        let GpuBufferData {
            data: uniform,
            buffer,
            bind_group_layout,
            bind_group,
        } = Self::init_buffer(device, &data);

        Self {
            data,
            uniform,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }
}

impl GpuBuffer<FontUniform> for Font {
    type Init = FontData;

    fn init_buffer(device: &wgpu::Device, init: &Self::Init) -> GpuBufferData<FontUniform> {
        let data = FontUniform {
            size: glam::uvec2(init.atlas.width, init.atlas.height),
            ..Default::default()
        };

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Font Uniform Buffer"),
            contents: bytemuck::cast_slice(&[FontUniform::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Font Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Font Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        GpuBufferData {
            data,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    // TODO: Specialize GpuBuffer into mutable / immutable
    fn write_buffer(&self, queue: &wgpu::Queue) {
        unreachable!()
    }
}

pub struct FontData {
    atlas: sdf::SdfRaster,
}

#[derive(Clone, Copy, Default, Pod, Zeroable)]
#[repr(C, align(16))]
pub struct FontUniform {
    pub size: glam::UVec2,
    _padding: glam::Vec2,
}

fn generate_atlas(font: &Path) -> sdf::SdfRaster {
    let font_data = fs::read(font).expect("Font file should be readable");
    let font = sdf::Font::from_bytes(font_data.as_slice(), sdf::FontSettings::default())
        .expect("Font file should be parsable");

    let px = 64.;
    let (_, rasters) = (0..26)
        .map(|offset| {
            let c = (b'a' + offset) as char;
            generate_glyph(&font, px, c)
        })
        .collect::<(Vec<f32>, Vec<sdf::SdfRaster>)>();

    let rows = 1 << (usize::BITS - rasters.len().leading_zeros() - 2);
    let cols = rasters.len() / rows + (rasters.len() % rows > 0) as usize;

    build_atlas(rasters, rows, cols)
}

fn build_atlas(rasters: Vec<sdf::SdfRaster>, width: usize, height: usize) -> sdf::SdfRaster {
    assert!(width * height >= rasters.len());

    let (cell_width, cell_height) = (rasters[0].width as usize, rasters[0].height as usize);
    let cell_size = cell_width * cell_height;

    let mut buf = vec![0.; cell_size * (width * height) as usize];

    for i in 0..height.min(rasters.len() / width + (rasters.len() % width > 0) as usize) {
        let cell_row_length = if rasters.len().saturating_sub(i * width) < width {
            rasters.len() - i * width
        } else {
            width
        };

        for j in 0..cell_row_length {
            let id = i * width + j;
            let start = i * width * cell_size + j * cell_width;

            for row in 0..cell_height as usize {
                let buf_row_start = start + row * width * cell_width;
                let buf_row_end = buf_row_start + cell_width;

                let row_start = row * cell_width;
                let row_end = row_start + cell_width;

                buf[buf_row_start..buf_row_end]
                    .copy_from_slice(&rasters[id].buffer[row_start..row_end]);
            }
        }
    }

    sdf::SdfRaster {
        width: (width * cell_width) as u32,
        height: (height * cell_height) as u32,
        buffer: buf,
    }
}

fn generate_glyph(font: &sdf::Font, px: f32, c: char) -> (f32, sdf::SdfRaster) {
    let padding = 2;
    let spread = 6.;

    let (_, mut glyph) = font
        .sdf_generate(px, padding, spread, c)
        .expect("SDF should be generated");

    let mut adjusted_px = px;

    // Decrease size to fit the glyph within the target bounds
    while glyph.width as f32 > px || glyph.height as f32 > px {
        adjusted_px -= 1.;

        (_, glyph) = font
            .sdf_generate(adjusted_px, padding, spread, c)
            .expect("SDF should be generated");
    }

    let glyph = square_glyph(glyph, px as u32);

    (adjusted_px, glyph)
}

fn square_glyph(mut glyph: sdf::SdfRaster, target: u32) -> sdf::SdfRaster {
    assert!(glyph.width <= target);
    assert!(glyph.height <= target);

    let hpad = (target - glyph.width) as usize;
    let lpad = hpad / 2;
    let rpad = lpad + hpad % 2;

    let inner = glyph
        .buffer
        .chunks(glyph.width as usize)
        .flat_map(|chunk| {
            repeat(&0.)
                .take(lpad)
                .chain(chunk)
                .chain(repeat(&0.).take(rpad))
        })
        .collect::<Vec<_>>();

    let vpad = (target - glyph.height) as usize;
    let tpad = vpad / 2;
    let bpad = tpad + vpad % 2;

    let tlid = repeat(&0f32).take(tpad * target as usize);
    let blid = repeat(&0f32).take(bpad * target as usize);

    let buf = tlid
        .chain(inner.into_iter())
        .chain(blid)
        .map(|f| *f)
        .collect::<Vec<_>>();

    glyph.buffer = buf;
    glyph.width = target;
    glyph.height = target;

    glyph
}
