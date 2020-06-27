

use crate::render::cache::AssetCache;
use crate::render::bytes;

use chashmap::CHashMap;


fn load_model_file(name: &'static str) -> anyhow::Result<(Vec<tobj::Model>, Vec<tobj::Material>)> {
    use std::path::PathBuf;

    let mut path = PathBuf::from("assets/models");
    path.push(name);

    let path = path.canonicalize()?;
    let data = tobj::load_obj(&path, false)?;

    Ok(data)
}


#[derive(Copy, Clone, Debug)]
pub struct ModelName {
    pub file: &'static str,
    pub name: &'static str,
}

pub struct ModelCache {
    device: &'static wgpu::Device,
    known_files: CHashMap<&'static str, ()>,
    obj_cache: CHashMap<String, ModelCacheEntry>,
}

pub type ModelRef<'a> = chashmap::ReadGuard<'a, String, ModelCacheEntry>;



impl ModelCache {
    
    pub fn new(device: &'static wgpu::Device) -> Self {
        let known_files = CHashMap::new();
        let obj_cache = CHashMap::new();
        Self {
            device,
            known_files,
            obj_cache,
        }
    }

    pub fn load(&self, ModelName { file, name }: ModelName) -> ModelRef {
        
        if let Some(_) = self.known_files.get(file) {
            return self.obj_cache.get(name).unwrap()
        }
        
        let file_data = load_model_file(file)
            .expect("Failed to load model file!");
        
        for model in file_data.0 {

            eprintln!("positions.len: {}\nindices.len: {}\ntexcoords.len: {}\nnormals.len: {}",
                model.mesh.positions.len(),
                model.mesh.indices.len(),
                model.mesh.texcoords.len(),
                model.mesh.normals.len());
            

            let mesh = model.mesh;
            
            let positions = self.device.create_buffer_with_data(
                bytes::of_slice(&mesh.positions),
                wgpu::BufferUsage::VERTEX,
            );

            let indices = self.device.create_buffer_with_data(
                bytes::of_slice(&mesh.indices),
                wgpu::BufferUsage::INDEX,
            );
            
            // TODO: handle material-mesh associations
            let material = None;

            let normals = if !mesh.normals.is_empty() {
                let buffer = self.device.create_buffer_with_data(
                    bytes::of_slice(&mesh.normals),
                    wgpu::BufferUsage::VERTEX,
                );
                Some(buffer)
            } else {
                None
            };

            let texcoords = if !mesh.texcoords.is_empty() {
                let buffer = self.device.create_buffer_with_data(
                    bytes::of_slice(&mesh.texcoords),
                    wgpu::BufferUsage::VERTEX,
                );
                Some(buffer)
            } else {
                None
            };

            let cache_entry = ModelCacheEntry {
                vertex_ct: mesh.indices.len() as u32,
                positions,
                indices,
                material,
                normals,
                texcoords,
            };

            
            eprintln!("loaded model name: {}, vertices: {}", &model.name, &cache_entry.vertex_ct);
            self.obj_cache.insert(model.name, cache_entry);
        }

        self.known_files.insert(file, ());
        self.obj_cache.get(name).unwrap()
    }

}


pub struct ModelCacheEntry {
    pub vertex_ct: u32,
    pub positions: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    
    pub material: Option<&'static str>,
    pub normals: Option<wgpu::Buffer>,
    pub texcoords: Option<wgpu::Buffer>,
}


impl<'a> AssetCache<ModelCacheEntry> for &'a ModelCache {
    type AssetName = ModelName;
    type AssetRef = ModelRef<'a>;

    fn load(self, name: Self::AssetName) -> Self::AssetRef {
        ModelCache::load(self, name)
    }
    
    fn invalidate(self, name: Self::AssetName) {
        self.known_files.remove(name.file);
    }

    fn clear(self) {
        self.known_files.clear();
        self.obj_cache.clear();
    }
}