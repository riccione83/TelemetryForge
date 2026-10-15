use crate::{
    config::schema::{AppConfig, BackgroundMode, WidgetConfig, WidgetKind, WidgetRenderMode},
    sensors::model::SensorSnapshot,
};
use anyhow::{Context, Result};
use chrono::Local;
use image::{
    codecs::gif::GifDecoder, imageops, AnimationDecoder, ImageDecoder, RgbImage, Rgba, RgbaImage,
};
use imageproc::{
    drawing::{draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut},
    rect::Rect,
};
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    sync::{Arc, LazyLock, Mutex},
    time::{Instant, UNIX_EPOCH},
};

static GIF_CACHE: LazyLock<Mutex<HashMap<String, Arc<Vec<RgbaImage>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static ANIMATION_START: LazyLock<Instant> = LazyLock::new(Instant::now);

#[derive(Default)]
pub struct LayerCache {
    entries: Vec<Option<CachedLayer>>,
    pub pending: bool,
    pending_until: Option<Instant>,
}
impl LayerCache {
    pub fn refresh_due(&self) -> bool {
        self.pending_until
            .is_some_and(|deadline| Instant::now() >= deadline)
    }
}
struct CachedLayer {
    config: WidgetConfig,
    signature: u64,
    updated: Instant,
    layers: Vec<(RgbaImage, i32, i32)>,
}

pub fn draw_all(
    image: &mut RgbImage,
    config: &AppConfig,
    sensors: &SensorSnapshot,
    remote_texts: &HashMap<String, String>,
) -> Result<()> {
    draw_cached(
        image,
        config,
        sensors,
        remote_texts,
        &mut LayerCache::default(),
    )
}

pub fn draw_cached(
    image: &mut RgbImage,
    config: &AppConfig,
    sensors: &SensorSnapshot,
    remote_texts: &HashMap<String, String>,
    cache: &mut LayerCache,
) -> Result<()> {
    cache.entries.resize_with(config.widgets.len(), || None);
    cache.pending = false;
    cache.pending_until = None;
    let mut single = config.clone();
    single.widgets.clear();
    for (index, widget) in config.widgets.iter().enumerate() {
        if !widget.enabled {
            cache.entries[index] = None;
            continue;
        }
        single.widgets.clear();
        single.widgets.push(widget.clone());
        let signature = super::visual_signature(&single, sensors, remote_texts);
        let old = &cache.entries[index];
        let config_changed = old.as_ref().is_none_or(|old| old.config != *widget);
        let changed = old.as_ref().is_none_or(|old| old.signature != signature);
        let animated = widget.kind == WidgetKind::Gif
            || (widget.kind == WidgetKind::SuperWidget
                && widget
                    .superwidget_id
                    .as_deref()
                    .and_then(crate::superwidgets::find)
                    .is_some_and(|m| m.animated_fps > 0));
        let due = animated
            || old.as_ref().is_none_or(|old| {
                old.updated.elapsed().as_millis()
                    >= widget.refresh_interval_ms.clamp(50, 60_000) as u128
            });
        if config_changed || (changed && due) {
            cache.entries[index] = Some(CachedLayer {
                config: widget.clone(),
                signature,
                updated: Instant::now(),
                layers: widget_layers(widget, sensors, remote_texts)?,
            });
        } else if changed {
            cache.pending = true;
            if let Some(old) = old {
                let deadline = old.updated
                    + std::time::Duration::from_millis(
                        widget.refresh_interval_ms.clamp(50, 60_000),
                    );
                cache.pending_until = Some(
                    cache
                        .pending_until
                        .map_or(deadline, |previous| previous.min(deadline)),
                );
            }
        }
        if let Some(cached) = &cache.entries[index] {
            for (layer, x, y) in &cached.layers {
                composite(image, layer, *x, *y);
            }
        }
    }
    Ok(())
}

