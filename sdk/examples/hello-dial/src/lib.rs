use telemetryforge_superwidget_sdk::{rgba, Canvas, Sensor};

#[no_mangle]
pub extern "C" fn tf_render(width: i32, height: i32) {
    let canvas = Canvas::new(width, height);
    let centre = (canvas.width / 2.0, canvas.height / 2.0);
    let radius = canvas.width.min(canvas.height) * 0.42;
    let usage = canvas.sensor(Sensor::CpuUsage).clamp(0.0, 100.0);
    let temperature = canvas.sensor(Sensor::CpuTemperature);

    canvas.clear(rgba(3, 8, 14, 220));
    canvas.circle(centre, radius, 2.0, rgba(40, 130, 200, 255));
    canvas.arc(
        centre,
        radius - 7.0,
        5.0,
        -220.0,
        260.0 * usage / 100.0,
        rgba(35, 210, 255, 255),
    );
    canvas.text(
        (centre.0 - radius * 0.45, centre.1 - 12.0),
        radius * 0.28,
        rgba(245, 250, 255, 255),
        &format!("{usage:.0}%"),
    );
    canvas.text(
        (centre.0 - radius * 0.45, centre.1 + 18.0),
        radius * 0.16,
        rgba(90, 220, 255, 255),
        &format!("{temperature:.0} C"),
    );
}
