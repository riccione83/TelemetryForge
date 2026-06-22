use telemetryforge_superwidget_sdk::{rgba, Canvas, Sensor};

const CYAN: u32 = rgba(33, 232, 255, 255);
const CYAN_SOFT: u32 = rgba(22, 116, 145, 255);
const MAGENTA: u32 = rgba(255, 50, 198, 255);
const MAGENTA_SOFT: u32 = rgba(120, 25, 100, 255);
const WHITE: u32 = rgba(238, 249, 255, 255);
const MUTED: u32 = rgba(85, 116, 140, 255);

#[no_mangle]
pub extern "C" fn tf_render(width: i32, height: i32) {
    let canvas = Canvas::new(width, height);
    let centre = (canvas.width / 2.0, canvas.height / 2.0);
    let radius = canvas.width.min(canvas.height) * 0.46;
    let scale = canvas.width.min(canvas.height) / 300.0;
    let time = canvas.animation_ms();
    let phase_ms = time % 1600;
    let triangle = if phase_ms > 800 {
        1600 - phase_ms
    } else {
        phase_ms
    };
    let pulse = triangle as f32 / 800.0;
    let rotation = time as f32 * 0.045;

    let cpu = canvas.sensor(Sensor::CpuUsage).clamp(0.0, 100.0);
    let gpu = canvas.sensor(Sensor::GpuUsage).clamp(0.0, 100.0);
    let cpu_temp = canvas.sensor(Sensor::CpuTemperature);
    let gpu_temp = canvas.sensor(Sensor::GpuTemperature);
    let gpu_power = canvas.sensor(Sensor::GpuPower);
    let combined = (cpu + gpu) * 0.5;

    canvas.clear(rgba(2, 5, 12, 246));

    canvas.circle(centre, radius, 2.0 * scale, CYAN_SOFT);
    canvas.circle(centre, radius - 8.0 * scale, 1.0 * scale, MAGENTA_SOFT);
    for tick in 0..72 {
        let colour = if tick % 3 == 0 {
            CYAN
        } else if tick % 3 == 1 {
            MAGENTA
        } else {
            MUTED
        };
        canvas.arc(
            centre,
            radius - 4.0 * scale,
            2.0 * scale,
            tick as f32 * 5.0 - 90.0,
            2.2,
            colour,
        );
    }

    canvas.arc(
        centre,
        radius - 18.0 * scale,
        7.0 * scale,
        -210.0,
        cpu * 2.4,
        CYAN,
    );
    canvas.arc(
        centre,
        radius - 29.0 * scale,
        7.0 * scale,
        30.0,
        gpu * 2.4,
        MAGENTA,
    );

    canvas.arc(
        centre,
        radius - 42.0 * scale,
        1.0 * scale,
        0.0,
        360.0,
        rgba(31, 55, 74, 180),
    );
    canvas.arc(
        centre,
        radius - 48.0 * scale,
        1.0 * scale,
        0.0,
        360.0,
        rgba(25, 48, 69, 180),
    );
    canvas.arc(
        centre,
        radius - 54.0 * scale,
        1.0 * scale,
        0.0,
        360.0,
        rgba(30, 42, 70, 180),
    );
    canvas.arc(
        centre,
        radius - 42.0 * scale,
        2.0 * scale,
        rotation,
        10.0,
        WHITE,
    );
    canvas.arc(
        centre,
        radius - 48.0 * scale,
        3.0 * scale,
        -rotation * 0.72,
        8.0,
        CYAN,
    );
    canvas.arc(
        centre,
        radius - 54.0 * scale,
        3.0 * scale,
        rotation * 1.31 + 160.0,
        7.0,
        MAGENTA,
    );

    canvas.fill_circle(centre, 49.0 * scale, rgba(8, 25, 55, 255));
    canvas.fill_circle(centre, 37.0 * scale, rgba(20, 90, 165, 255));
    canvas.fill_circle(centre, 23.0 * scale, rgba(100, 235, 255, 255));
    canvas.fill_circle(
        centre,
        (7.0 + pulse * 3.0) * scale,
        rgba(235, 253, 255, 255),
    );
    canvas.circle(centre, 54.0 * scale, 2.0 * scale, MAGENTA);
    canvas.line(
        (centre.0 - 50.0 * scale, centre.1 + 36.0 * scale),
        (centre.0 - 67.0 * scale, centre.1 + 53.0 * scale),
        2.0 * scale,
        CYAN_SOFT,
    );
    canvas.line(
        (centre.0 + 50.0 * scale, centre.1 + 36.0 * scale),
        (centre.0 + 67.0 * scale, centre.1 + 53.0 * scale),
        2.0 * scale,
        MAGENTA_SOFT,
    );

    canvas.text_center(
        (centre.0, centre.1 - 30.0 * scale),
        10.0 * scale,
        MUTED,
        "TOTAL LOAD",
    );
    canvas.number_center(
        (centre.0, centre.1 - 12.0 * scale),
        29.0 * scale,
        rgba(0, 0, 0, 255),
        combined,
        0,
    );
    canvas.text_center(
        (centre.0, centre.1 + 20.0 * scale),
        10.0 * scale,
        CYAN,
        "REACTOR CORE",
    );

    metric(
        &canvas,
        (centre.0 - 76.0 * scale, centre.1 + 68.0 * scale),
        "CPU",
        cpu,
        cpu_temp,
        CYAN,
        scale,
    );
    metric(
        &canvas,
        (centre.0 + 76.0 * scale, centre.1 + 68.0 * scale),
        "GPU",
        gpu,
        gpu_temp,
        MAGENTA,
        scale,
    );

    canvas.text_center(
        (centre.0, centre.1 + 108.0 * scale),
        8.0 * scale,
        MUTED,
        "GPU POWER",
    );
    canvas.number_center(
        (centre.0, centre.1 + 119.0 * scale),
        13.0 * scale,
        WHITE,
        gpu_power,
        0,
    );
}

fn metric(
    canvas: &Canvas,
    position: (f32, f32),
    label: &str,
    usage: f32,
    temperature: f32,
    colour: u32,
    scale: f32,
) {
    canvas.fill_circle(position, 28.0 * scale, rgba(3, 11, 24, 250));
    canvas.circle(position, 29.0 * scale, 1.0 * scale, MUTED);
    canvas.arc(
        position,
        26.0 * scale,
        3.0 * scale,
        -90.0,
        usage.clamp(0.0, 100.0) * 3.6,
        colour,
    );
    canvas.text_center(
        (position.0, position.1 - 19.0 * scale),
        8.0 * scale,
        colour,
        label,
    );
    canvas.number_center(
        (position.0, position.1 - 8.0 * scale),
        19.0 * scale,
        WHITE,
        usage,
        0,
    );
    canvas.number_center(
        (position.0, position.1 + 14.0 * scale),
        9.0 * scale,
        colour,
        temperature,
        0,
    );
}
