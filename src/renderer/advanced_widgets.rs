use crate::{
    config::schema::{AppConfig, BackgroundMode, WidgetConfig, WidgetKind, WidgetRenderMode},
    sensors::model::SensorSnapshot,
};
use anyhow::{Context, Result};
use chrono::Local;
use image::{codecs::gif::GifDecoder, imageops, AnimationDecoder, RgbImage, Rgba, RgbaImage};
use imageproc::{
    drawing::{draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut},
    rect::Rect,
};
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    sync::{LazyLock, Mutex},
    time::{Instant, UNIX_EPOCH},
};

static GIF_CACHE: LazyLock<Mutex<HashMap<String, Vec<RgbaImage>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static ANIMATION_START: LazyLock<Instant> = LazyLock::new(Instant::now);

pub fn draw_all(image: &mut RgbImage, config: &AppConfig, sensors: &SensorSnapshot) -> Result<()> {
    for widget in config.widgets.iter().filter(|w| w.enabled) {
        if widget.kind == WidgetKind::Gif {
            draw_gif(image, widget)?;
            continue;
        }
        if widget.kind == WidgetKind::SuperWidget {
            draw_superwidget(image, widget, sensors)?;
            continue;
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
                let value = shown(widget.kind, sensors);
                let middle = widget
                    .label_format
                    .replace("{value:.0}", &value)
                    .replace("{value}", &value);
                let text = format!("{}{}{}", widget.left_text, middle, widget.right_text);
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
                history(widget.kind, sensors),
                primary,
                secondary,
            ),
        }
        let origin_x = widget.x - padding as i32;
        let origin_y = widget.y - padding as i32;
        if widget.shadow > 0 {
            composite(
                image,
                &tint(
                    &imageops::blur(&layer, widget.shadow as f32 / 2.0),
                    Rgba([0, 0, 0, 160]),
                ),
                origin_x + 3,
                origin_y + 3,
            );
        }
        if widget.glow > 0 {
            composite(
                image,
                &imageops::blur(&layer, widget.glow as f32),
                origin_x,
                origin_y,
            );
        }
        composite(image, &layer, origin_x, origin_y);
    }
    Ok(())
}

fn draw_superwidget(
    image: &mut RgbImage,
    widget: &WidgetConfig,
    sensors: &SensorSnapshot,
) -> Result<()> {
    let Some(id) = widget.superwidget_id.as_deref() else {
        return Ok(());
    };
    let Some(manifest) = crate::superwidgets::find(id) else {
        return Ok(());
    };
    let rendered = crate::superwidgets::renderer::draw(
        &manifest,
        widget.width,
        widget.height,
        sensors,
        &widget.superwidget_background_colour,
        widget.superwidget_background_opacity,
        &widget.superwidget_bindings,
    )?;
    composite_rgba(
        image,
        &rendered,
        widget.x,
        widget.y,
        widget.opacity.clamp(0.0, 1.0),
    );
    Ok(())
}

fn draw_gif(image: &mut RgbImage, widget: &WidgetConfig) -> Result<()> {
    let Some(path) = widget.gif_path.as_deref().filter(|path| !path.is_empty()) else {
        return Ok(());
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
            let decoder = GifDecoder::new(BufReader::new(
                File::open(path).with_context(|| format!("Could not open GIF {path}"))?,
            ))
            .with_context(|| format!("Could not decode GIF {path}"))?;
            let decoded = decoder
                .into_frames()
                .collect_frames()
                .with_context(|| format!("Could not read GIF frames from {path}"))?
                .into_iter()
                .map(|frame| frame.into_buffer())
                .collect::<Vec<_>>();
            cache.retain(|cached_key, _| !cached_key.starts_with(&format!("{path}:")));
            cache.insert(key.clone(), decoded);
        }
        cache.get(&key).cloned().unwrap_or_default()
    };
    if frames.is_empty() || widget.width == 0 || widget.height == 0 {
        return Ok(());
    }
    let elapsed_ms = ANIMATION_START.elapsed().as_millis();
    let frame_duration = (1000 / widget.gif_fps.clamp(1, 30) as u128).max(1);
    let raw_index = (elapsed_ms / frame_duration) as usize;
    let index = if widget.gif_loop {
        raw_index % frames.len()
    } else {
        raw_index.min(frames.len() - 1)
    };
    let fitted = fit_gif_frame(&frames[index], widget.width, widget.height, widget.gif_fit);
    composite_rgba(
        image,
        &fitted,
        widget.x,
        widget.y,
        widget.opacity.clamp(0.0, 1.0),
    );
    Ok(())
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
            let x = width.saturating_sub(source.width()) / 2;
            let y = height.saturating_sub(source.height()) / 2;
            imageops::overlay(&mut canvas, source, x as i64, y as i64);
            canvas
        }
    }
}

fn composite_rgba(dst: &mut RgbImage, src: &RgbaImage, ox: i32, oy: i32, opacity: f32) {
    for (x, y, pixel) in src.enumerate_pixels() {
        let tx = ox + x as i32;
        let ty = oy + y as i32;
        if tx < 0 || ty < 0 || tx >= dst.width() as i32 || ty >= dst.height() as i32 {
            continue;
        }
        let alpha = pixel[3] as f32 / 255.0 * opacity;
        if alpha <= 0.0 {
            continue;
        }
        let target = dst.get_pixel_mut(tx as u32, ty as u32);
        for channel in 0..3 {
            target[channel] =
                (target[channel] as f32 * (1.0 - alpha) + pixel[channel] as f32 * alpha) as u8;
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
        WidgetKind::Gif => None,
        WidgetKind::SuperWidget => None,
        _ => None,
    }
}
fn shown(k: WidgetKind, s: &SensorSnapshot) -> String {
    match k {
        WidgetKind::Clock => Local::now().format("%H:%M").to_string(),
        WidgetKind::Date => Local::now().format("%d/%m/%Y").to_string(),
        WidgetKind::CpuClock => frequency(s.cpu_clock),
        WidgetKind::GpuClock => frequency(s.gpu_clock),
        WidgetKind::Text => String::new(),
        WidgetKind::Gif => String::new(),
        WidgetKind::SuperWidget => String::new(),
        WidgetKind::Fps => "--".into(),
        WidgetKind::WeatherCondition => s.weather_condition.clone().unwrap_or_else(|| "--".into()),
        WidgetKind::WeatherIcon => s.weather_code.map(weather_icon).unwrap_or("--").into(),
        _ => numeric(k, s)
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
    match k {
        WidgetKind::CpuUsage | WidgetKind::CpuTemperature | WidgetKind::CpuClock => &s.history_cpu,
        WidgetKind::GpuUsage | WidgetKind::GpuTemperature | WidgetKind::GpuClock => &s.history_gpu,
        WidgetKind::GpuPower => &s.history_gpu_power,
        WidgetKind::NetworkUpload => &s.history_network_upload,
        WidgetKind::NetworkDownload => &s.history_network_download,
        WidgetKind::Volume => &s.history_volume,
        _ => &[],
    }
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
