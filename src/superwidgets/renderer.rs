use crate::{renderer::fonts, sensors::model::SensorSnapshot, superwidgets::Manifest};
use anyhow::Result;
use image::{imageops, Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_hollow_circle_mut, draw_line_segment_mut, draw_text_mut,
};

const DESIGN_SIZE: u32 = 240;

pub fn draw(
    manifest: &Manifest,
    width: u32,
    height: u32,
    sensors: &SensorSnapshot,
    background_colour: &str,
    background_opacity: f32,
    bindings: &std::collections::HashMap<String, String>,
) -> Result<RgbaImage> {
    if manifest.runtime == "wasm" {
        let entry = manifest.entry.as_deref().unwrap_or("widget.wasm");
        return super::wasm_runtime::render(
            &super::component_dir(&manifest.id).join(entry),
            width,
            height,
            sensors,
        );
    }
    let temperature = selected_temperature(manifest, sensors, bindings);
    let fan_speed = selected_fan(sensors, bindings);
    let source = match manifest.template.as_str() {
        "cpu_command_dial" => cpu_command_dial(sensors, temperature, fan_speed)?,
        "gpu_command_dial" => gpu_command_dial(sensors, temperature, fan_speed)?,
        _ => RgbaImage::new(DESIGN_SIZE, DESIGN_SIZE),
    };
    let rendered = imageops::resize(
        &source,
        width.max(1),
        height.max(1),
        imageops::FilterType::Lanczos3,
    );
    if background_opacity <= 0.0 {
        return Ok(rendered);
    }
    let rgb = crate::renderer::canvas::parse_colour(background_colour);
    let mut result = RgbaImage::from_pixel(
        width.max(1),
        height.max(1),
        Rgba([
            rgb[0],
            rgb[1],
            rgb[2],
            (background_opacity.clamp(0.0, 1.0) * 255.0) as u8,
        ]),
    );
    imageops::overlay(&mut result, &rendered, 0, 0);
    Ok(result)
}

fn gpu_command_dial(
    sensors: &SensorSnapshot,
    temperature: f32,
    fan_speed: f32,
) -> Result<RgbaImage> {
    render_command_dial(
        sensors.gpu_clock,
        Some(temperature),
        sensors.gpu_usage,
        Some(fan_speed),
        "G P U   T E M P E R A T U R E",
        Rgba([178, 61, 255, 255]),
        Rgba([255, 72, 215, 255]),
        Rgba([62, 20, 88, 240]),
        "FAN SPEED",
    )
}

fn cpu_command_dial(
    sensors: &SensorSnapshot,
    temperature: f32,
    fan_speed: f32,
) -> Result<RgbaImage> {
    render_command_dial(
        sensors.cpu_clock,
        Some(temperature),
        sensors.cpu_usage,
        Some(fan_speed),
        "C P U   T E M P E R A T U R E",
        Rgba([31, 153, 255, 255]),
        Rgba([101, 196, 255, 255]),
        Rgba([8, 39, 67, 240]),
        "PUMP SPEED",
    )
}

fn selected_temperature(
    manifest: &Manifest,
    sensors: &SensorSnapshot,
    bindings: &std::collections::HashMap<String, String>,
) -> f32 {
    if manifest.template == "cpu_command_dial" {
        match bindings.get("temperature").map(String::as_str) {
            Some("cpu_socket") => sensors
                .cpu_temperature_socket
                .or(sensors.cpu_temperature_core),
            Some("cpu_core") => sensors
                .cpu_temperature_core
                .or(sensors.cpu_temperature_socket),
            _ => sensors.cpu_temperature,
        }
        .unwrap_or_default()
    } else {
        sensors.gpu_temperature.unwrap_or_default()
    }
}

