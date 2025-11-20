use log::{debug, info};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct AssetMeta {
    pub id: String,
    pub path: PathBuf,
}

pub enum AssetData {
    SPIRVShader(Vec<u8>),
}

pub struct Asset {
    pub meta: AssetMeta,
    pub data: AssetData,
}

#[allow(unreachable_patterns)]
impl Asset {
    pub fn get_spirv(&self) -> anyhow::Result<&[u8]> {
        match &self.data {
            AssetData::SPIRVShader(shader) => Ok(shader),
            _ => anyhow::bail!("Asset {} is not a SPIRV shader", self.meta.id),
        }
    }
}

pub struct AssetManagerInner {
    assets_dir: PathBuf,
}

impl AssetManagerInner {
    // Traverses upwards until it finds a directory with an asset subdirectory
    fn find_assets_dir(pwd: &Path) -> anyhow::Result<PathBuf> {
        debug!("Looking for assets directory in {}", pwd.display());
        let assets_dir = pwd.join("assets");
        if assets_dir.exists() {
            Ok(assets_dir)
        } else {
            Self::find_assets_dir(pwd.parent().ok_or_else(|| {
                anyhow::anyhow!("Could not find assets directory in current or parent directories")
            })?)
        }
    }

    fn new_from_pwd(pwd: &Path) -> anyhow::Result<Self> {
        let assets_dir = Self::find_assets_dir(pwd)?;
        info!("Using assets directory: {}", assets_dir.display());
        Ok(Self { assets_dir })
    }

    fn load_asset(&self, id: &str) -> anyhow::Result<Asset> {
        let asset_path = self.assets_dir.join(id);
        if !asset_path.exists() {
            anyhow::bail!("Asset not found: {}", id);
        }

        let meta = AssetMeta {
            id: id.to_string(),
            path: asset_path.clone(),
        };
        // For simplicity, we assume all assets are SPIRV shaders in this example
        let data = AssetData::SPIRVShader(std::fs::read(&asset_path)?);

        info!("Loaded asset: {}", id);
        Ok(Asset { meta, data })
    }
}

#[derive(Clone)]
pub struct AssetManager(Rc<RefCell<AssetManagerInner>>);

impl AssetManager {
    pub fn new_from_pwd(pwd: &Path) -> anyhow::Result<Self> {
        Ok(Self(Rc::new(RefCell::new(
            AssetManagerInner::new_from_pwd(pwd)?,
        ))))
    }

    pub fn load_asset(&self, id: &str) -> anyhow::Result<Asset> {
        self.0.borrow_mut().load_asset(id)
    }
}