fn widget_layers(
    widget: &WidgetConfig,
    sensors: &SensorSnapshot,
    remote_texts: &HashMap<String, String>,
) -> Result<Vec<(RgbaImage, i32, i32)>> {
    if widget.kind == WidgetKind::Gif {
        let mut layer = gif_layer(widget)?;
        apply_opacity(&mut layer, widget.opacity);
        return Ok(vec![(layer, widget.x, widget.y)]);
    }
    if widget.kind == WidgetKind::SuperWidget {
        let mut layer = superwidget_layer(widget, sensors)?;
        apply_opacity(&mut layer, widget.opacity);
        return Ok(vec![(layer, widget.x, widget.y)]);
    }
    let padding = widget
        .glow
        .saturating_mul(3)
        .max(widget.shadow.saturating_mul(2))
        .saturating_add(6) as u32;
    let mut local_widget = widget.clone();
    local_widget.x = padding as i32;
    local_widget.y = padding as i32;
    let value = numeric(widget.kind, sensors);
    let primary = if widget.use_thresholds
        && matches!(
            widget.render_mode,
            WidgetRenderMode::Bar | WidgetRenderMode::Circle | WidgetRenderMode::Graph
        ) {
        parse(&widget.colour, widget.opacity)
    } else {
        opacity(active_colour(widget, value), widget.opacity)
    };
    let secondary = parse(&widget.secondary_colour, widget.opacity);
    let mut layer = RgbaImage::new(
        widget.width.max(1).saturating_add(padding * 2),
        widget.height.max(1).saturating_add(padding * 2),
    );
    match widget.render_mode {
        WidgetRenderMode::Text => {
            let font = super::fonts::load(&widget.font)?;
            let text = label_text(widget, sensors, remote_texts);
            if widget.kind == WidgetKind::WeatherIcon {
                draw_centred_glyph(&mut layer, &local_widget, primary, &font, &text);
            } else {
                draw_text_mut(
                    &mut layer,
                    primary,
                    local_widget.x,
                    local_widget.y,
                    widget.font_size,
                    &font,
                    &text,
                );
            }
        }
        WidgetRenderMode::Bar => bar(
            &mut layer,
            &local_widget,
            ratio(widget.kind, value),
            primary,
            secondary,
        ),
        WidgetRenderMode::Circle => circle(
            &mut layer,
            &local_widget,
            ratio(widget.kind, value),
            primary,
            secondary,
        ),
        WidgetRenderMode::Graph => graph(
            &mut layer,
            &local_widget,
            if value.is_none() {
                &[]
            } else {
                history(widget.kind, sensors)
            },
            primary,
            secondary,
        ),
    }
    if value.is_none()
        && widget.render_mode != WidgetRenderMode::Text
        && !matches!(
            widget.kind,
            WidgetKind::Text | WidgetKind::Clock | WidgetKind::Date | WidgetKind::RemoteText
        )
    {
        let font = super::fonts::load(&widget.font)?;
        let mut placeholder = local_widget.clone();
        placeholder.font_size = widget.font_size.min(widget.height as f32).max(8.0);
        draw_centred_glyph(&mut layer, &placeholder, primary, &font, "--");
    }
    let origin_x = widget.x - padding as i32;
    let origin_y = widget.y - padding as i32;
    let mut layers = Vec::new();
    if widget.shadow > 0 {
        layers.push((
            tint(
                &imageops::blur(&layer, widget.shadow as f32 / 2.0),
                Rgba([0, 0, 0, 160]),
            ),
            origin_x + 3,
            origin_y + 3,
        ));
    }
    if widget.glow > 0 {
        layers.push((
            imageops::blur(&layer, widget.glow as f32),
            origin_x,
            origin_y,
        ));
    }
    layers.push((layer, origin_x, origin_y));
    Ok(layers)
}

fn apply_opacity(layer: &mut RgbaImage, opacity: f32) {
    if opacity >= 1.0 {
        return;
    }
    for pixel in layer.pixels_mut() {
        pixel[3] = (pixel[3] as f32 * opacity.clamp(0.0, 1.0)) as u8;
    }
}

fn superwidget_layer(widget: &WidgetConfig, sensors: &SensorSnapshot) -> Result<RgbaImage> {
    let Some(id) = widget.superwidget_id.as_deref() else {
        return Ok(RgbaImage::new(1, 1));
    };
    let Some(manifest) = crate::superwidgets::find(id) else {
        return Ok(RgbaImage::new(1, 1));
    };
    crate::superwidgets::renderer::draw(
        &manifest,
        widget.width,
        widget.height,
        sensors,
        &widget.superwidget_background_colour,
        widget.superwidget_background_opacity,
        &widget.superwidget_bindings,
    )
}

fn gif_layer(widget: &WidgetConfig) -> Result<RgbaImage> {
    let Some(path) = widget.gif_path.as_deref().filter(|path| !path.is_empty()) else {
        return Ok(RgbaImage::new(1, 1));
    };
    let modified = std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let key = format!("{path}:{modified}");
    let frames = {
        let mut cache = GIF_CACHE.lock().expect("GIF cache poisoned");
        if !cache.contains_key(&key) {
            let mut decoder = GifDecoder::new(BufReader::new(
                File::open(path).with_context(|| format!("Could not open GIF {path}"))?,
            ))
            .with_context(|| format!("Could not decode GIF {path}"))?;
            let mut limits = image::Limits::default();
            limits.max_alloc = Some(64 * 1024 * 1024);
            limits.max_image_width = Some(4096);
            limits.max_image_height = Some(4096);
            decoder.set_limits(limits)?;
            let mut decoded = Vec::new();
            let mut bytes = 0usize;
            for frame in decoder.into_frames() {
                let frame = frame?.into_buffer();
                bytes = bytes.saturating_add(frame.as_raw().len());
                anyhow::ensure!(
                    bytes <= 64 * 1024 * 1024 && decoded.len() < 2000,
                    "GIF exceeds the decoded memory/frame limit (64 MiB / 2000 frames)"
                );
                decoded.push(frame);
            }
            cache.retain(|cached_key, _| !cached_key.starts_with(&format!("{path}:")));
            let cached_bytes: usize = cache
                .values()
                .flat_map(|frames| frames.iter())
                .map(|frame| frame.as_raw().len())
                .sum();
            if cached_bytes + bytes > 128 * 1024 * 1024 {
                cache.clear();
            }
            cache.insert(key.clone(), Arc::new(decoded));
        }
        cache.get(&key).cloned().unwrap_or_default()
    };
    if frames.is_empty() || widget.width == 0 || widget.height == 0 {
        return Ok(RgbaImage::new(1, 1));
    }
    let elapsed_ms = ANIMATION_START.elapsed().as_millis();
    let frame_duration = (1000 / widget.gif_fps.clamp(1, 30) as u128).max(1);
    let raw_index = (elapsed_ms / frame_duration) as usize;
    let index = if widget.gif_loop {
        raw_index % frames.len()
    } else {
        raw_index.min(frames.len() - 1)
    };
    Ok(fit_gif_frame(
        &frames[index],
        widget.width,
        widget.height,
        widget.gif_fit,
    ))
}

