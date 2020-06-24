

pub trait AssetCache<Asset> {
    type AssetRef: std::ops::Deref<Target=Asset>;
    fn load(self, name: &'static str) -> Self::AssetRef;
    fn invalidate(self, name: &'static str);
    fn clear(self);
}

