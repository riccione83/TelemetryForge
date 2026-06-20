use crate::{
    config::schema::{AppConfig, WidgetConfig, WidgetKind, WidgetRenderMode},
    sensors::model::SensorSnapshot,
};
use anyhow::Result;
use chrono::Local;
use image::{imageops, RgbImage, Rgba, RgbaImage};
use imageproc::{
    drawing::{draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut},
    rect::Rect,
};

pub fn draw_all(image: &mut RgbImage, config: &AppConfig, sensors: &SensorSnapshot) -> Result<()> {
    for widget in config.widgets.iter().filter(|w| w.enabled) {
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
        let mut layer = RgbaImage::new(image.width(), image.height());
        match widget.render_mode {
            WidgetRenderMode::Text => {
                let font = super::fonts::load(&widget.font)?;
                let value = shown(widget.kind, sensors);
                let middle = widget
                    .label_format
                    .replace("{value:.0}", &value)
                    .replace("{value}", &value);
                draw_text_mut(
                    &mut layer,
                    primary,
                    widget.x,
                    widget.y,
                    widget.font_size,
                    &font,
                    &format!("{}{}{}", widget.left_text, middle, widget.right_text),
                );
            }
            WidgetRenderMode::Bar => bar(
                &mut layer,
                widget,
                ratio(widget.kind, value),
                primary,
                secondary,
            ),
            WidgetRenderMode::Circle => circle(
                &mut layer,
                widget,
                ratio(widget.kind, value),
                primary,
                secondary,
            ),
            WidgetRenderMode::Graph => graph(
                image,
                &mut layer,
                widget,
                history(widget.kind, sensors),
                primary,
                secondary,
            ),
        }
        if widget.shadow > 0 {
            composite(
                image,
                &tint(
                    &imageops::blur(&layer, widget.shadow as f32 / 2.0),
                    Rgba([0, 0, 0, 160]),
                ),
                3,
                3,
            );
        }
        if widget.glow > 0 {
            composite(image, &imageops::blur(&layer, widget.glow as f32), 0, 0);
        }
        composite(image, &layer, 0, 0);
    }
    Ok(())
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

fn graph(
    image: &mut RgbImage,
    layer: &mut RgbaImage,
    w: &WidgetConfig,
    values: &[f32],
    a: Rgba<u8>,
    b: Rgba<u8>,
) {
    if w.width < 2 || w.height < 2 {
        return;
    }
    if w.graph_background_opacity > 0.0 {
        let colour = parse(
            &w.graph_background_colour,
            w.graph_background_opacity.clamp(0.0, 1.0),
        );
        let alpha = colour[3] as f32 / 255.0;
        for y in w.y.max(0)..(w.y + w.height as i32).min(image.height() as i32) {
            for x in w.x.max(0)..(w.x + w.width as i32).min(image.width() as i32) {
                let pixel = image.get_pixel_mut(x as u32, y as u32);
                for channel in 0..3 {
                    pixel[channel] = (pixel[channel] as f32 * (1.0 - alpha)
                        + colour[channel] as f32 * alpha)
                        as u8;
                }
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
        _ => None,
    }
}
fn shown(k: WidgetKind, s: &SensorSnapshot) -> String {
    match k {
        WidgetKind::Clock => Local::now().format("%H:%M").to_string(),
        WidgetKind::Date => Local::now().format("%d/%m/%Y").to_string(),
        WidgetKind::Text => String::new(),
        WidgetKind::Fps => "--".into(),
        _ => numeric(k, s)
            .map(|v| format!("{v:.0}"))
            .unwrap_or_else(|| "--".into()),
    }
}
fn max_for(k: WidgetKind) -> f32 {
    match k {
        WidgetKind::CpuClock => 6000.0,
        WidgetKind::GpuPower => 600.0,
        WidgetKind::GpuClock | WidgetKind::FanSpeed => 3000.0,
        WidgetKind::NetworkUpload | WidgetKind::NetworkDownload => 10000.0,
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
}
