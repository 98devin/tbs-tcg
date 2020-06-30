
use crate::render::{
    self,
    bytes,
    cache::AssetCache,
};

use chashmap::CHashMap;


fn load_texture_file(name: &str) -> anyhow::Result<image::DynamicImage> {
    use std::path::PathBuf;

    let mut path = PathBuf::from("assets/textures");
    path.push(name);

    let path = path.canonicalize()?;
    let img = image::open(&path)?;

    Ok(img)
}


enum Raw {
    VecU16(Vec<u16>),
    VecU8(Vec<u8>),
}


pub struct Texture {
    handle: wgpu::Texture,
    desc: wgpu::TextureDescriptor<'static>,
    view: wgpu::TextureView,
}



pub struct TextureCache {
    device: &'static wgpu::Device,
    queue:  &'static wgpu::Queue,
    cache: CHashMap<&'static str, TextureCacheEntry>,
}

pub struct TextureCacheEntry {
    pub texture: wgpu::Texture,
    pub sampler: wgpu::Sampler,
    pub format: wgpu::TextureFormat,

    pub bind_layout: wgpu::BindGroupLayout,
}


pub type TextureRef<'a> = chashmap::ReadGuard<'a, &'static str, TextureCacheEntry>;


impl TextureCache {

    pub fn new(device: &'static wgpu::Device, queue: &'static wgpu::Queue) -> Self {
        let cache = CHashMap::new();
        Self {
            device,
            queue,
            cache,
        }
    }

    pub fn load(&self, name: &'static str) -> TextureRef {

        if let Some(texture) = self.cache.get(name) {
            return texture;
        }
        
        let img = load_texture_file(name).expect("Failed to load texture!");
        
        use image::{ColorType, DynamicImage::*, GenericImageView as _};

        let (width, height) = img.dimensions();
        
    
        let (raw, format, bpp) = match img.color() {
            ColorType::Rgb8 | ColorType::Rgba8 =>
                ( Raw::VecU8(img.into_rgba().into_raw())
                , wgpu::TextureFormat::Rgba8Unorm
                , 4),
    
            ColorType::Bgr8 | ColorType::Bgra8 =>
                ( Raw::VecU8(img.into_bgra().into_raw())
                , wgpu::TextureFormat::Bgra8Unorm
                , 4),
            
            ColorType::Rgba16 =>
                ( Raw::VecU16(match img { ImageRgba16(img) => img.into_raw(), _ => panic!("Image lied about its formatting!") })
                , wgpu::TextureFormat::Rgba16Float
                , 8),
            
            other =>
                panic!("Unsupported image color format: {:?}", other),
        };
    
        let raw_bytes = match &raw {
            Raw::VecU8(v) => &v,
            Raw::VecU16(v) => bytes::of_slice(&v),
        };
    
        let texture_desc = wgpu::TextureDescriptor {
            label: Some(name),
            size: wgpu::Extent3d { width, height, depth: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
            format,
        };
    
        let texture = self.device.create_texture(&texture_desc);
        self.queue.write_texture(
            wgpu::TextureCopyView {
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                texture: &texture,
            }, 
            raw_bytes,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: bpp*width,
                rows_per_image: height,
            },
            texture_desc.size,
        );
    
        let sample_desc = wgpu::SamplerDescriptor {
            label: Some(name),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: None,
            ..Default::default()
        };
    
        let sampler = self.device.create_sampler(&sample_desc);
    
        let bind_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(name),
            bindings: &[
                wgpu::BindGroupLayoutEntry::new(
                    0, wgpu::ShaderStage::FRAGMENT, 
                    wgpu::BindingType::SampledTexture {
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                        multisampled: false,
                    },
                ),
                wgpu::BindGroupLayoutEntry::new(
                    1, wgpu::ShaderStage::FRAGMENT,
                    wgpu::BindingType::Sampler {
                        comparison: false,
                    },
                ),
            ],
        });
    
        let entry = TextureCacheEntry {
            texture,
            sampler,
            format,
            bind_layout,
        };

        self.cache.insert(name, entry);
        self.cache.get(name).unwrap()
    }
}

impl<'a> AssetCache<TextureCacheEntry> for &'a TextureCache {
    type AssetName = &'static str;
    type AssetRef = TextureRef<'a>;
    
    fn load(self, name: &'static str) -> Self::AssetRef {
        TextureCache::load(self, name)
    }
    
    fn invalidate(self, name: &'static str) {
        self.cache.remove(name);
    }

    fn clear(self) {
        self.cache.clear();
    }
}