fn fit_gif_frame(source: &RgbaImage, width: u32, height: u32, mode: BackgroundMode) -> RgbaImage {
    match mode {
        BackgroundMode::Stretch => {
            imageops::resize(source, width, height, imageops::FilterType::Lanczos3)
        }
        BackgroundMode::Contain => {
            let scale =
                (width as f32 / source.width() as f32).min(height as f32 / source.height() as f32);
            let target_width = (source.width() as f32 * scale).round().max(1.0) as u32;
            let target_height = (source.height() as f32 * scale).round().max(1.0) as u32;
            let resized = imageops::resize(
                source,
                target_width,
                target_height,
                imageops::FilterType::Lanczos3,
            );
            let mut canvas = RgbaImage::new(width, height);
            imageops::overlay(
                &mut canvas,
                &resized,
                ((width - target_width) / 2) as i64,
                ((height - target_height) / 2) as i64,
            );
            canvas
        }
        BackgroundMode::Cover => {
            let scale =
                (width as f32 / source.width() as f32).max(height as f32 / source.height() as f32);
            let target_width = (source.width() as f32 * scale).round().max(1.0) as u32;
            let target_height = (source.height() as f32 * scale).round().max(1.0) as u32;
            let resized = imageops::resize(
                source,
                target_width,
                target_height,
                imageops::FilterType::Lanczos3,
            );
            imageops::crop_imm(
                &resized,
                (target_width - width) / 2,
                (target_height - height) / 2,
                width,
                height,
            )
            .to_image()
        }
        BackgroundMode::Centre => {
            let mut canvas = RgbaImage::new(width, height);
            let x = (width as i64 - source.width() as i64) / 2;
            let y = (height as i64 - source.height() as i64) / 2;
            imageops::overlay(&mut canvas, source, x, y);
            canvas
        }
    }
}

fn bar(layer: &mut RgbaImage, w: &WidgetConfig, ratio: f32, a: Rgba<u8>, b: Rgba<u8>) {
    let fill = (w.width as f32 * ratio) as u32;
    for py in 0..w.height {
        for px in 0..w.width {
            let x = w.x + px as i32;
            let y = w.y + py as i32;
            if !inside(layer, x, y) {
                continue;
            }
            let edge = px == 0 || py == 0 || px + 1 == w.width || py + 1 == w.height;
            let c = if edge {
                opacity(a, 0.7)
            } else if px < fill {
                indicator_colour(w, a, b, px as f32 / w.width.max(1) as f32)
            } else {
                Rgba([a[0] / 5, a[1] / 5, a[2] / 5, a[3] / 2])
            };
            layer.put_pixel(x as u32, y as u32, c);
        }
    }
}

fn circle(layer: &mut RgbaImage, w: &WidgetConfig, ratio: f32, a: Rgba<u8>, b: Rgba<u8>) {
    let diameter = w.width.min(w.height);
    let radius = diameter as f32 / 2.0;
    if radius < 4.0 {
        return;
    }
    let thickness = w.circle_thickness.clamp(1.0, radius);
    let cx = w.x as f32 + w.width as f32 / 2.0;
    let cy = w.y as f32 + w.height as f32 / 2.0;
    let start = w.circle_start_angle.to_radians();
    let sweep = w.circle_sweep_angle.clamp(1.0, 360.0).to_radians();
    for y in w.y.max(0)..(w.y + w.height as i32).min(layer.height() as i32) {
        for x in w.x.max(0)..(w.x + w.width as i32).min(layer.width() as i32) {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            if d < radius - thickness || d > radius {
                continue;
            }
            let mut angle = dy.atan2(dx) - start;
            while angle < 0.0 {
                angle += std::f32::consts::TAU;
            }
            if angle > sweep {
                continue;
            }
            let c = if angle <= sweep * ratio {
                indicator_colour(w, a, b, angle / sweep)
            } else {
                Rgba([a[0] / 5, a[1] / 5, a[2] / 5, a[3] / 2])
            };
            layer.put_pixel(x as u32, y as u32, c);
        }
    }
}