fn selected_fan(
    sensors: &SensorSnapshot,
    bindings: &std::collections::HashMap<String, String>,
) -> f32 {
    bindings
        .get("fan")
        .filter(|id| !id.is_empty())
        .and_then(|id| sensors.fan_sensors.iter().find(|sensor| &sensor.id == id))
        .map(|sensor| sensor.value)
        .or(sensors.fan_speed)
        .unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
fn render_command_dial(
    clock: Option<f32>,
    temperature: Option<f32>,
    usage: Option<f32>,
    fan_speed: Option<f32>,
    title: &str,
    active_colour: Rgba<u8>,
    pale_colour: Rgba<u8>,
    dark_colour: Rgba<u8>,
    fan_label: &str,
) -> Result<RgbaImage> {
    let mut image = RgbaImage::new(DESIGN_SIZE, DESIGN_SIZE);
    let centre = (120.0_f32, 120.0_f32);
    let white = Rgba([245, 250, 255, 255]);
    let muted = Rgba([130, 153, 174, 255]);

    draw_filled_circle_mut(&mut image, (120, 120), 113, Rgba([2, 4, 8, 245]));
    draw_hollow_circle_mut(&mut image, (120, 120), 111, Rgba([47, 122, 177, 255]));
    draw_hollow_circle_mut(&mut image, (120, 120), 96, dark_colour);

    for tick in 0..120 {
        let angle = tick as f32 / 120.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let major = tick % 10 == 0;
        let outer = 106.0;
        let inner = if major { 95.0 } else { 99.0 };
        draw_line_segment_mut(
            &mut image,
            polar(centre, inner, angle),
            polar(centre, outer, angle),
            if major { white } else { pale_colour },
        );
    }

    let usage = usage.unwrap_or_default().clamp(0.0, 100.0);
    arc(
        &mut image,
        centre,
        89.0,
        5.0,
        -220.0,
        260.0,
        1.0,
        Rgba([20, 65, 96, 255]),
        Rgba([20, 65, 96, 255]),
    );

    // The scale remains static while the cyan indicator moves with CPU load.
    gauge_indicator(
        &mut image,
        centre,
        89.0,
        value_angle(usage, 100.0, -220.0, 260.0),
        active_colour,
    );
    // Fixed reference markers inspired by the photographed dial.
    pointer(&mut image, centre, 89.0, -30.0, Rgba([255, 39, 51, 255]));
    pointer(&mut image, centre, 89.0, 88.0, white);

    let font = fonts::load("Bahnschrift")?;
    draw_centred(&mut image, &font, 11.0, 120, 37, title, pale_colour);

    let clock = clock.unwrap_or_default();
    let (clock_text, unit) = if clock >= 1000.0 {
        (format!("{:.2}", clock / 1000.0), "GHz")
    } else {
        (format!("{clock:.0}"), "MHz")
    };
    draw_centred(&mut image, &font, 37.0, 120, 55, &clock_text, white);
    draw_centred(&mut image, &font, 13.0, 120, 94, unit, white);

    temperature_gauge(
        &mut image,
        &font,
        (73, 126),
        temperature.unwrap_or_default(),
        active_colour,
    );
    fan_gauge(
        &mut image,
        &font,
        (168, 126),
        fan_speed.unwrap_or_default(),
        dark_colour,
        pale_colour,
        fan_label,
    );

    draw_centred(
        &mut image,
        &font,
        35.0,
        120,
        154,
        &format!("{usage:.0}%"),
        white,
    );
    draw_centred(
        &mut image,
        &font,
        9.0,
        120,
        194,
        "U T I L I Z A T I O N",
        muted,
    );
    Ok(image)
}

fn temperature_gauge(
    image: &mut RgbaImage,
    font: &ab_glyph::FontArc,
    centre: (i32, i32),
    temperature: f32,
    indicator_colour: Rgba<u8>,
) {
    draw_filled_circle_mut(image, centre, 33, Rgba([1, 5, 8, 250]));
    temperature_scale(
        image,
        (centre.0 as f32, centre.1 as f32),
        31.0,
        5.0,
        -220.0,
        260.0,
    );
    gauge_indicator(
        image,
        (centre.0 as f32, centre.1 as f32),
        31.0,
        value_angle(temperature, 100.0, -220.0, 260.0),
        indicator_colour,
    );
    draw_centred(
        image,
        font,
        27.0,
        centre.0,
        centre.1 - 18,
        &format!("{temperature:.0}"),
        Rgba([255, 255, 255, 255]),
    );
    draw_centred(
        image,
        font,
        12.0,
        centre.0,
        centre.1 + 13,
        "°C",
        Rgba([255, 255, 255, 255]),
    );
}

fn fan_gauge(
    image: &mut RgbaImage,
    font: &ab_glyph::FontArc,
    centre: (i32, i32),
    rpm: f32,
    background: Rgba<u8>,
    indicator_colour: Rgba<u8>,
    label: &str,
) {
    draw_filled_circle_mut(image, centre, 33, background);
    arc(
        image,
        (centre.0 as f32, centre.1 as f32),
        31.0,
        4.0,
        -220.0,
        260.0,
        1.0,
        Rgba([63, 126, 165, 255]),
        Rgba([63, 126, 165, 255]),
    );
    gauge_indicator(
        image,
        (centre.0 as f32, centre.1 as f32),
        31.0,
        value_angle(rpm, 3000.0, -220.0, 260.0),
        indicator_colour,
    );
    draw_centred(
        image,
        font,
        18.0,
        centre.0,
        centre.1 - 19,
        &format!("{rpm:.0}"),
        Rgba([255, 255, 255, 255]),
    );
    draw_centred(
        image,
        font,
        8.0,
        centre.0,
        centre.1 + 1,
        label,
        Rgba([230, 244, 255, 255]),
    );
    fan_icon(image, centre.0, centre.1 + 16);
}

fn fan_icon(image: &mut RgbaImage, cx: i32, cy: i32) {
    draw_filled_circle_mut(image, (cx, cy), 3, Rgba([230, 240, 247, 255]));
    for blade in 0..4 {
        let angle = blade as f32 * std::f32::consts::FRAC_PI_2;
        let p1 = polar((cx as f32, cy as f32), 4.0, angle);
        let p2 = polar((cx as f32, cy as f32), 12.0, angle + 0.32);
        let p3 = polar((cx as f32, cy as f32), 8.0, angle + 0.8);
        draw_line_segment_mut(image, p1, p2, Rgba([225, 236, 244, 255]));
        draw_line_segment_mut(image, p2, p3, Rgba([225, 236, 244, 255]));
        draw_line_segment_mut(image, p3, p1, Rgba([225, 236, 244, 255]));
    }
}

fn temperature_scale(
    image: &mut RgbaImage,
    centre: (f32, f32),
    radius: f32,
    thickness: f32,
    start: f32,
    sweep: f32,
) {
    let segments = [
        (0.0, 0.55, Rgba([36, 231, 84, 255])),
        (0.55, 0.78, Rgba([255, 220, 44, 255])),
        (0.78, 1.0, Rgba([255, 55, 55, 255])),
    ];
    for (from, to, colour) in segments {
        arc(
            image,
            centre,
            radius,
            thickness,
            start + sweep * from,
            sweep * (to - from),
            1.0,
            colour,
            colour,
        );
    }
}

fn arc(
    image: &mut RgbaImage,
    centre: (f32, f32),
    radius: f32,
    thickness: f32,
    start_degrees: f32,
    sweep_degrees: f32,
    progress: f32,
    active: Rgba<u8>,
    inactive: Rgba<u8>,
) {
    let steps = (radius * sweep_degrees.to_radians()).max(24.0) as usize;
    for step in 0..=steps {
        let fraction = step as f32 / steps as f32;
        let angle = (start_degrees + sweep_degrees * fraction).to_radians();
        let colour = if fraction <= progress {
            active
        } else {
            inactive
        };
        for offset in 0..thickness.max(1.0) as usize {
            let point = polar(centre, radius - offset as f32, angle);
            let x = point.0.round() as i32;
            let y = point.1.round() as i32;
            if x >= 0 && y >= 0 && x < image.width() as i32 && y < image.height() as i32 {
                image.put_pixel(x as u32, y as u32, colour);
            }
        }
    }
}

fn pointer(image: &mut RgbaImage, centre: (f32, f32), radius: f32, degrees: f32, colour: Rgba<u8>) {
    let angle = degrees.to_radians();
    let tip = polar(centre, radius + 2.0, angle);
    let left = polar(centre, radius - 7.0, angle - 0.06);
    let right = polar(centre, radius - 7.0, angle + 0.06);
    draw_line_segment_mut(image, left, tip, colour);
    draw_line_segment_mut(image, tip, right, colour);
    draw_line_segment_mut(image, right, left, colour);
}

fn value_angle(value: f32, maximum: f32, start: f32, sweep: f32) -> f32 {
    start + sweep * (value / maximum.max(1.0)).clamp(0.0, 1.0)
}

fn gauge_indicator(
    image: &mut RgbaImage,
    centre: (f32, f32),
    radius: f32,
    degrees: f32,
    colour: Rgba<u8>,
) {
    let angle = degrees.to_radians();
    let inner = polar(centre, radius - 12.0, angle);
    let outer = polar(centre, radius + 2.0, angle);
    let tangent = (-angle.sin(), angle.cos());
    for offset in -1..=1 {
        let shift = offset as f32;
        draw_line_segment_mut(
            image,
            (inner.0 + tangent.0 * shift, inner.1 + tangent.1 * shift),
            (outer.0 + tangent.0 * shift, outer.1 + tangent.1 * shift),
            colour,
        );
    }
    draw_filled_circle_mut(
        image,
        (outer.0.round() as i32, outer.1.round() as i32),
        3,
        colour,
    );
}

fn polar(centre: (f32, f32), radius: f32, angle: f32) -> (f32, f32) {
    (
        centre.0 + radius * angle.cos(),
        centre.1 + radius * angle.sin(),
    )
}

fn draw_centred(
    image: &mut RgbaImage,
    font: &ab_glyph::FontArc,
    scale: f32,
    centre_x: i32,
    y: i32,
    text: &str,
    colour: Rgba<u8>,
) {
    let width = text_width(font, scale, text);
    draw_text_mut(
        image,
        colour,
        centre_x - width as i32 / 2,
        y,
        scale,
        font,
        text,
    );
}

fn text_width(font: &ab_glyph::FontArc, scale: f32, text: &str) -> f32 {
    use ab_glyph::{Font, ScaleFont};
    let scaled = font.as_scaled(scale);
    text.chars()
        .map(|character| scaled.h_advance(scaled.glyph_id(character)))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_command_dial_renders_at_requested_size() {
        let manifest = Manifest {
            id: "cpu-command-dial".into(),
            name: "CPU Command Dial".into(),
            description: String::new(),
            template: "cpu_command_dial".into(),
            runtime: "native".into(),
            entry: None,
            width: 220,
            height: 220,
            sensors: Vec::new(),
            abi_version: 1,
            animated_fps: 0,
        };
        let sensors = SensorSnapshot {
            cpu_usage: Some(73.0),
            cpu_temperature: Some(65.0),
            cpu_clock: Some(4750.0),
            fan_speed: Some(1240.0),
            ..Default::default()
        };
        let image = draw(
            &manifest,
            180,
            180,
            &sensors,
            "#000000",
            0.0,
            &Default::default(),
        )
        .unwrap();
        assert_eq!(image.dimensions(), (180, 180));
        assert!(image.pixels().any(|pixel| pixel[3] > 0));
    }

    #[test]
    fn gpu_command_dial_renders_at_requested_size() {
        let manifest = Manifest {
            id: "gpu-command-dial".into(),
            name: "GPU Command Dial".into(),
            description: String::new(),
            template: "gpu_command_dial".into(),
            runtime: "native".into(),
            entry: None,
            width: 220,
            height: 220,
            sensors: Vec::new(),
            abi_version: 1,
            animated_fps: 0,
        };
        let sensors = SensorSnapshot {
            gpu_usage: Some(96.0),
            gpu_temperature: Some(68.0),
            gpu_clock: Some(2685.0),
            fan_speed: Some(2040.0),
            ..Default::default()
        };
        let image = draw(
            &manifest,
            180,
            180,
            &sensors,
            "#000000",
            0.0,
            &Default::default(),
        )
        .unwrap();
        assert_eq!(image.dimensions(), (180, 180));
        assert!(image.pixels().any(|pixel| pixel[3] > 0));
    }

    #[test]
    #[ignore = "regenerates the README screenshot"]
    fn generate_command_dials_readme_screenshot() {
        let mut image = RgbaImage::from_pixel(480, 320, Rgba([2, 5, 12, 255]));
        for y in 0..image.height() {
            for x in 0..image.width() {
                let cyan = ((x as f32 / image.width() as f32) * 12.0) as u8;
                let magenta = ((y as f32 / image.height() as f32) * 10.0) as u8;
                image.put_pixel(x, y, Rgba([2 + magenta, 5 + cyan / 2, 12 + cyan, 255]));
            }
        }
        let sensors = SensorSnapshot {
            cpu_usage: Some(68.0),
            cpu_temperature: Some(64.0),
            cpu_clock: Some(5175.0),
            gpu_usage: Some(91.0),
            gpu_temperature: Some(72.0),
            gpu_clock: Some(2760.0),
            fan_speed: Some(1840.0),
            ..Default::default()
        };
        let cpu = cpu_command_dial(&sensors, 64.0, 1420.0).unwrap();
        let gpu = gpu_command_dial(&sensors, 72.0, 1840.0).unwrap();
        let cpu = imageops::resize(&cpu, 220, 220, imageops::FilterType::Lanczos3);
        let gpu = imageops::resize(&gpu, 220, 220, imageops::FilterType::Lanczos3);
        imageops::overlay(&mut image, &cpu, 8, 48);
        imageops::overlay(&mut image, &gpu, 252, 48);

        let font = fonts::load("Bahnschrift").unwrap();
        draw_centred(
            &mut image,
            &font,
            18.0,
            240,
            13,
            "TELEMETRYFORGE  /  COMMAND DIALS",
            Rgba([220, 244, 255, 255]),
        );
        draw_centred(
            &mut image,
            &font,
            11.0,
            240,
            285,
            "RESIZABLE MULTI-SENSOR SUPER WIDGETS",
            Rgba([82, 203, 255, 255]),
        );

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/screenshots/superwidgets-command-dials.png");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image.save(path).unwrap();
    }
}
