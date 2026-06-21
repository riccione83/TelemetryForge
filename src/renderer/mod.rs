pub mod advanced_widgets;
pub mod background;
pub mod canvas;
pub mod fonts;

use crate::{
    config::schema::{AppConfig, WidgetKind, WidgetRenderMode},
    sensors::model::SensorSnapshot,
};
use anyhow::Result;
use image::RgbImage;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::LazyLock,
    time::Instant,
};

static ANIMATION_START: LazyLock<Instant> = LazyLock::new(Instant::now);

pub fn render(config: &AppConfig, sensors: &SensorSnapshot) -> Result<RgbImage> {
    let mut frame = background::create(config)?;
    advanced_widgets::draw_all(&mut frame, config, sensors)?;
    Ok(frame)
}

pub fn visual_signature(config: &AppConfig, sensors: &SensorSnapshot) -> u64 {
    let mut hasher = DefaultHasher::new();
    for widget in config.widgets.iter().filter(|widget| widget.enabled) {
        widget.kind.hash(&mut hasher);
        widget.render_mode.hash(&mut hasher);
        if widget.kind == WidgetKind::Gif {
            let frame_duration = (1000 / widget.gif_fps.clamp(1, 30) as u128).max(1);
            let elapsed = ANIMATION_START.elapsed().as_millis();
            (elapsed / frame_duration).hash(&mut hasher);
            continue;
        }
        match widget.render_mode {
            WidgetRenderMode::Text => {
                numeric(widget.kind, sensors)
                    .map(|value| value.round() as i64)
                    .hash(&mut hasher);
            }
            WidgetRenderMode::Bar => {
                quantized_ratio(widget.kind, sensors, widget.width.max(1) as f32).hash(&mut hasher);
            }
            WidgetRenderMode::Circle => {
                let radius = widget.width.min(widget.height) as f32 / 2.0;
                let steps = (radius * widget.circle_sweep_angle.to_radians())
                    .round()
                    .max(1.0);
                quantized_ratio(widget.kind, sensors, steps).hash(&mut hasher);
            }
            WidgetRenderMode::Graph => {
                for value in history(widget.kind, sensors) {
                    value.to_bits().hash(&mut hasher);
                }
            }
        }
    }
    hasher.finish()
}

fn quantized_ratio(kind: WidgetKind, sensors: &SensorSnapshot, steps: f32) -> i64 {
    let value = numeric(kind, sensors).unwrap_or_default();
    ((value / maximum(kind)).clamp(0.0, 1.0) * steps).floor() as i64
}

fn numeric(kind: WidgetKind, sensors: &SensorSnapshot) -> Option<f32> {
    match kind {
        WidgetKind::CpuTemperature => sensors.cpu_temperature,
        WidgetKind::CpuUsage => sensors.cpu_usage,
        WidgetKind::CpuClock => sensors.cpu_clock,
        WidgetKind::GpuTemperature => sensors.gpu_temperature,
        WidgetKind::GpuUsage => sensors.gpu_usage,
        WidgetKind::GpuClock => sensors.gpu_clock,
        WidgetKind::GpuPower => sensors.gpu_power,
        WidgetKind::RamUsage => sensors.ram_usage,
        WidgetKind::VramUsage => sensors.vram_usage,
        WidgetKind::DiskUsage => sensors.disk_usage,
        WidgetKind::NetworkUpload => sensors.network_upload,
        WidgetKind::NetworkDownload => sensors.network_download,
        WidgetKind::FanSpeed => sensors.fan_speed,
        WidgetKind::Volume => sensors.system_volume,
        WidgetKind::Clock
        | WidgetKind::Date
        | WidgetKind::Fps
        | WidgetKind::Text
        | WidgetKind::Gif => None,
    }
}

fn maximum(kind: WidgetKind) -> f32 {
    match kind {
        WidgetKind::CpuClock => 6000.0,
        WidgetKind::GpuPower => 600.0,
        WidgetKind::GpuClock | WidgetKind::FanSpeed => 3000.0,
        WidgetKind::NetworkUpload | WidgetKind::NetworkDownload => 10000.0,
        WidgetKind::Volume => 100.0,
        _ => 100.0,
    }
}

fn history(kind: WidgetKind, sensors: &SensorSnapshot) -> &[f32] {
    match kind {
        WidgetKind::CpuUsage | WidgetKind::CpuTemperature | WidgetKind::CpuClock => {
            &sensors.history_cpu
        }
        WidgetKind::GpuUsage | WidgetKind::GpuTemperature | WidgetKind::GpuClock => {
            &sensors.history_gpu
        }
        WidgetKind::GpuPower => &sensors.history_gpu_power,
        WidgetKind::NetworkUpload => &sensors.history_network_upload,
        WidgetKind::NetworkDownload => &sensors.history_network_download,
        WidgetKind::Volume => &sensors.history_volume,
        _ => &[],
    }
}
