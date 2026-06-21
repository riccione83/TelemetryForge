use crate::{
    app_state::{AppState, RenderWorker},
    config::{persistence, schema::AppConfig},
    display_driver, package, renderer, scene,
    sensors::poller,
    windows_startup,
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
use tauri_plugin_dialog::DialogExt;

fn merge_screen_settings(mut screen: AppConfig, current: &AppConfig) -> AppConfig {
    screen.automation = current.automation.clone();
    screen.transition = current.transition.clone();
    screen.libre_hardware_monitor_dll = current.libre_hardware_monitor_dll.clone();
    screen.cpu_temperature_source = current.cpu_temperature_source;
    screen.cpu_clock_source = current.cpu_clock_source;
    screen.fan_sensor = current.fan_sensor.clone();
    screen
}

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
    let loaded = persistence::load_or_create(&path).map_err(|e| e.to_string())?;
    let config = merge_screen_settings(loaded, &state.config.read());
    persistence::save(&state.config_path, &config).map_err(|e| e.to_string())?;
    *state.config.write() = config.clone();
    state.scene_revision.fetch_add(1, Ordering::Relaxed);
    Ok(config)
}

#[tauri::command]
pub fn new_screen(state: State<AppState>, name: String) -> Result<AppConfig, String> {
    let current = state.config.read().clone();
    let mut config = AppConfig::default();
    config.display = current.display;
    config.libre_hardware_monitor_dll = current.libre_hardware_monitor_dll;
    config.cpu_clock_source = current.cpu_clock_source;
    config.fan_sensor = current.fan_sensor;
    config.automation = current.automation;
    config.transition = current.transition;
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
    state.scene_revision.fetch_add(1, Ordering::Relaxed);
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
pub async fn export_package(
    app: tauri::AppHandle,
    config: AppConfig,
) -> Result<Option<String>, String> {
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("TelemetryForge package", &["telemetryforge"])
        .set_file_name("screen.telemetryforge")
        .blocking_save_file()
        .and_then(|path| path.as_path().map(std::path::Path::to_path_buf))
    else {
        return Ok(None);
    };
    package::export(&path, &config).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn import_package(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<AppConfig>, String> {
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("TelemetryForge package", &["telemetryforge"])
        .blocking_pick_file()
        .and_then(|path| path.as_path().map(std::path::Path::to_path_buf))
    else {
        return Ok(None);
    };
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("imported");
    let config = package::import(&path, name).map_err(|error| error.to_string())?;
    persistence::save(&state.config_path, &config).map_err(|error| error.to_string())?;
    *state.config.write() = config.clone();
    state.scene_revision.fetch_add(1, Ordering::Relaxed);
    Ok(Some(config))
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
        config.cpu_clock_source,
        config.fan_sensor.as_deref(),
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
pub async fn select_gif(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let file = app
        .dialog()
        .file()
        .add_filter("Animated GIF", &["gif"])
        .blocking_pick_file();
    Ok(file.and_then(|path| path.as_path().map(|p| p.to_string_lossy().into_owned())))
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
        config.cpu_clock_source,
        config.fan_sensor.as_deref(),
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
    let scene_revision = state.scene_revision.clone();
    let config_path = state.config_path.clone();
    thread::Builder::new()
        .name("telemetryforge-renderer".into())
        .spawn(move || {
            let initial = config.read().clone();
            let mut session = display_driver::DisplaySession::connect(&initial.display).ok();
            let mut sensor_poller = poller::SensorPoller::new();
            thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
            let mut target = sensor_poller.read(
                initial.libre_hardware_monitor_dll.as_deref(),
                initial.cpu_temperature_source,
                initial.cpu_clock_source,
                initial.fan_sensor.as_deref(),
            );
            let mut displayed = target.clone();
            let mut previous_frame: Option<RgbImage> = None;
            let mut last_sensor_poll = Instant::now();
            let mut last_volume_poll = Instant::now() - Duration::from_millis(200);
            let mut applied_brightness = initial.display.brightness;
            let mut rendered_config: Option<AppConfig> = None;
            let mut rendered_dynamic_key = String::new();
            let mut rendered_visual_signature = 0;
            let mut observed_scene_revision = scene_revision.load(Ordering::Relaxed);
            let mut transition: Option<scene::Transition> = None;
            let mut rule_engine = scene::RuleEngine::new();
            let mut last_rule_check = Instant::now() - Duration::from_secs(2);
            let mut active_rule_screen: Option<String> = None;
            let mut active_rule_revision = observed_scene_revision;
            while !thread_stop.load(Ordering::Relaxed) {
                let mut current = config.read().clone();
                if volume_widget_enabled(&current)
                    && last_volume_poll.elapsed() >= Duration::from_millis(100)
                {
                    target.system_volume = sensor_poller.read_system_volume();
                    last_volume_poll = Instant::now();
                }
                if last_sensor_poll.elapsed()
                    >= Duration::from_millis(current.sensor_poll_ms.max(250))
                {
                    let mut next = sensor_poller.read(
                        current.libre_hardware_monitor_dll.as_deref(),
                        current.cpu_temperature_source,
                        current.cpu_clock_source,
                        current.fan_sensor.as_deref(),
                    );
                    update_histories(&mut next, &target, 120);
                    target = next;
                    last_sensor_poll = Instant::now();
                }
                interpolate_snapshot(&mut displayed, &target, 0.22);
                *sensors.write() = displayed.clone();
                if last_rule_check.elapsed() >= Duration::from_secs(1) {
                    let current_revision = scene_revision.load(Ordering::Relaxed);
                    if current_revision != active_rule_revision {
                        active_rule_screen = None;
                        active_rule_revision = current_revision;
                    }
                    let requested = rule_engine.target_screen(&current, &displayed);
                    if requested != active_rule_screen {
                        if let Some(name) = requested.as_deref() {
                            if let Ok(path) = persistence::profile_path(&config_path, name) {
                                match persistence::load_or_create(&path) {
                                    Ok(loaded) => {
                                        let switched = merge_screen_settings(loaded, &current);
                                        if let Err(error) = persistence::save(&config_path, &switched)
                                        {
                                            tracing::error!(%error, "could not persist automatic screen");
                                        } else {
                                            *config.write() = switched.clone();
                                            current = switched;
                                            active_rule_revision =
                                                scene_revision.fetch_add(1, Ordering::Relaxed) + 1;
                                            active_rule_screen = requested.clone();
                                            *status.write() =
                                                format!("Automatic screen: {name}");
                                        }
                                    }
                                    Err(error) => tracing::error!(
                                        %error,
                                        screen = name,
                                        "could not load automatic screen"
                                    ),
                                }
                            }
                        } else {
                            active_rule_screen = None;
                        }
                    }
                    last_rule_check = Instant::now();
                }
                let revision = scene_revision.load(Ordering::Relaxed);
                if revision != observed_scene_revision {
                    transition = previous_frame.clone().and_then(|frame| {
                        scene::Transition::new(
                            frame,
                            current.transition.kind,
                            current.transition.duration_ms,
                        )
                    });
                    observed_scene_revision = revision;
                }
                let dynamic_key = renderer_dynamic_key(&current);
                let visual_signature = renderer::visual_signature(&current, &displayed);
                let should_render = previous_frame.is_none()
                    || transition.is_some()
                    || rendered_config.as_ref() != Some(&current)
                    || rendered_dynamic_key != dynamic_key
                    || rendered_visual_signature != visual_signature;
                if !should_render {
                    thread::sleep(renderer_sleep_interval(&current, transition.is_some()));
                    continue;
                }
                let result = renderer::render(&current, &displayed).and_then(|target_image| {
                    let (image, transition_finished) = transition
                        .as_ref()
                        .map(|animation| animation.frame(&target_image))
                        .unwrap_or((target_image, true));
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
                        if transition_finished {
                            transition = None;
                            rendered_config = Some(current.clone());
                            rendered_dynamic_key = dynamic_key.clone();
                            rendered_visual_signature = visual_signature;
                        }
                    }
                    send_result
                });
                *status.write() = match result {
                    Ok(()) => "Rendering active".into(),
                    Err(error) => format!("Error: {error:#}"),
                };
                thread::sleep(renderer_sleep_interval(&current, transition.is_some()));
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
) -> bool {
    fn blend(current: &mut Option<f32>, target: Option<f32>, amount: f32) -> bool {
        if let Some(target) = target {
            let next = match *current {
                Some(value) if (target - value).abs() > 0.05 => value + (target - value) * amount,
                _ => target,
            };
            let changed = current.is_none_or(|value| (next - value).abs() > f32::EPSILON);
            *current = Some(next);
            changed
        } else {
            false
        }
    }
    let mut changed = blend(&mut current.cpu_temperature, target.cpu_temperature, amount);
    changed |= blend(
        &mut current.cpu_temperature_core,
        target.cpu_temperature_core,
        amount,
    );
    changed |= blend(
        &mut current.cpu_temperature_socket,
        target.cpu_temperature_socket,
        amount,
    );
    changed |= blend(&mut current.cpu_usage, target.cpu_usage, amount);
    changed |= blend(&mut current.cpu_clock, target.cpu_clock, amount);
    changed |= blend(
        &mut current.cpu_clock_average,
        target.cpu_clock_average,
        amount,
    );
    changed |= blend(
        &mut current.cpu_clock_effective,
        target.cpu_clock_effective,
        amount,
    );
    changed |= blend(&mut current.gpu_temperature, target.gpu_temperature, amount);
    changed |= blend(&mut current.gpu_usage, target.gpu_usage, amount);
    changed |= blend(&mut current.gpu_clock, target.gpu_clock, amount);
    changed |= blend(&mut current.gpu_power, target.gpu_power, amount);
    changed |= blend(&mut current.ram_usage, target.ram_usage, amount);
    changed |= blend(&mut current.vram_usage, target.vram_usage, amount);
    changed |= blend(&mut current.vram_used_mb, target.vram_used_mb, amount);
    changed |= blend(&mut current.vram_total_mb, target.vram_total_mb, amount);
    changed |= blend(&mut current.disk_usage, target.disk_usage, amount);
    changed |= blend(&mut current.network_upload, target.network_upload, amount);
    changed |= blend(
        &mut current.network_download,
        target.network_download,
        amount,
    );
    changed |= blend(&mut current.fan_speed, target.fan_speed, amount);
    changed |= blend(&mut current.system_volume, target.system_volume, 0.68);
    if current.history_cpu != target.history_cpu {
        current.history_cpu.clone_from(&target.history_cpu);
        changed = true;
    }
    if current.history_gpu != target.history_gpu {
        current.history_gpu.clone_from(&target.history_gpu);
        changed = true;
    }
    if current.history_gpu_power != target.history_gpu_power {
        current
            .history_gpu_power
            .clone_from(&target.history_gpu_power);
        changed = true;
    }
    if current.history_network_upload != target.history_network_upload {
        current
            .history_network_upload
            .clone_from(&target.history_network_upload);
        changed = true;
    }
    if current.history_network_download != target.history_network_download {
        current
            .history_network_download
            .clone_from(&target.history_network_download);
        changed = true;
    }
    if current.history_volume != target.history_volume {
        current.history_volume.clone_from(&target.history_volume);
        changed = true;
    }
    current.fan_sensors.clone_from(&target.fan_sensors);
    changed
}

fn volume_widget_enabled(config: &AppConfig) -> bool {
    use crate::config::schema::WidgetKind;
    config
        .widgets
        .iter()
        .any(|widget| widget.enabled && widget.kind == WidgetKind::Volume)
}

fn renderer_sleep_interval(config: &AppConfig, transition_active: bool) -> Duration {
    if transition_active {
        Duration::from_millis(50)
    } else if volume_widget_enabled(config) {
        Duration::from_millis(config.frame_interval_ms.clamp(50, 100))
    } else {
        Duration::from_millis(config.frame_interval_ms.clamp(50, 1000))
    }
}

fn renderer_dynamic_key(config: &AppConfig) -> String {
    use crate::config::schema::{BackgroundSource, WidgetKind};
    use chrono::Local;
    use std::time::{SystemTime, UNIX_EPOCH};

    let has_clock = config
        .widgets
        .iter()
        .any(|widget| widget.enabled && widget.kind == WidgetKind::Clock);
    let has_date = config
        .widgets
        .iter()
        .any(|widget| widget.enabled && widget.kind == WidgetKind::Date);
    let now = Local::now();
    let clock = has_clock.then(|| now.format("%Y%m%d%H%M").to_string());
    let date = has_date.then(|| now.format("%Y%m%d").to_string());
    let slide = if config.background.source == BackgroundSource::Folder {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Some(
            seconds
                / config
                    .background
                    .slideshow_interval_minutes
                    .max(1)
                    .saturating_mul(60),
        )
    } else {
        None
    };
    format!("{clock:?}:{date:?}:{slide:?}")
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
    next.history_gpu_power = append(previous.history_gpu_power.clone(), next.gpu_power, limit);
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
    next.history_volume = append(previous.history_volume.clone(), next.system_volume, limit);
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
pub fn get_autostart() -> Result<bool, String> {
    windows_startup::is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    windows_startup::set_enabled(enabled).map_err(|e| e.to_string())
}