fn graph(layer: &mut RgbaImage, w: &WidgetConfig, values: &[f32], a: Rgba<u8>, b: Rgba<u8>) {
    if w.width < 2 || w.height < 2 {
        return;
    }
    if w.graph_background_opacity > 0.0 {
        let colour = parse(
            &w.graph_background_colour,
            w.graph_background_opacity.clamp(0.0, 1.0),
        );
        for y in w.y.max(0)..(w.y + w.height as i32).min(layer.height() as i32) {
            for x in w.x.max(0)..(w.x + w.width as i32).min(layer.width() as i32) {
                layer.put_pixel(x as u32, y as u32, colour);
            }
        }
    }
    if values.len() < 2 {
        draw_hollow_rect_mut(
            layer,
            Rect::at(w.x, w.y).of_size(w.width, w.height),
            opacity(a, 0.35),
        );
        return;
    }
    let count = values.len().min(w.width as usize);
    let values = &values[values.len() - count..];
    let max = max_for(w.kind).max(values.iter().copied().fold(0.0, f32::max));
    for i in 1..values.len() {
        let x0 = w.x as f32 + (i - 1) as f32 * (w.width - 1) as f32 / (values.len() - 1) as f32;
        let x1 = w.x as f32 + i as f32 * (w.width - 1) as f32 / (values.len() - 1) as f32;
        let y0 =
            w.y as f32 + w.height as f32 - (values[i - 1] / max).clamp(0.0, 1.0) * w.height as f32;
        let y1 = w.y as f32 + w.height as f32 - (values[i] / max).clamp(0.0, 1.0) * w.height as f32;
        draw_line_segment_mut(
            layer,
            (x0, y0),
            (x1, y1),
            graph_colour(
                w,
                a,
                b,
                values[i - 1],
                values[i],
                i as f32 / values.len() as f32,
            ),
        );
    }
    draw_hollow_rect_mut(
        layer,
        Rect::at(w.x, w.y).of_size(w.width, w.height),
        opacity(a, 0.35),
    );
}

fn active_colour(w: &WidgetConfig, value: Option<f32>) -> Rgba<u8> {
    if w.use_thresholds {
        if let Some(v) = value {
            if v >= w.critical_threshold {
                return parse(&w.critical_colour, 1.0);
            }
            if v >= w.warning_threshold {
                return parse(&w.warning_colour, 1.0);
            }
        }
    }
    parse(&w.colour, 1.0)
}

fn indicator_colour(
    widget: &WidgetConfig,
    base: Rgba<u8>,
    secondary: Rgba<u8>,
    progress: f32,
) -> Rgba<u8> {
    let progress = progress.clamp(0.0, 1.0);
    if !widget.use_thresholds {
        return gradient(base, secondary, progress);
    }

    let maximum = max_for(widget.kind).max(1.0);
    let warning_point = (widget.warning_threshold / maximum).clamp(0.0, 1.0);
    let critical_point = (widget.critical_threshold / maximum).clamp(warning_point, 1.0);
    let warning = parse(&widget.warning_colour, widget.opacity);
    let critical = parse(&widget.critical_colour, widget.opacity);

    if progress <= warning_point {
        let amount = if warning_point > 0.0 {
            progress / warning_point
        } else {
            1.0
        };
        gradient(base, warning, amount)
    } else if progress <= critical_point {
        let span = critical_point - warning_point;
        let amount = if span > 0.0 {
            (progress - warning_point) / span
        } else {
            1.0
        };
        gradient(warning, critical, amount)
    } else {
        critical
    }
}

fn graph_colour(
    widget: &WidgetConfig,
    base: Rgba<u8>,
    secondary: Rgba<u8>,
    previous_value: f32,
    current_value: f32,
    horizontal_progress: f32,
) -> Rgba<u8> {
    if !widget.use_thresholds {
        return gradient(base, secondary, horizontal_progress);
    }
    let value = (previous_value + current_value) / 2.0;
    threshold_value_colour(widget, base, value)
}

