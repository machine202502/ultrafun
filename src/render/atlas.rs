use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

pub const ATLAS_DIRECTORY: &str = "assets/textures";
pub const ATLAS_MANIFEST: &str = "atlas.ron";
pub const WHITE_TILE: &str = "white";
const DEFAULT_TILE_METERS: f32 = 2.0;

#[derive(Debug, Clone, Deserialize)]
pub struct TileEntry {
    pub name: String,
    pub col: u32,
    pub row: u32,
    #[serde(default = "default_tile_meters")]
    pub meters: f32,
}

fn default_tile_meters() -> f32 {
    DEFAULT_TILE_METERS
}

#[derive(Debug, Deserialize)]
pub struct AtlasManifest {
    pub image: String,
    pub tile_size: u32,
    pub tiles: Vec<TileEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tile {
    pub uv_origin: [f32; 2],
    pub span: f32,
    pub meters: f32,
}

pub struct Atlas {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    tiles: HashMap<String, Tile>,
    pub tile_span: f32,
}

impl Atlas {
    pub fn load_or_white(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        match Self::load(device, queue) {
            Ok(atlas) => atlas,
            Err(error) => {
                log::warn!("level textures unavailable, using flat colors: {error:#}");
                Self::white(device, queue)
            }
        }
    }

    pub fn load(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self> {
        Self::load_manifest(device, queue, ATLAS_MANIFEST)
    }

    pub fn load_manifest(device: &wgpu::Device, queue: &wgpu::Queue, name: &str) -> Result<Self> {
        let manifest_path = Path::new(ATLAS_DIRECTORY).join(name);
        let manifest_text = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("cannot read {}", manifest_path.display()))?;
        let manifest: AtlasManifest = ron::from_str(&manifest_text)?;
        let image_path = Path::new(ATLAS_DIRECTORY).join(&manifest.image);
        let bytes = std::fs::read(&image_path)
            .with_context(|| format!("cannot read {}", image_path.display()))?;
        let image = image::load_from_memory(&bytes)?.to_rgba8();
        let (width, height) = image.dimensions();
        let tiles = manifest_tiles(&manifest, width, height)?;
        let span = tiles[WHITE_TILE].span;
        let view = upload_texture(device, queue, &image);
        let sampler = nearest_sampler(device);
        Ok(Self {
            view,
            sampler,
            tiles,
            tile_span: span,
        })
    }

    pub fn white(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]));
        let mut tiles = HashMap::new();
        tiles.insert(
            WHITE_TILE.to_owned(),
            Tile {
                uv_origin: [0.0, 0.0],
                span: 1.0,
                meters: DEFAULT_TILE_METERS,
            },
        );
        Self {
            view: upload_texture(device, queue, &image),
            sampler: nearest_sampler(device),
            tiles,
            tile_span: 1.0,
        }
    }

    pub fn tile(&self, name: Option<&str>) -> Tile {
        name.and_then(|name| self.tiles.get(name))
            .copied()
            .unwrap_or(self.tiles[WHITE_TILE])
    }
}

fn manifest_tiles(
    manifest: &AtlasManifest,
    width: u32,
    height: u32,
) -> Result<HashMap<String, Tile>> {
    if manifest.tile_size == 0
        || width == 0
        || width != height
        || !width.is_multiple_of(manifest.tile_size)
    {
        bail!("atlas dimensions must contain a square grid of nonzero tiles");
    }
    let columns = width / manifest.tile_size;
    let texel = 1.0 / width as f32;
    let span = (manifest.tile_size - 1) as f32 * texel;
    let mut tiles = HashMap::new();
    for entry in &manifest.tiles {
        if entry.col >= columns || entry.row >= columns {
            bail!("tile \"{}\" is outside the atlas", entry.name);
        }
        if !entry.meters.is_finite() || entry.meters <= 0.0 || entry.name.is_empty() {
            bail!("tile \"{}\" has invalid scale or name", entry.name);
        }
        let tile = Tile {
            uv_origin: [
                (entry.col as f32 * manifest.tile_size as f32 + 0.5) * texel,
                (entry.row as f32 * manifest.tile_size as f32 + 0.5) * texel,
            ],
            span,
            meters: entry.meters,
        };
        if tiles.insert(entry.name.clone(), tile).is_some() {
            bail!("duplicate atlas material: {}", entry.name);
        }
    }
    if !tiles.contains_key(WHITE_TILE) {
        bail!("atlas manifest has no white fallback tile");
    }
    Ok(tiles)
}

impl Tile {
    pub fn rectangle(self) -> [f32; 4] {
        [self.uv_origin[0], self.uv_origin[1], self.span, self.span]
    }
}

fn upload_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image: &image::RgbaImage,
) -> wgpu::TextureView {
    let (width, height) = image.dimensions();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("texture atlas"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        image.as_raw(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn nearest_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("texture atlas"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(tile_size: u32) -> AtlasManifest {
        AtlasManifest {
            image: "image.png".into(),
            tile_size,
            tiles: vec![TileEntry {
                name: "white".into(),
                col: 0,
                row: 0,
                meters: 1.0,
            }],
        }
    }

    #[test]
    fn invalid_manifest_does_not_divide_by_zero_or_accept_overflowing_tiles() {
        assert!(manifest_tiles(&manifest(0), 512, 512).is_err());
        let mut input = manifest(64);
        input.tiles[0].col = u32::MAX;
        assert!(manifest_tiles(&input, 512, 512).is_err());
    }

    #[test]
    fn duplicate_names_and_nonfinite_scales_are_rejected() {
        let mut input = manifest(64);
        input.tiles.push(input.tiles[0].clone());
        assert!(manifest_tiles(&input, 512, 512).is_err());
        input.tiles.pop();
        input.tiles[0].meters = f32::NAN;
        assert!(manifest_tiles(&input, 512, 512).is_err());
    }
}
