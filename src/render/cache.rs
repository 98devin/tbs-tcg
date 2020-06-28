
pub mod shaders;
pub mod textures;
pub mod models;


pub trait AssetCache<Asset> {
    type AssetName: 'static;
    type AssetRef: std::ops::Deref<Target=Asset>;
    
    fn load(self, name: Self::AssetName) -> Self::AssetRef;
    fn invalidate(self, name: Self::AssetName);
    fn clear(self);
}








