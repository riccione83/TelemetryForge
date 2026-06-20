use crate::{
    app_state::{AppState, RenderWorker},
    config::{
        persistence,
        schema::{AppConfig, BackgroundMode, Orientation, WidgetConfig, WidgetKind},
    },
    display_driver, renderer,
    sensors::poller,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{Rgb, RgbImage};
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::State;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize)]
pub struct Status {
    running: bool,
    message: String,
}

#[tauri::command]
pub fn get_config(state: State<AppState>) -> AppConfig {
    state.config.read().clone()
}

#[tauri::command]
pub fn save_config(state: State<AppState>, config: AppConfig) -> Result<(), String> {
    persistence::save(&state.config_path, &config).map_err(|e| e.to_string())?;
    *state.config.write() = config;
    Ok(())
}

#[tauri::command]
pub fn list_screens(state: State<AppState>) -> Result<Vec<String>, String> {
    persistence::list_profiles(&state.config_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_screen(state: State<AppState>, name: String, config: AppConfig) -> Result<(), String> {
    let path = persistence::profile_path(&state.config_path, &name).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    persistence::save(&path, &config).map_err(|e| e.to_string())?;
    persistence::save(&state.config_path, &config).map_err(|e| e.to_string())?;
    *state.config.write() = config;
    Ok(())
}

#[tauri::command]
pub fn load_screen(state: State<AppState>, name: String) -> Result<AppConfig, String> {
    let path = persistence::profile_path(&state.config_path, &name).map_err(|e| e.to_string())?;
    let config = persistence::load_or_create(&path).map_err(|e| e.to_string())?;
    persistence::save(&state.config_path, &config).map_err(|e| e.to_string())?;
    *state.config.write() = config.clone();
    Ok(config)
}

#[tauri::command]
pub fn new_screen(state: State<AppState>, name: String) -> Result<AppConfig, String> {
    let current = state.config.read().clone();
    let mut config = AppConfig::default();
    config.display = current.display;
    config.libre_hardware_monitor_dll = current.libre_hardware_monitor_dll;
    config.widgets.clear();
    let path = persistence::profile_path(&state.config_path, &name).map_err(|e| e.to_string())?;
    if path.exists() {
        return Err("A screen with this name already exists".into());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    persistence::save(&path, &config).map_err(|e| e.to_string())?;
    persistence::save(&state.config_path, &config).map_err(|e| e.to_string())?;
    *state.config.write() = config.clone();
    Ok(config)
}

#[tauri::command]
pub fn delete_screen(state: State<AppState>, name: String) -> Result<(), String> {
    let path = persistence::profile_path(&state.config_path, &name).map_err(|e| e.to_string())?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn load_neon_sample(state: State<AppState>) -> Result<AppConfig, String> {
    let mut config = state.config.read().clone();
    let background = state
        .config_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("samples")
        .join("neon-telemetry.png");
    if !background.exists() {
        if let Some(parent) = background.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(
            &background,
            include_bytes!("../../samples/neon-telemetry.png"),
        )
        .map_err(|e| format!("Could not extract the sample: {e}"))?;
    }

    config.display.width = 480;
    config.display.height = 320;
    config.display.orientation = Orientation::Landscape;
    config.background.path = Some(background.to_string_lossy().into_owned());
    config.background.source = crate::config::schema::BackgroundSource::File;
    config.background.mode = BackgroundMode::Stretch;
    config.background.colour = "#020407".into();
    config.theme.name = "Neon Telemetry".into();
    config.theme.foreground = "#eaffff".into();
    config.theme.accent = "#3fffe5".into();
    config.widgets = vec![
        WidgetConfig::styled(WidgetKind::CpuUsage, 39, 65, 20.0, "#eaffff", "{value}%"),
        WidgetConfig::styled(
            WidgetKind::CpuTemperature,
            112,
            38,
            15.0,
            "#4dffe8",
            "CPU {value}C",
        ),
        WidgetConfig::styled(WidgetKind::GpuUsage, 280, 65, 20.0, "#fff1f1", "{value}%"),
        WidgetConfig::styled(
            WidgetKind::GpuTemperature,
            353,
            38,
            15.0,
            "#ff6b72",
            "GPU {value}C",
        ),
        WidgetConfig::styled(
            WidgetKind::NetworkUpload,
            76,
            146,
            14.0,
            "#55fff0",
            "UP {value}",
        ),
        WidgetConfig::styled(
            WidgetKind::NetworkDownload,
            300,
            146,
            14.0,
            "#ff6f78",
            "DOWN {value}",
        ),
        WidgetConfig::styled(WidgetKind::RamUsage, 40, 247, 19.0, "#eaffff", "{value}%"),
        WidgetConfig::styled(WidgetKind::Clock, 213, 276, 15.0, "#ffffff", "{value}"),
        WidgetConfig::styled(WidgetKind::DiskUsage, 357, 247, 18.0, "#70cfff", "{value}%"),
    ];

    persistence::save(&state.config_path, &config).map_err(|e| e.to_string())?;
    *state.config.write() = config.clone();
    Ok(config)
}

#[tauri::command]
pub fn get_preview(state: State<AppState>) -> Result<String, String> {
    let config = state.config.read().clone();
    let sensors = state.sensors.read().clone();
    let image = renderer::render(&config, &sensors).map_err(|e| e.to_string())?;
    let png = renderer::canvas::png_bytes(&image).map_err(|e| e.to_string())?;
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

#[tauri::command]
pub fn preview_config(state: State<AppState>, config: AppConfig) -> Result<String, String> {
    let sensors = state.sensors.read().clone();
    let image = renderer::render(&config, &sensors).map_err(|e| e.to_string())?;
    let png = renderer::canvas::png_bytes(&image).map_err(|e| e.to_string())?;
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

#[tauri::command]
pub fn test_sensors(state: State<AppState>) -> crate::sensors::model::SensorSnapshot {
    let config = state.config.read().clone();
    let snapshot = poller::read_snapshot(
        config.libre_hardware_monitor_dll.as_deref(),
        config.cpu_temperature_source,
    );
    *state.sensors.write() = snapshot.clone();
    snapshot
}

#[tauri::command]
pub async fn select_background(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let file = app
        .dialog()
        .file()
        .add_filter("Immagini", &["png", "jpg", "jpeg", "bmp", "gif"])
        .blocking_pick_file();
    Ok(file.and_then(|path| path.as_path().map(|p| p.to_string_lossy().into_owned())))
}

#[tauri::command]
pub async fn select_background_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let folder = app.dialog().file().blocking_pick_folder();
    Ok(folder.and_then(|path| path.as_path().map(|p| p.to_string_lossy().into_owned())))
}

#[tauri::command]
pub fn list_displays() -> Result<Vec<display_driver::detection::DisplayPort>, String> {
    display_driver::detection::list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn render_once(state: State<AppState>) -> Result<(), String> {
    let config = state.config.read().clone();
    let snapshot = poller::read_snapshot(
        config.libre_hardware_monitor_dll.as_deref(),
        config.cpu_temperature_source,
    );
    let image = renderer::render(&config, &snapshot).map_err(|e| e.to_string())?;
    display_driver::send_frame(&config.display, &image).map_err(|e| e.to_string())?;
    *state.sensors.write() = snapshot;
    *state.status.write() = "Frame sent".into();
    Ok(())
}

#[tauri::command]
pub fn test_display(state: State<AppState>) -> Result<String, String> {
    let config = state.config.read().clone();
    let port = display_driver::detection::resolve_port(&config.display.port)
        .map_err(|e| format!("Port detection: {e:#}"))?;
    let mut image = RgbImage::new(config.display.width, config.display.height);
    let half_width = image.width() / 2;
    let half_height = image.height() / 2;
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        *pixel = match (x < half_width, y < half_height) {
            (true, true) => Rgb([255, 0, 0]),
            (false, true) => Rgb([0, 255, 0]),
            (true, false) => Rgb([0, 0, 255]),
            (false, false) => Rgb([255, 255, 255]),
        };
    }
    display_driver::send_frame(&config.display, &image)
        .map_err(|e| format!("Serial write to {port}: {e:#}"))?;
    let message = format!("RGB test successfully sent to {port}");
    *state.status.write() = message.clone();
    Ok(message)
}

#[tauri::command]
pub fn set_display_brightness(state: State<AppState>, brightness: u8) -> Result<(), String> {
    let mut config = state.config.write();
    config.display.brightness = brightness.min(100);
    persistence::save(&state.config_path, &config).map_err(|e| e.to_string())?;

    // A running renderer owns the serial connection and applies the change
    // inside its loop. When stopped, open a short session and apply it now.
    if state.worker.lock().is_none() {
        display_driver::apply_brightness(&config.display).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn start_rendering(state: State<AppState>) -> Result<(), String> {
    let mut worker = state.worker.lock();
    if worker.is_some() {
        return Ok(());
    }
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = stop.clone();
    let config = state.config.clone();
    let sensors = state.sensors.clone();
    let status = state.status.clone();
    thread::Builder::new()
        .name("turzx-renderer".into())
        .spawn(move || {
            let initial = config.read().clone();
            let mut session = display_driver::DisplaySession::connect(&initial.display).ok();
            let mut target = poller::read_snapshot(
                initial.libre_hardware_monitor_dll.as_deref(),
                initial.cpu_temperature_source,
            );
            let mut displayed = target.clone();
            let mut previous_frame: Option<RgbImage> = None;
            let mut last_sensor_poll = Instant::now();
            let mut applied_brightness = initial.display.brightness;
            while !thread_stop.load(Ordering::Relaxed) {
                let current = config.read().clone();
                if last_sensor_poll.elapsed()
                    >= Duration::from_millis(current.sensor_poll_ms.max(250))
                {
                    let mut next = poller::read_snapshot(
                        current.libre_hardware_monitor_dll.as_deref(),
                        current.cpu_temperature_source,
                    );
                    update_histories(&mut next, &target, 120);
                    target = next;
                    last_sensor_poll = Instant::now();
                }
                interpolate_snapshot(&mut displayed, &target, 0.22);
                *sensors.write() = displayed.clone();
                let result = renderer::render(&current, &displayed).and_then(|image| {
                    if session.is_none() {
                        session = Some(display_driver::DisplaySession::connect(&current.display)?);
                        applied_brightness = current.display.brightness;
                    }
                    if current.display.brightness != applied_brightness {
                        session
                            .as_mut()
                            .expect("session initialized")
                            .set_brightness(current.display.brightness)?;
                        applied_brightness = current.display.brightness;
                    }
                    let send_result = send_changed_region(
                        session.as_mut().expect("session initialized"),
                        previous_frame.as_ref(),
                        &image,
                    );
                    if send_result.is_err() {
                        session = None;
                    } else {
                        previous_frame = Some(image);
                    }
                    send_result
                });
                *status.write() = match result {
                    Ok(()) => "Rendering active".into(),
                    Err(error) => format!("Error: {error:#}"),
                };
                thread::sleep(Duration::from_millis(
                    current.frame_interval_ms.clamp(50, 1000),
                ));
            }
            *status.write() = "Stopped".into();
        })
        .map_err(|e| e.to_string())?;
    *worker = Some(RenderWorker { stop });
    Ok(())
}

fn interpolate_snapshot(
    current: &mut crate::sensors::model::SensorSnapshot,
    target: &crate::sensors::model::SensorSnapshot,
    amount: f32,
) {
    fn blend(current: &mut Option<f32>, target: Option<f32>, amount: f32) {
        if let Some(target) = target {
            *current = Some(match *current {
                Some(value) => value + (target - value) * amount,
                None => target,
            });
        }
    }
    blend(&mut current.cpu_temperature, target.cpu_temperature, amount);
    blend(
        &mut current.cpu_temperature_core,
        target.cpu_temperature_core,
        amount,
    );
    blend(
        &mut current.cpu_temperature_socket,
        target.cpu_temperature_socket,
        amount,
    );
    blend(&mut current.cpu_usage, target.cpu_usage, amount);
    blend(&mut current.gpu_temperature, target.gpu_temperature, amount);
    blend(&mut current.gpu_usage, target.gpu_usage, amount);
    blend(&mut current.gpu_clock, target.gpu_clock, amount);
    blend(&mut current.ram_usage, target.ram_usage, amount);
    blend(&mut current.vram_usage, target.vram_usage, amount);
    blend(&mut current.disk_usage, target.disk_usage, amount);
    blend(&mut current.network_upload, target.network_upload, amount);
    blend(
        &mut current.network_download,
        target.network_download,
        amount,
    );
    blend(&mut current.fan_speed, target.fan_speed, amount);
    current.history_cpu = target.history_cpu.clone();
    current.history_gpu = target.history_gpu.clone();
    current.history_network_upload = target.history_network_upload.clone();
    current.history_network_download = target.history_network_download.clone();
}

fn update_histories(
    next: &mut crate::sensors::model::SensorSnapshot,
    previous: &crate::sensors::model::SensorSnapshot,
    limit: usize,
) {
    fn append(mut history: Vec<f32>, value: Option<f32>, limit: usize) -> Vec<f32> {
        if let Some(value) = value {
            history.push(value);
        }
        if history.len() > limit {
            history.drain(..history.len() - limit);
        }
        history
    }
    next.history_cpu = append(previous.history_cpu.clone(), next.cpu_usage, limit);
    next.history_gpu = append(previous.history_gpu.clone(), next.gpu_usage, limit);
    next.history_network_upload = append(
        previous.history_network_upload.clone(),
        next.network_upload,
        limit,
    );
    next.history_network_download = append(
        previous.history_network_download.clone(),
        next.network_download,
        limit,
    );
}

fn send_changed_region(
    session: &mut display_driver::DisplaySession,
    previous: Option<&RgbImage>,
    current: &RgbImage,
) -> anyhow::Result<()> {
    let Some(previous) = previous.filter(|image| image.dimensions() == current.dimensions()) else {
        return session.send_region(current, 0, 0);
    };
    let mut min_x = current.width();
    let mut min_y = current.height();
    let mut max_x = 0;
    let mut max_y = 0;
    let mut changed = false;
    for y in 0..current.height() {
        for x in 0..current.width() {
            if current.get_pixel(x, y) != previous.get_pixel(x, y) {
                changed = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if !changed {
        return Ok(());
    }
    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    if width * height > current.width() * current.height() * 3 / 4 {
        session.send_region(current, 0, 0)
    } else {
        let region = image::imageops::crop_imm(current, min_x, min_y, width, height).to_image();
        session.send_region(&region, min_x as u16, min_y as u16)
    }
}

#[tauri::command]
pub fn stop_rendering(state: State<AppState>) {
    if let Some(worker) = state.worker.lock().take() {
        worker.stop.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn get_status(state: State<AppState>) -> Status {
    Status {
        running: state.worker.lock().is_some(),
        message: state.status.read().clone(),
    }
}

#[tauri::command]
pub fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|e| e.to_string())
}