fn threshold_value_colour(widget: &WidgetConfig, base: Rgba<u8>, value: f32) -> Rgba<u8> {
    let warning = widget.warning_threshold.max(0.0);
    let critical = widget.critical_threshold.max(warning);
    let warning_colour = parse(&widget.warning_colour, widget.opacity);
    let critical_colour = parse(&widget.critical_colour, widget.opacity);

    if value <= warning {
        let amount = if warning > 0.0 {
            (value / warning).clamp(0.0, 1.0)
        } else {
            1.0
        };
        gradient(base, warning_colour, amount)
    } else if value <= critical {
        let span = critical - warning;
        let amount = if span > 0.0 {
            ((value - warning) / span).clamp(0.0, 1.0)
        } else {
            1.0
        };
        gradient(warning_colour, critical_colour, amount)
    } else {
        critical_colour
    }
}
fn numeric(k: WidgetKind, s: &SensorSnapshot) -> Option<f32> {
    match k {
        WidgetKind::CpuTemperature => s.cpu_temperature,
        WidgetKind::CpuUsage => s.cpu_usage,
        WidgetKind::CpuClock => s.cpu_clock,
        WidgetKind::GpuTemperature => s.gpu_temperature,
        WidgetKind::GpuUsage => s.gpu_usage,
        WidgetKind::GpuClock => s.gpu_clock,
        WidgetKind::GpuPower => s.gpu_power,
        WidgetKind::RamUsage => s.ram_usage,
        WidgetKind::VramUsage => s.vram_usage,
        WidgetKind::DiskUsage => s.disk_usage,
        WidgetKind::NetworkUpload => s.network_upload,
        WidgetKind::NetworkDownload => s.network_download,
        WidgetKind::FanSpeed => s.fan_speed,
        WidgetKind::Volume => s.system_volume,
        WidgetKind::WeatherTemperature => s.weather_temperature,
        WidgetKind::WeatherHumidity => s.weather_humidity,
        WidgetKind::WeatherWind => s.weather_wind_speed,
        WidgetKind::WeatherCondition => None,
        WidgetKind::WeatherIcon => None,
        WidgetKind::RemoteText => None,
        WidgetKind::Gif => None,
        WidgetKind::SuperWidget => None,
        _ => None,
    }
}
pub(super) fn label_text(
    widget: &WidgetConfig,
    sensors: &SensorSnapshot,
    remote_texts: &HashMap<String, String>,
) -> String {
    let value = shown(widget, sensors, remote_texts);
    let template = if matches!(
        widget.kind,
        WidgetKind::NetworkUpload | WidgetKind::NetworkDownload
    ) && widget.label_format.contains("{speed}")
    {
        widget
            .label_format
            .replace("{speed}", &network_speed(numeric(widget.kind, sensors)))
    } else {
        widget.label_format.clone()
    };
    let middle = template
        .replace("{value:.0}", &value)
        .replace("{value}", &value);
    format!("{}{}{}", widget.left_text, middle, widget.right_text)
}

/// The poller stores bytes / 1024 / second (historically labelled KB/s).
/// Convert to decimal bit rates only for display; keep sensor/history values intact.
fn network_speed(value: Option<f32>) -> String {
    let Some(value) = value.filter(|value| value.is_finite() && *value >= 0.0) else {
        return "--".into();
    };
    let mut rate = f64::from(value) * 1024.0 * 8.0 / 1000.0;
    let units = ["Kbit/s", "Mbit/s", "Gbit/s"];
    let mut unit = 0;
    // Promote values that would round to 1000.0 in the smaller unit, too.
    while unit < units.len() - 1 && rate >= 999.95 {
        rate /= 1000.0;
        unit += 1;
    }
    let number = format!("{rate:.1}");
    format!(
        "{} {}",
        number.strip_suffix(".0").unwrap_or(&number),
        units[unit]
    )
}

fn shown(
    widget: &WidgetConfig,
    s: &SensorSnapshot,
    remote_texts: &HashMap<String, String>,
) -> String {
    match widget.kind {
        WidgetKind::Clock => Local::now().format("%H:%M").to_string(),
        WidgetKind::Date => Local::now().format("%d/%m/%Y").to_string(),
        WidgetKind::CpuClock => frequency(s.cpu_clock),
        WidgetKind::GpuClock => frequency(s.gpu_clock),
        WidgetKind::Text => String::new(),
        WidgetKind::RemoteText => remote_texts
            .get(&widget.remote_text_key)
            .cloned()
            .unwrap_or_else(|| "Waiting…".into()),
        WidgetKind::Gif => String::new(),
        WidgetKind::SuperWidget => String::new(),
        WidgetKind::Fps => "--".into(),
        WidgetKind::WeatherCondition => s.weather_condition.clone().unwrap_or_else(|| "--".into()),
        WidgetKind::WeatherIcon => s.weather_code.map(weather_icon).unwrap_or("--").into(),
        _ => numeric(widget.kind, s)
            .map(|v| format!("{v:.0}"))
            .unwrap_or_else(|| "--".into()),
    }
}

fn draw_centred_glyph(
    layer: &mut RgbaImage,
    widget: &WidgetConfig,
    colour: Rgba<u8>,
    font: &ab_glyph::FontArc,
    text: &str,
) {
    let margin = widget.font_size.ceil().max(8.0) as u32;
    let scratch_width = widget
        .width
        .max((widget.font_size * 3.0).ceil() as u32)
        .saturating_add(margin * 2);
    let scratch_height = widget
        .height
        .max((widget.font_size * 3.0).ceil() as u32)
        .saturating_add(margin * 2);
    let mut scratch = RgbaImage::new(scratch_width.max(1), scratch_height.max(1));
    draw_text_mut(
        &mut scratch,
        colour,
        margin as i32,
        margin as i32,
        widget.font_size,
        font,
        text,
    );
    let Some((left, top, right, bottom)) = visible_alpha_bounds(&scratch) else {
        return;
    };
    let glyph_width = right - left + 1;
    let glyph_height = bottom - top + 1;
    let target_x = widget.x + (widget.width as i32 - glyph_width as i32) / 2;
    let target_y = widget.y + (widget.height as i32 - glyph_height as i32) / 2;
    for y in 0..glyph_height {
        for x in 0..glyph_width {
            let pixel = scratch.get_pixel(left + x, top + y);
            if pixel[3] == 0 {
                continue;
            }
            let destination_x = target_x + x as i32;
            let destination_y = target_y + y as i32;
            if inside(layer, destination_x, destination_y) {
                layer.put_pixel(destination_x as u32, destination_y as u32, *pixel);
            }
        }
    }
}

