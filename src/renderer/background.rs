use crate::config::schema::{AppConfig, BackgroundMode, BackgroundSource};
use anyhow::{Context, Result};
use image::{imageops, RgbImage};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn create(config: &AppConfig) -> Result<RgbImage> {
    let width = config.display.width;
    let height = config.display.height;
    let colour = super::canvas::parse_colour(&config.background.colour);
    let mut canvas = RgbImage::from_pixel(width, height, colour);
    let path = resolve_background_path(config)?;
    let Some(path) = path.as_deref() else {
        return Ok(canvas);
    };

    let source = image::open(path)
        .with_context(|| format!("Could not open background image {}", path.display()))?
        .to_rgb8();
    match config.background.mode {
        BackgroundMode::Stretch => {
            canvas = imageops::resize(&source, width, height, imageops::FilterType::Lanczos3);
        }
        BackgroundMode::Contain => {
            let (w, h) = fit(source.dimensions(), (width, height), false);
            let resized = imageops::resize(&source, w, h, imageops::FilterType::Lanczos3);
            imageops::overlay(
                &mut canvas,
                &resized,
                ((width - w) / 2) as i64,
                ((height - h) / 2) as i64,
            );
        }
        BackgroundMode::Cover => {
            let (w, h) = fit(source.dimensions(), (width, height), true);
            let resized = imageops::resize(&source, w, h, imageops::FilterType::Lanczos3);
            let x = (w - width) / 2;
            let y = (h - height) / 2;
            canvas = imageops::crop_imm(&resized, x, y, width, height).to_image();
        }
        BackgroundMode::Centre => {
            let x = width.saturating_sub(source.width()) / 2;
            let y = height.saturating_sub(source.height()) / 2;
            imageops::overlay(&mut canvas, &source, x as i64, y as i64);
        }
    }
    Ok(canvas)
}

pub fn resolve_background_path(config: &AppConfig) -> Result<Option<PathBuf>> {
    match config.background.source {
        BackgroundSource::Colour => Ok(None),
        BackgroundSource::File => Ok(config.background.path.as_deref().map(PathBuf::from)),
        BackgroundSource::Folder => {
            let Some(folder) = config.background.folder.as_deref() else {
                return Ok(None);
            };
            let mut images = fs::read_dir(folder)
                .with_context(|| format!("Could not read background folder {folder}"))?
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| is_supported_image(path))
                .collect::<Vec<_>>();
            images.sort_by_key(|path| path.to_string_lossy().to_lowercase());
            if images.is_empty() {
                return Ok(None);
            }
            let interval_seconds = config
                .background
                .slideshow_interval_minutes
                .max(1)
                .saturating_mul(60);
            let elapsed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let index = (elapsed / interval_seconds) as usize % images.len();
            Ok(Some(images[index].clone()))
        }
    }
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp"
            )
        })
}

fn fit(source: (u32, u32), target: (u32, u32), cover: bool) -> (u32, u32) {
    let sx = target.0 as f32 / source.0 as f32;
    let sy = target.1 as f32 / source.1 as f32;
    let scale = if cover { sx.max(sy) } else { sx.min(sy) };
    (
        (source.0 as f32 * scale).round().max(1.0) as u32,
        (source.1 as f32 * scale).round().max(1.0) as u32,
    )
}
