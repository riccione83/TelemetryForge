use super::schema::AppConfig;
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn default_config_path() -> PathBuf {
    data_dir().join("config.yaml")
}

pub fn active_screen_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("active-screen.txt")
}

pub fn load_active_screen(config_path: &Path) -> Option<String> {
    let name = fs::read_to_string(active_screen_path(config_path)).ok()?;
    let name = name.trim();
    (!name.is_empty() && profile_path(config_path, name).ok()?.is_file()).then(|| name.to_string())
}

pub fn infer_active_screen(config_path: &Path, current: &AppConfig) -> Option<String> {
    list_profiles(config_path).ok()?.into_iter().find(|name| {
        let Ok(path) = profile_path(config_path, name) else {
            return false;
        };
        let Ok(mut profile) = load_or_create(&path) else {
            return false;
        };
        profile.automation = current.automation.clone();
        profile.transition = current.transition.clone();
        profile.libre_hardware_monitor_dll = current.libre_hardware_monitor_dll.clone();
        profile.cpu_temperature_source = current.cpu_temperature_source;
        profile.cpu_clock_source = current.cpu_clock_source;
        profile.fan_sensor = current.fan_sensor.clone();
        profile.weather = current.weather.clone();
        profile.remote = current.remote.clone();
        profile.quick_screens = current.quick_screens.clone();
        profile == *current
    })
}

pub fn save_active_screen(config_path: &Path, name: Option<&str>) -> Result<()> {
    let path = active_screen_path(config_path);
    if let Some(name) = name.filter(|name| !name.trim().is_empty()) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, name.trim())?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| PathBuf::from("."))
        })
        .join("TelemetryForge")
}

pub fn migrate_legacy_data(config_path: &Path) -> Result<()> {
    let data_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(data_dir)?;
    if config_path.exists() {
        return Ok(());
    }

    let executable_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let working_dir = std::env::current_dir().ok();
    let source_dir = executable_dir
        .into_iter()
        .chain(working_dir)
        .find(|directory| directory.join("config.yaml").is_file());

    let Some(source_dir) = source_dir else {
        return Ok(());
    };

    fs::copy(source_dir.join("config.yaml"), config_path)?;
    copy_directory_if_present(&source_dir.join("screens"), &data_dir.join("screens"))?;
    copy_directory_if_present(&source_dir.join("samples"), &data_dir.join("samples"))?;
    Ok(())
}

fn copy_directory_if_present(source: &Path, destination: &Path) -> Result<()> {
    if !source.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        if source_path.is_file() {
            let destination_path = destination.join(entry.file_name());
            if !destination_path.exists() {
                fs::copy(source_path, destination_path)?;
            }
        }
    }
    Ok(())
}

pub fn load_or_create(path: &Path) -> Result<AppConfig> {
    migrate_legacy_data(path)?;
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
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_the_active_screen_from_the_saved_config() {
        let root = std::env::temp_dir().join(format!(
            "telemetryforge-active-screen-{}",
            std::process::id()
        ));
        let config_path = root.join("config.yaml");
        let profile = AppConfig::default();
        let profile_path = profile_path(&config_path, "My Screen").unwrap();
        save(&profile_path, &profile).unwrap();
        save(&config_path, &profile).unwrap();

        assert_eq!(
            infer_active_screen(&config_path, &profile),
            Some("My Screen".into())
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn loads_the_installed_reactor_core_demo() {
        let path = data_dir().join("screens").join("Reactor Core Demo.yaml");
        if !path.is_file() {
            return;
        }
        let loaded = load_or_create(&path).unwrap();
        assert_eq!(loaded.theme.name, "Reactor Core");
        assert_eq!(
            loaded.widgets[0].superwidget_id.as_deref(),
            Some("sdk.reactor-core")
        );
    }
}
