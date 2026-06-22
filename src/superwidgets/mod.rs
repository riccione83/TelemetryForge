pub mod renderer;

use crate::config::persistence;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

const CPU_MANIFEST: &str = include_str!("../../superwidgets/cpu-command-dial/manifest.yaml");
const GPU_MANIFEST: &str = include_str!("../../superwidgets/gpu-command-dial/manifest.yaml");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub template: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub sensors: Vec<String>,
}

pub fn ensure_bundled() -> Result<()> {
    for (id, contents) in [
        ("cpu-command-dial", CPU_MANIFEST),
        ("gpu-command-dial", GPU_MANIFEST),
    ] {
        let folder = directory().join(id);
        fs::create_dir_all(&folder)?;
        fs::write(folder.join("manifest.yaml"), contents)?;
    }
    Ok(())
}

pub fn list() -> Result<Vec<Manifest>> {
    ensure_bundled()?;
    let mut manifests: Vec<Manifest> = Vec::new();
    for entry in fs::read_dir(directory())?.flatten() {
        let path = entry.path().join("manifest.yaml");
        if !path.is_file() {
            continue;
        }
        if let Ok(text) = fs::read_to_string(path) {
            if let Ok(manifest) = serde_yaml::from_str(&text) {
                manifests.push(manifest);
            }
        }
    }
    manifests.sort_by_key(|manifest| manifest.name.to_lowercase());
    Ok(manifests)
}

pub fn find(id: &str) -> Option<Manifest> {
    list().ok()?.into_iter().find(|manifest| manifest.id == id)
}

pub fn directory() -> PathBuf {
    persistence::data_dir().join("superwidgets")
}
