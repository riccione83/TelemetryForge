pub mod renderer;
mod wasm_runtime;

use crate::config::persistence;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};
use zip::ZipArchive;

const CPU_MANIFEST: &str = include_str!("../../superwidgets/cpu-command-dial/manifest.yaml");
const GPU_MANIFEST: &str = include_str!("../../superwidgets/gpu-command-dial/manifest.yaml");
const HELLO_MANIFEST: &str = include_str!("../../sdk/examples/hello-dial/manifest.yaml");
const HELLO_WASM: &[u8] = include_bytes!("../../sdk/examples/hello-dial/widget.wasm");
const REACTOR_MANIFEST: &str = include_str!("../../sdk/examples/reactor-core/manifest.yaml");
const REACTOR_WASM: &[u8] = include_bytes!("../../sdk/examples/reactor-core/widget.wasm");
const COMMAND_DIALS_SCREEN: &str =
    include_str!("../../samples/superwidgets/screens/Command Dials.yaml");
const REACTOR_SCREEN: &str = include_str!("../../samples/superwidgets/screens/Reactor Core.yaml");
const HELLO_SCREEN: &str = include_str!("../../samples/superwidgets/screens/SDK Hello Dial.yaml");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub template: String,
    #[serde(default = "native_runtime")]
    pub runtime: String,
    #[serde(default)]
    pub entry: Option<String>,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub sensors: Vec<String>,
    #[serde(default = "abi_version")]
    pub abi_version: u32,
    #[serde(default)]
    pub animated_fps: u16,
}

fn native_runtime() -> String {
    "native".into()
}

const fn abi_version() -> u32 {
    1
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
    for (id, manifest, module) in [
        ("sdk.hello-dial", HELLO_MANIFEST, HELLO_WASM),
        ("sdk.reactor-core", REACTOR_MANIFEST, REACTOR_WASM),
    ] {
        let folder = directory().join(id);
        fs::create_dir_all(&folder)?;
        fs::write(folder.join("manifest.yaml"), manifest)?;
        fs::write(folder.join("widget.wasm"), module)?;
    }
    let screens = persistence::data_dir().join("screens");
    fs::create_dir_all(&screens)?;
    for (name, contents) in [
        ("Command Dials.yaml", COMMAND_DIALS_SCREEN),
        ("Reactor Core.yaml", REACTOR_SCREEN),
        ("SDK Hello Dial.yaml", HELLO_SCREEN),
    ] {
        let path = screens.join(name);
        if !path.exists() {
            fs::write(path, contents)?;
        }
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

pub fn component_dir(id: &str) -> PathBuf {
    directory().join(id)
}

pub fn install(package: &Path) -> Result<Manifest> {
    let mut archive = ZipArchive::new(File::open(package)?)?;
    let manifest: Manifest = {
        let mut entry = archive
            .by_name("manifest.yaml")
            .map_err(|_| anyhow::anyhow!("Package does not contain manifest.yaml"))?;
        serde_yaml::from_reader(&mut entry)?
    };
    validate_manifest(&manifest)?;
    let destination = component_dir(&manifest.id);
    if destination.exists() {
        fs::remove_dir_all(&destination)?;
    }
    fs::create_dir_all(&destination)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let output = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output)?;
        } else {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = File::create(output)?;
            std::io::copy(&mut entry, &mut file)?;
        }
    }
    let entry = manifest.entry.as_deref().unwrap_or("widget.wasm");
    if manifest.runtime == "wasm" && !destination.join(entry).is_file() {
        anyhow::bail!("Package does not contain {entry}");
    }
    Ok(manifest)
}

fn validate_manifest(manifest: &Manifest) -> Result<()> {
    if manifest.id.is_empty()
        || manifest.id.len() > 100
        || manifest.id.chars().any(|character| {
            !(character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_'))
        })
    {
        anyhow::bail!("Invalid Super Widget id");
    }
    if !matches!(manifest.runtime.as_str(), "native" | "wasm") {
        anyhow::bail!("Unsupported Super Widget runtime");
    }
    if manifest.abi_version != 1 {
        anyhow::bail!(
            "Unsupported Super Widget ABI version {}",
            manifest.abi_version
        );
    }
    Ok(())
}

pub fn directory() -> PathBuf {
    persistence::data_dir().join("superwidgets")
}
