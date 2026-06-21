use crate::config::{persistence, schema::AppConfig};
use anyhow::{Context, Result};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

pub fn export(path: &Path, config: &AppConfig) -> Result<()> {
    let file = File::create(path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut portable = config.clone();

    if let Some(source) = portable.background.path.clone() {
        portable.background.path = add_asset(&mut zip, &source, "background", options)?;
    }
    for (index, widget) in portable.widgets.iter_mut().enumerate() {
        if let Some(source) = widget.gif_path.clone() {
            widget.gif_path = add_asset(&mut zip, &source, &format!("gif-{index}"), options)?;
        }
    }

    zip.start_file("config.yaml", options)?;
    zip.write_all(serde_yaml::to_string(&portable)?.as_bytes())?;
    zip.finish()?;
    Ok(())
}

pub fn import(path: &Path, name: &str) -> Result<AppConfig> {
    let destination = persistence::data_dir()
        .join("packages")
        .join(safe_name(name));
    fs::create_dir_all(&destination)?;
    let mut archive = ZipArchive::new(File::open(path)?)?;
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
    let mut config: AppConfig = serde_yaml::from_str(
        &fs::read_to_string(destination.join("config.yaml"))
            .context("Package does not contain config.yaml")?,
    )?;
    resolve_asset(&destination, &mut config.background.path);
    for widget in &mut config.widgets {
        resolve_asset(&destination, &mut widget.gif_path);
    }
    Ok(config)
}

fn add_asset(
    zip: &mut ZipWriter<File>,
    source: &str,
    name: &str,
    options: SimpleFileOptions,
) -> Result<Option<String>> {
    let path = Path::new(source);
    if !path.is_file() {
        return Ok(None);
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("bin");
    let archive_path = format!("assets/{name}.{extension}");
    zip.start_file(&archive_path, options)?;
    let mut source_file = File::open(path)?;
    let mut bytes = Vec::new();
    source_file.read_to_end(&mut bytes)?;
    zip.write_all(&bytes)?;
    Ok(Some(archive_path))
}

fn resolve_asset(root: &Path, value: &mut Option<String>) {
    if let Some(path) = value.as_deref() {
        if !Path::new(path).is_absolute() {
            *value = Some(root.join(path).to_string_lossy().into_owned());
        }
    }
}

fn safe_name(name: &str) -> String {
    let clean = name
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if clean.is_empty() {
        "imported".into()
    } else {
        clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{BackgroundSource, WidgetConfig, WidgetKind};

    #[test]
    fn package_round_trip_embeds_background_and_gif() {
        let root = std::env::temp_dir().join(format!(
            "telemetryforge-package-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let background = root.join("background.png");
        let gif = root.join("animation.gif");
        fs::write(&background, b"background-bytes").unwrap();
        fs::write(&gif, b"gif-bytes").unwrap();
        let package = root.join("sample.telemetryforge");

        let mut config = AppConfig::default();
        config.background.source = BackgroundSource::File;
        config.background.path = Some(background.to_string_lossy().into_owned());
        let mut widget = WidgetConfig::new(WidgetKind::Gif, 0, 0, "");
        widget.gif_path = Some(gif.to_string_lossy().into_owned());
        config.widgets = vec![widget];

        export(&package, &config).unwrap();
        let imported = import(&package, "package-test").unwrap();
        assert!(Path::new(imported.background.path.as_deref().unwrap()).is_file());
        assert!(Path::new(imported.widgets[0].gif_path.as_deref().unwrap()).is_file());
        let _ = fs::remove_dir_all(&root);
    }
}
