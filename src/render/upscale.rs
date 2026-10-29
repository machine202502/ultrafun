pub const LOW_RES_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct Upscaler {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    size: (u32, u32),
    targets: Targets,
}

impl Upscaler {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("upscale.wgsl"));
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("upscale"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("upscale"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("upscale"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("upscale"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });
        let targets = create_targets(device, &bind_group_layout, &sampler, size);

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            size,
            targets,
        }
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn color_view(&self) -> &wgpu::TextureView {
        &self.targets.color_view
    }

    pub fn color_texture(&self) -> &wgpu::Texture {
        &self.targets.color_texture
    }

    pub fn depth_view(&self) -> &wgpu::TextureView {
        &self.targets.depth_view
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        if size == self.size {
            return;
        }
        self.size = size;
        self.targets = create_targets(device, &self.bind_group_layout, &self.sampler, size);
    }

    pub fn blit(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("upscale"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.targets.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

pub fn internal_size(window: (u32, u32), internal_height: Option<u32>) -> (u32, u32) {
    let (width, height) = (window.0.max(1), window.1.max(1));
    let Some(internal_height) = internal_height else {
        return (width, height);
    };
    let internal_height = internal_height.clamp(1, height);
    let internal_width = (width as f32 / height as f32 * internal_height as f32).round() as u32;
    (internal_width.max(1), internal_height)
}

struct Targets {
    color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

fn create_targets(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    (width, height): (u32, u32),
) -> Targets {
    let texture = |label, format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color_texture = texture(
        "low resolution color",
        LOW_RES_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
    );
    let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = texture(
        "low resolution depth",
        DEPTH_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    )
    .create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("upscale"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&color_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    Targets {
        color_texture,
        color_view,
        depth_view,
        bind_group,
    }
}

pub fn read_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
) -> anyhow::Result<image::RgbaImage> {
    let (width, height) = (texture.width(), texture.height());
    let unpadded_row = width * 4;
    let padded_row = unpadded_row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("screenshot"),
        size: u64::from(padded_row * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("screenshot"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    buffer.slice(..).map_async(wgpu::MapMode::Read, |result| {
        if let Err(error) = result {
            log::error!("cannot read screenshot: {error}");
        }
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let mapped = buffer
        .slice(..)
        .get_mapped_range()
        .map_err(|error| anyhow::anyhow!("cannot map screenshot buffer: {error:?}"))?;
    let pixels = mapped
        .chunks(padded_row as usize)
        .flat_map(|row| &row[..unpadded_row as usize])
        .copied()
        .collect();
    image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| anyhow::anyhow!("screenshot buffer has wrong size"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_size_follows_window_aspect() {
        assert_eq!(internal_size((1920, 1080), Some(270)), (480, 270));
        assert_eq!(internal_size((1280, 1024), Some(270)), (338, 270));
    }

    #[test]
    fn no_internal_height_means_native_size() {
        assert_eq!(internal_size((1280, 720), None), (1280, 720));
    }

    #[test]
    fn degenerate_sizes_stay_valid() {
        assert_eq!(internal_size((0, 0), Some(270)), (1, 1));
        assert_eq!(internal_size((640, 200), Some(270)), (640, 200));
    }
}
