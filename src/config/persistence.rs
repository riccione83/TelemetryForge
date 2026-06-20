use super::schema::AppConfig;
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn default_config_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config.yaml")
}

pub fn load_or_create(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        let config = AppConfig::default();
        save(path, &config)?;
        return Ok(config);
    }
    let yaml =
        fs::read_to_string(path).with_context(|| format!("Could not read {}", path.display()))?;
    serde_yaml::from_str(&yaml).context("Invalid config.yaml")
}

pub fn save(path: &Path, config: &AppConfig) -> Result<()> {
    let yaml = serde_yaml::to_string(config)?;
    fs::write(path, yaml).with_context(|| format!("Could not save {}", path.display()))
}

pub fn profiles_dir(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("screens")
}

pub fn profile_path(config_path: &Path, name: &str) -> Result<PathBuf> {
    let clean = name.trim();
    if clean.is_empty()
        || clean.len() > 80
        || clean
            .chars()
            .any(|c| !(c.is_alphanumeric() || matches!(c, ' ' | '-' | '_')))
    {
        anyhow::bail!("Invalid screen name");
    }
    Ok(profiles_dir(config_path).join(format!("{clean}.yaml")))
}

pub fn list_profiles(config_path: &Path) -> Result<Vec<String>> {
    let dir = profiles_dir(config_path);
    fs::create_dir_all(&dir)?;
    let mut names = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if !path
                .extension()?
                .to_string_lossy()
                .eq_ignore_ascii_case("yaml")
            {
                return None;
            }
            Some(path.file_stem()?.to_string_lossy().into_owned())
        })
        .collect::<Vec<_>>();
    names.sort_by_key(|name| name.to_lowercase());
    Ok(names)
}
