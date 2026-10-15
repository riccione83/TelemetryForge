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
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::LazyLock,
    time::Instant,
};

static ANIMATION_START: LazyLock<Instant> = LazyLock::new(Instant::now);

pub fn render(
    config: &AppConfig,
    sensors: &SensorSnapshot,
    remote_texts: &HashMap<String, String>,
) -> Result<RgbImage> {
    config.validate()?;
    let mut frame = background::create(config)?;
    advanced_widgets::draw_all(&mut frame, config, sensors, remote_texts)?;
    Ok(frame)
}

pub fn visual_signature(
    config: &AppConfig,
    sensors: &SensorSnapshot,
    remote_texts: &HashMap<String, String>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    for widget in config.widgets.iter().filter(|widget| widget.enabled) {
        widget.kind.hash(&mut hasher);
        widget.render_mode.hash(&mut hasher);
        numeric(widget.kind, sensors).is_some().hash(&mut hasher);
        if widget.kind == WidgetKind::Clock || widget.kind == WidgetKind::Date {
            chrono::Local::now()
                .format(if widget.kind == WidgetKind::Clock {
                    "%Y%m%d%H%M"
                } else {
                    "%Y%m%d"
                })
                .to_string()
                .hash(&mut hasher);
        }
        if widget.kind == WidgetKind::RemoteText {
            widget.remote_text_key.hash(&mut hasher);
            remote_texts.get(&widget.remote_text_key).hash(&mut hasher);
            continue;
        }
        if matches!(
            widget.kind,
            WidgetKind::WeatherCondition | WidgetKind::WeatherIcon
        ) {
            sensors.weather_condition.hash(&mut hasher);
            sensors.weather_code.hash(&mut hasher);
            continue;
        }
        if widget.kind == WidgetKind::Gif {
            let frame_duration = (1000 / widget.gif_fps.clamp(1, 30) as u128).max(1);
            let elapsed = ANIMATION_START.elapsed().as_millis();
            (elapsed / frame_duration).hash(&mut hasher);
            continue;
        }
        if widget.kind == WidgetKind::SuperWidget {
            for sensor in &sensors.fan_sensors {
                sensor.id.hash(&mut hasher);
                sensor.value.to_bits().hash(&mut hasher);
            }
            sensors
                .cpu_temperature_core
                .map(f32::to_bits)
                .hash(&mut hasher);
            sensors
                .cpu_temperature_socket
                .map(f32::to_bits)
                .hash(&mut hasher);
            if let Some(fps) = widget
                .superwidget_id
                .as_deref()
                .and_then(crate::superwidgets::find)
                .map(|manifest| manifest.animated_fps)
                .filter(|fps| *fps > 0)
            {
                let frame_duration = (1000 / fps.clamp(1, 30) as u128).max(1);
                (ANIMATION_START.elapsed().as_millis() / frame_duration).hash(&mut hasher);
            }
            for value in [
                sensors.cpu_usage,
                sensors.cpu_temperature,
                sensors.cpu_clock,
                sensors.gpu_usage,
                sensors.gpu_temperature,
                sensors.gpu_clock,
                sensors.fan_speed,
                sensors.gpu_power,
                sensors.ram_usage,
                sensors.vram_usage,
            ] {
                value.map(|value| value.round() as i64).hash(&mut hasher);
            }
            continue;
        }
        match widget.render_mode {
            WidgetRenderMode::Text => {
                if matches!(
                    widget.kind,
                    WidgetKind::NetworkUpload | WidgetKind::NetworkDownload
                ) {
                    // Auto units expose changes smaller than one raw KB/s.
                    advanced_widgets::label_text(widget, sensors, remote_texts).hash(&mut hasher);
                }
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
    let Some(value) = numeric(kind, sensors) else {
        return -1;
    };
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
        WidgetKind::WeatherTemperature => sensors.weather_temperature,
        WidgetKind::WeatherHumidity => sensors.weather_humidity,
        WidgetKind::WeatherWind => sensors.weather_wind_speed,
        WidgetKind::WeatherCondition => None,
        WidgetKind::WeatherIcon => None,
        WidgetKind::Clock
        | WidgetKind::Date
        | WidgetKind::Fps
        | WidgetKind::Text
        | WidgetKind::RemoteText
        | WidgetKind::Gif => None,
        WidgetKind::SuperWidget => None,
    }
}

fn maximum(kind: WidgetKind) -> f32 {
    match kind {
        WidgetKind::CpuClock => 6000.0,
        WidgetKind::GpuPower => 600.0,
        WidgetKind::GpuClock | WidgetKind::FanSpeed => 3000.0,
        WidgetKind::NetworkUpload | WidgetKind::NetworkDownload => 10000.0,
        WidgetKind::Volume => 100.0,
        WidgetKind::WeatherTemperature => 50.0,
        WidgetKind::WeatherHumidity => 100.0,
        WidgetKind::WeatherWind => 150.0,
        _ => 100.0,
    }
}

fn history(kind: WidgetKind, sensors: &SensorSnapshot) -> &[f32] {
    sensors.history(kind)
}