fn visible_alpha_bounds(image: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    alpha_bounds_above(image, 8)
}

fn alpha_bounds_above(image: &RgbaImage, threshold: u8) -> Option<(u32, u32, u32, u32)> {
    let mut left = image.width();
    let mut top = image.height();
    let mut right = 0;
    let mut bottom = 0;
    let mut found = false;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] <= threshold {
            continue;
        }
        found = true;
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    found.then_some((left, top, right, bottom))
}

fn weather_icon(code: u16) -> &'static str {
    match code {
        0 => "☀",
        1 => "☀",
        2 => "⛅",
        3 => "☁",
        45 | 48 => "≋",
        51 | 53 | 55 | 56 | 57 => "☂",
        61 | 63 | 65 | 66 | 67 | 80..=82 => "☔",
        71 | 73 | 75 | 77 | 85 | 86 => "❄",
        95..=99 => "⚡",
        _ => "?",
    }
}

fn frequency(value: Option<f32>) -> String {
    value
        .map(|mhz| {
            if mhz >= 1000.0 {
                let ghz = mhz / 1000.0;
                if ghz >= 10.0 {
                    format!("{ghz:.1} GHz")
                } else {
                    format!("{ghz:.2} GHz")
                }
            } else {
                format!("{mhz:.0} MHz")
            }
        })
        .unwrap_or_else(|| "--".into())
}
fn max_for(k: WidgetKind) -> f32 {
    match k {
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
fn ratio(k: WidgetKind, v: Option<f32>) -> f32 {
    (v.unwrap_or(0.0) / max_for(k)).clamp(0.0, 1.0)
}
fn history(k: WidgetKind, s: &SensorSnapshot) -> &[f32] {
    s.history(k)
}
fn parse(s: &str, o: f32) -> Rgba<u8> {
    let c = super::canvas::parse_colour(s);
    Rgba([c[0], c[1], c[2], (o.clamp(0.0, 1.0) * 255.0) as u8])
}
fn opacity(mut c: Rgba<u8>, o: f32) -> Rgba<u8> {
    c[3] = (c[3] as f32 * o.clamp(0.0, 1.0)) as u8;
    c
}
fn gradient(a: Rgba<u8>, b: Rgba<u8>, t: f32) -> Rgba<u8> {
    let t = t.clamp(0.0, 1.0);
    Rgba([
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
        (a[3] as f32 + (b[3] as f32 - a[3] as f32) * t) as u8,
    ])
}
fn tint(src: &RgbaImage, c: Rgba<u8>) -> RgbaImage {
    let mut out = RgbaImage::new(src.width(), src.height());
    for (x, y, p) in src.enumerate_pixels() {
        out.put_pixel(
            x,
            y,
            Rgba([c[0], c[1], c[2], (p[3] as u16 * c[3] as u16 / 255) as u8]),
        );
    }
    out
}
fn composite(dst: &mut RgbImage, src: &RgbaImage, ox: i32, oy: i32) {
    for (x, y, p) in src.enumerate_pixels() {
        let tx = x as i32 + ox;
        let ty = y as i32 + oy;
        if tx < 0 || ty < 0 || tx >= dst.width() as i32 || ty >= dst.height() as i32 {
            continue;
        }
        let a = p[3] as f32 / 255.0;
        if a == 0.0 {
            continue;
        }
        let d = dst.get_pixel_mut(tx as u32, ty as u32);
        for i in 0..3 {
            d[i] = (d[i] as f32 * (1.0 - a) + p[i] as f32 * a) as u8;
        }
    }
}
fn inside(i: &RgbaImage, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < i.width() as i32 && y < i.height() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_layers_preserve_overlap_and_respect_refresh_intervals() {
        let mut config = AppConfig::default();
        let mut bar = WidgetConfig::new(WidgetKind::CpuUsage, 5, 5, "{value}");
        bar.render_mode = WidgetRenderMode::Bar;
        bar.width = 70;
        bar.height = 25;
        bar.refresh_interval_ms = 60_000;
        let mut overlay = bar.clone();
        overlay.kind = WidgetKind::RamUsage;
        overlay.x = 15;
        overlay.y = 10;
        overlay.opacity = 0.4;
        config.widgets = vec![bar, overlay];
        let mut sensors = SensorSnapshot {
            cpu_usage: Some(10.0),
            ram_usage: Some(50.0),
            ..Default::default()
        };
        let mut cache = LayerCache::default();
        let texts = HashMap::new();
        let mut first = super::super::background::create(&config).unwrap();
        draw_cached(&mut first, &config, &sensors, &texts, &mut cache).unwrap();
        assert_eq!(
            first,
            super::super::render(&config, &sensors, &texts).unwrap()
        );
        sensors.cpu_usage = Some(95.0);
        let mut held = super::super::background::create(&config).unwrap();
        draw_cached(&mut held, &config, &sensors, &texts, &mut cache).unwrap();
        assert_eq!(held, first);
        assert!(cache.pending);
        assert!(
            !cache.refresh_due(),
            "pending layers must not trigger premature recomposition"
        );
        cache.pending_until = Some(Instant::now() - std::time::Duration::from_millis(1));
        assert!(cache.refresh_due());
        cache.entries[0].as_mut().unwrap().updated =
            Instant::now() - std::time::Duration::from_secs(61);
        let mut changed = super::super::background::create(&config).unwrap();
        draw_cached(&mut changed, &config, &sensors, &texts, &mut cache).unwrap();
        assert_eq!(
            changed,
            super::super::render(&config, &sensors, &texts).unwrap()
        );
        assert_ne!(first, changed);
    }

    fn threshold_widget() -> WidgetConfig {
        let mut widget = WidgetConfig::new(WidgetKind::CpuUsage, 0, 0, "{value}");
        widget.render_mode = WidgetRenderMode::Bar;
        widget.use_thresholds = true;
        widget.colour = "#0000ff".into();
        widget.warning_colour = "#ffff00".into();
        widget.critical_colour = "#ff0000".into();
        widget.warning_threshold = 70.0;
        widget.critical_threshold = 90.0;
        widget
    }

    #[test]
    fn missing_sensor_is_not_rendered_or_cached_as_zero() {
        let mut config = AppConfig::default();
        let texts = HashMap::new();
        let missing = SensorSnapshot::default();
        let zero = SensorSnapshot {
            gpu_usage: Some(0.0),
            ..Default::default()
        };
        for mode in [
            WidgetRenderMode::Bar,
            WidgetRenderMode::Circle,
            WidgetRenderMode::Graph,
        ] {
            let mut widget = WidgetConfig::new(WidgetKind::GpuUsage, 0, 0, "{value}");
            widget.width = 100;
            widget.height = 50;
            widget.render_mode = mode;
            config.widgets = vec![widget];
            assert_ne!(
                super::super::visual_signature(&config, &missing, &texts),
                super::super::visual_signature(&config, &zero, &texts)
            );
            assert_ne!(
                super::super::render(&config, &missing, &texts).unwrap(),
                super::super::render(&config, &zero, &texts).unwrap()
            );
        }
    }

    #[test]
    fn threshold_gradient_hits_each_configured_colour() {
        let widget = threshold_widget();
        let base = parse(&widget.colour, 1.0);
        let secondary = parse(&widget.secondary_colour, 1.0);
        assert_eq!(indicator_colour(&widget, base, secondary, 0.0), base);
        assert_eq!(
            indicator_colour(&widget, base, secondary, 0.7),
            parse(&widget.warning_colour, 1.0)
        );
        assert_eq!(
            indicator_colour(&widget, base, secondary, 0.9),
            parse(&widget.critical_colour, 1.0)
        );
        assert_eq!(
            indicator_colour(&widget, base, secondary, 1.0),
            parse(&widget.critical_colour, 1.0)
        );
    }

    #[test]
    fn normal_gradient_is_preserved_when_thresholds_are_disabled() {
        let mut widget = threshold_widget();
        widget.use_thresholds = false;
        let base = parse(&widget.colour, 1.0);
        let secondary = parse(&widget.secondary_colour, 1.0);
        assert_eq!(
            indicator_colour(&widget, base, secondary, 0.5),
            gradient(base, secondary, 0.5)
        );
    }

    #[test]
    fn graph_segments_use_their_historical_value_for_threshold_colour() {
        let widget = threshold_widget();
        let base = parse(&widget.colour, 1.0);
        let secondary = parse(&widget.secondary_colour, 1.0);
        assert_eq!(
            graph_colour(&widget, base, secondary, 69.0, 71.0, 0.5),
            parse(&widget.warning_colour, 1.0)
        );
        assert_eq!(
            graph_colour(&widget, base, secondary, 89.0, 91.0, 0.5),
            parse(&widget.critical_colour, 1.0)
        );
    }

    #[test]
    fn contained_gif_frame_preserves_transparent_padding() {
        let source = RgbaImage::from_pixel(8, 4, Rgba([255, 0, 0, 255]));
        let fitted = fit_gif_frame(&source, 8, 8, BackgroundMode::Contain);
        assert_eq!(fitted.get_pixel(0, 0)[3], 0);
        assert_eq!(fitted.get_pixel(4, 4), &Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn frequency_switches_automatically_between_mhz_and_ghz() {
        assert_eq!(frequency(Some(518.0)), "518 MHz");
        assert_eq!(frequency(Some(4850.0)), "4.85 GHz");
        assert_eq!(frequency(None), "--");
    }

    #[test]
    fn network_speed_converts_binary_kilobytes_to_decimal_bits() {
        for (raw, expected) in [
            (0.0, "0 Kbit/s"),
            (0.125, "1 Kbit/s"),
            (1.0, "8.2 Kbit/s"),
            (122.0, "999.4 Kbit/s"),
            (122.0703125, "1 Mbit/s"),
            (69580.078125, "570 Mbit/s"),
            (80000.0, "655.4 Mbit/s"),
            (122000.0, "999.4 Mbit/s"),
            (122070.3125, "1 Gbit/s"),
            (1250000.0, "10.2 Gbit/s"),
        ] {
            assert_eq!(network_speed(Some(raw)), expected, "raw KB/s: {raw}");
        }
        // Rounding must not leave a confusing 1000 Kbit/s or 1000 Mbit/s.
        assert_eq!(network_speed(Some(999.96 / 8.192)), "1 Mbit/s");
        assert_eq!(network_speed(Some(999960.0 / 8.192)), "1 Gbit/s");
        for value in [None, Some(-1.0), Some(f32::NAN), Some(f32::INFINITY)] {
            assert_eq!(network_speed(value), "--");
        }
    }

    #[test]
    fn network_labels_preserve_legacy_formats_and_allow_auto_units() {
        let sensors = SensorSnapshot {
            network_upload: Some(80000.0),
            network_download: Some(80000.0),
            ..Default::default()
        };
        let texts = HashMap::new();
        for kind in [WidgetKind::NetworkUpload, WidgetKind::NetworkDownload] {
            // The legacy layout needs no migration and keeps its explicit affixes.
            let mut widget = WidgetConfig::new(kind, 31, 288, "{value}");
            widget.left_text = "UP   ".into();
            widget.right_text = " B/s".into();
            let yaml = serde_yaml::to_string(&widget).unwrap();
            let restored: WidgetConfig = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(restored, widget);
            assert_eq!(label_text(&restored, &sensors, &texts), "UP   80000 B/s");
            widget.label_format = "{value:.0} KB/s".into();
            widget.right_text.clear();
            assert_eq!(label_text(&widget, &sensors, &texts), "UP   80000 KB/s");
            widget.label_format = "{speed} ({value} KB/s)".into();
            assert_eq!(
                label_text(&widget, &sensors, &texts),
                "UP   655.4 Mbit/s (80000 KB/s)"
            );
            widget.label_format = "{speed}".into();
            assert_eq!(
                label_text(&widget, &SensorSnapshot::default(), &texts),
                "UP   --"
            );
            assert_eq!(numeric(kind, &sensors), Some(80000.0));
            assert_eq!(max_for(kind), 10000.0);
        }
        let text = WidgetConfig::new(WidgetKind::Text, 0, 0, "Literal {speed}");
        assert_eq!(label_text(&text, &sensors, &texts), "Literal {speed}");
    }

    #[test]
    fn network_auto_text_invalidates_cache_for_fractional_raw_changes() {
        let texts = HashMap::new();
        for kind in [WidgetKind::NetworkUpload, WidgetKind::NetworkDownload] {
            let mut config = AppConfig::default();
            config.widgets = vec![WidgetConfig::new(kind, 0, 0, "{speed}")];
            let mut sensors = SensorSnapshot {
                network_upload: Some(0.1),
                network_download: Some(0.1),
                ..Default::default()
            };
            let first = super::super::visual_signature(&config, &sensors, &texts);
            sensors.network_upload = Some(0.2);
            sensors.network_download = Some(0.2);
            let second = super::super::visual_signature(&config, &sensors, &texts);
            assert_ne!(first, second, "0.8 and 1.6 Kbit/s must render differently");
            assert_eq!(
                label_text(&config.widgets[0], &sensors, &texts),
                "1.6 Kbit/s"
            );
        }
    }

    #[test]
    fn weather_codes_select_clear_rain_and_storm_icons() {
        assert_eq!(weather_icon(0), "☀");
        assert_eq!(weather_icon(63), "☔");
        assert_eq!(weather_icon(95), "⚡");
    }

    #[test]
    fn weather_icon_is_visually_centred_inside_its_widget_box() {
        let font = super::super::fonts::load("Segoe UI Symbol").unwrap();
        let mut widget = WidgetConfig::new(WidgetKind::WeatherIcon, 6, 6, "{value}");
        widget.width = 72;
        widget.height = 72;
        widget.font_size = 56.0;
        let mut layer = RgbaImage::new(84, 84);
        draw_centred_glyph(&mut layer, &widget, Rgba([255, 255, 255, 255]), &font, "☀");
        let (left, top, right, bottom) = visible_alpha_bounds(&layer).unwrap();
        let glyph_centre_x = (left + right) as i32;
        let glyph_centre_y = (top + bottom) as i32;
        let widget_centre_x = widget.x * 2 + widget.width as i32 - 1;
        let widget_centre_y = widget.y * 2 + widget.height as i32 - 1;
        assert!(
            (glyph_centre_x - widget_centre_x).abs() <= 1,
            "horizontal centre {glyph_centre_x}, expected {widget_centre_x}, bounds {left},{right}"
        );
        assert!(
            (glyph_centre_y - widget_centre_y).abs() <= 1,
            "vertical centre {glyph_centre_y}, expected {widget_centre_y}, bounds {top},{bottom}"
        );
    }
}
