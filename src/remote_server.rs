use crate::{
    app_state::AppState,
    config::{persistence, schema::AppConfig},
    display_driver, renderer,
    sensors::poller,
    superwidgets, ui, windows_startup,
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Request, Response, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::atomic::Ordering, time::Duration};
use tokio::{sync::oneshot, task::JoinHandle};

const INDEX: &str = include_str!("../frontend/index.html");
const STYLES: &str = include_str!("../frontend/styles.css");
const I18N: &str = include_str!("../frontend/i18n.js");
const APP: &str = include_str!("../frontend/app-v2.js");
const BRIDGE: &str = include_str!("../frontend/remote-bridge.js");

#[derive(Serialize)]
struct ApiResponse {
    ok: bool,
    result: Value,
    error: Option<String>,
}

pub async fn manage(state: AppState) {
    let mut server: Option<(oneshot::Sender<()>, JoinHandle<()>)> = None;
    loop {
        let enabled = state.config.read().remote.enabled;
        match (enabled, server.is_some()) {
            (true, false) => {
                let (shutdown_tx, shutdown_rx) = oneshot::channel();
                let server_state = state.clone();
                let handle = tokio::spawn(async move {
                    if let Err(error) = serve(server_state, shutdown_rx).await {
                        tracing::error!(%error, "Remote Deck server stopped");
                    }
                });
                server = Some((shutdown_tx, handle));
            }
            (false, true) => {
                let (shutdown, handle) = server.take().expect("server handle exists");
                let _ = shutdown.send(());
                let _ = handle.await;
            }
            _ => {}
        }
        if server
            .as_ref()
            .is_some_and(|(_, handle)| handle.is_finished())
        {
            if let Some((_, handle)) = server.take() {
                let _ = handle.await;
            }
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn serve(state: AppState, shutdown: oneshot::Receiver<()>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index))
        .route("/styles.css", get(styles))
        .route("/i18n.js", get(i18n))
        .route("/remote-bridge.js", get(bridge))
        .route("/app-v2.js", get(app_script))
        .route("/api/invoke/{command}", post(invoke))
        .route("/api/health", get(health))
        .route_layer(middleware::from_fn_with_state(state.clone(), authorize))
        .with_state(state);
    let address = SocketAddr::from(([0, 0, 0, 0], 8787));
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "TelemetryForge Remote Deck listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = shutdown.await;
        })
        .await?;
    Ok(())
}

async fn authorize(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let remote = state.config.read().remote.clone();
    if !remote.authentication_enabled {
        return next.run(request).await;
    }
    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Basic "))
        .and_then(|encoded| STANDARD.decode(encoded).ok())
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|credentials| {
            let (username, password) = credentials.split_once(':')?;
            Some(
                username == remote.username
                    && crate::remote_auth::verify_password(password, &remote.password_hash),
            )
        })
        .unwrap_or(false);
    if authorized {
        next.run(request).await
    } else {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(
                header::WWW_AUTHENTICATE,
                "Basic realm=\"TelemetryForge Remote Deck\"",
            )
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Body::from("Authentication required"))
            .expect("valid authentication response")
    }
}

async fn health() -> Json<Value> {
    Json(json!({"name":"TelemetryForge Remote Deck","version":env!("CARGO_PKG_VERSION")}))
}

async fn index() -> impl IntoResponse {
    let html = INDEX.replace(
        "<script src=\"i18n.js\"></script>",
        "<script src=\"remote-bridge.js\"></script>\n  <script src=\"i18n.js\"></script>",
    );
    asset("text/html; charset=utf-8", html.into_bytes())
}

async fn styles() -> impl IntoResponse {
    asset("text/css; charset=utf-8", STYLES.as_bytes().to_vec())
}

async fn i18n() -> impl IntoResponse {
    asset("text/javascript; charset=utf-8", I18N.as_bytes().to_vec())
}

async fn bridge() -> impl IntoResponse {
    asset("text/javascript; charset=utf-8", BRIDGE.as_bytes().to_vec())
}

async fn app_script() -> impl IntoResponse {
    asset("text/javascript; charset=utf-8", APP.as_bytes().to_vec())
}

fn asset(content_type: &'static str, bytes: Vec<u8>) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(bytes))
        .expect("valid asset response")
}

async fn invoke(
    State(state): State<AppState>,
    Path(command): Path<String>,
    payload: Option<Json<Value>>,
) -> impl IntoResponse {
    let arguments = payload
        .map(|Json(value)| value)
        .unwrap_or_else(|| json!({}));
    match invoke_command(&state, &command, arguments).await {
        Ok(result) => (
            StatusCode::OK,
            Json(ApiResponse {
                ok: true,
                result,
                error: None,
            }),
        ),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                ok: false,
                result: Value::Null,
                error: Some(error),
            }),
        ),
    }
}

async fn invoke_command(state: &AppState, command: &str, args: Value) -> Result<Value, String> {
    match command {
        "get_config" => value(ui::public_config(state.config.read().clone())),
        "get_remote_info" => value(ui::remote_info_state(state)),
        "set_remote_security" => {
            ui::set_remote_security_state(state, argument(&args, "settings")?)?;
            value(ui::remote_info_state(state))
        }
        "get_active_screen" => value(state.active_screen.read().clone()),
        "save_config" => {
            let mut config: AppConfig = argument(&args, "config")?;
            config.remote = state.config.read().remote.clone();
            config.quick_screens = state.config.read().quick_screens.clone();
            persistence::save(&state.config_path, &config).map_err(error)?;
            *state.config.write() = config;
            Ok(Value::Null)
        }
        "list_screens" => value(persistence::list_profiles(&state.config_path).map_err(error)?),
        "set_quick_screen" => {
            let slot: String = argument(&args, "slot")?;
            let screen: Option<String> = argument(&args, "screen")?;
            ui::set_quick_screen_state(state, &slot, screen)?;
            Ok(Value::Null)
        }
        "save_screen" => {
            let name: String = argument(&args, "name")?;
            let mut config: AppConfig = argument(&args, "config")?;
            config.remote = state.config.read().remote.clone();
            config.quick_screens = state.config.read().quick_screens.clone();
            let path = persistence::profile_path(&state.config_path, &name).map_err(error)?;
            persistence::save(&path, &ui::profile_config(config.clone())).map_err(error)?;
            persistence::save(&state.config_path, &config).map_err(error)?;
            persistence::save_active_screen(&state.config_path, Some(&name)).map_err(error)?;
            *state.config.write() = config;
            *state.active_screen.write() = Some(name);
            Ok(Value::Null)
        }
        "load_screen" => {
            let name: String = argument(&args, "name")?;
            let path = persistence::profile_path(&state.config_path, &name).map_err(error)?;
            let loaded = persistence::load_or_create(&path).map_err(error)?;
            let config = merge_screen_settings(loaded, &state.config.read());
            persistence::save(&state.config_path, &config).map_err(error)?;
            persistence::save_active_screen(&state.config_path, Some(&name)).map_err(error)?;
            *state.config.write() = config.clone();
            *state.active_screen.write() = Some(name);
            state.scene_revision.fetch_add(1, Ordering::Relaxed);
            value(ui::public_config(config))
        }
        "new_screen" => new_screen(state, argument(&args, "name")?),
        "delete_screen" => {
            let name: String = argument(&args, "name")?;
            let path = persistence::profile_path(&state.config_path, &name).map_err(error)?;
            if path.exists() {
                std::fs::remove_file(path).map_err(error)?;
            }
            if state.active_screen.read().as_deref() == Some(name.as_str()) {
                persistence::save_active_screen(&state.config_path, None).map_err(error)?;
                *state.active_screen.write() = None;
            }
            Ok(Value::Null)
        }
        "get_preview" => preview(state, state.config.read().clone()),
        "preview_config" => preview(state, argument(&args, "config")?),
        "test_sensors" => test_sensors(state),
        "list_displays" => value(display_driver::detection::list().map_err(error)?),
        "list_superwidgets" => value(superwidgets::list().map_err(error)?),
        "start_rendering" => {
            ui::start_rendering_state(state)?;
            Ok(Value::Null)
        }
        "stop_rendering" => {
            ui::stop_rendering_state(state);
            Ok(Value::Null)
        }
        "get_status" => value(ui::get_status_state(state)),
        "render_once" => {
            render_once(state)?;
            Ok(Value::Null)
        }
        "test_display" => value(test_display(state)?),
        "set_display_brightness" => {
            set_brightness(state, argument(&args, "brightness")?)?;
            Ok(Value::Null)
        }
        "get_autostart" => value(windows_startup::is_enabled().map_err(error)?),
        "set_autostart" => {
            windows_startup::set_enabled(argument(&args, "enabled")?).map_err(error)?;
            Ok(Value::Null)
        }
        "select_background"
        | "select_background_folder"
        | "select_gif"
        | "import_package"
        | "export_package"
        | "import_superwidget" => Err(
            "This file operation is not available in Remote Deck yet. Use the Windows app.".into(),
        ),
        _ => Err(format!("Unsupported Remote Deck command: {command}")),
    }
}

fn new_screen(state: &AppState, name: String) -> Result<Value, String> {
    let current = state.config.read().clone();
    let mut config = AppConfig::default();
    config.display = current.display;
    config.libre_hardware_monitor_dll = current.libre_hardware_monitor_dll;
    config.cpu_temperature_source = current.cpu_temperature_source;
    config.cpu_clock_source = current.cpu_clock_source;
    config.fan_sensor = current.fan_sensor;
    config.automation = current.automation;
    config.transition = current.transition;
    config.remote = current.remote;
    config.quick_screens = current.quick_screens;
    config.widgets.clear();
    let path = persistence::profile_path(&state.config_path, &name).map_err(error)?;
    if path.exists() {
        return Err("A screen with this name already exists".into());
    }
    persistence::save(&path, &ui::profile_config(config.clone())).map_err(error)?;
    persistence::save(&state.config_path, &config).map_err(error)?;
    persistence::save_active_screen(&state.config_path, Some(&name)).map_err(error)?;
    *state.config.write() = config.clone();
    *state.active_screen.write() = Some(name);
    state.scene_revision.fetch_add(1, Ordering::Relaxed);
    value(ui::public_config(config))
}

fn merge_screen_settings(mut screen: AppConfig, current: &AppConfig) -> AppConfig {
    screen.automation = current.automation.clone();
    screen.transition = current.transition.clone();
    screen.libre_hardware_monitor_dll = current.libre_hardware_monitor_dll.clone();
    screen.cpu_temperature_source = current.cpu_temperature_source;
    screen.cpu_clock_source = current.cpu_clock_source;
    screen.fan_sensor = current.fan_sensor.clone();
    screen.remote = current.remote.clone();
    screen.quick_screens = current.quick_screens.clone();
    screen
}

fn preview(state: &AppState, config: AppConfig) -> Result<Value, String> {
    let image = renderer::render(&config, &state.sensors.read()).map_err(error)?;
    let png = renderer::canvas::png_bytes(&image).map_err(error)?;
    value(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

fn test_sensors(state: &AppState) -> Result<Value, String> {
    let config = state.config.read().clone();
    let snapshot = poller::read_snapshot(
        config.libre_hardware_monitor_dll.as_deref(),
        config.cpu_temperature_source,
        config.cpu_clock_source,
        config.fan_sensor.as_deref(),
    );
    *state.sensors.write() = snapshot.clone();
    value(snapshot)
}

fn render_once(state: &AppState) -> Result<(), String> {
    let config = state.config.read().clone();
    let snapshot = poller::read_snapshot(
        config.libre_hardware_monitor_dll.as_deref(),
        config.cpu_temperature_source,
        config.cpu_clock_source,
        config.fan_sensor.as_deref(),
    );
    let image = renderer::render(&config, &snapshot).map_err(error)?;
    display_driver::send_frame(&config.display, &image).map_err(error)?;
    *state.sensors.write() = snapshot;
    *state.status.write() = "Frame sent".into();
    Ok(())
}

fn test_display(state: &AppState) -> Result<String, String> {
    use image::{Rgb, RgbImage};
    let config = state.config.read().clone();
    let port = display_driver::detection::resolve_port(&config.display.port).map_err(error)?;
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
    display_driver::send_frame(&config.display, &image).map_err(error)?;
    let message = format!("RGB test successfully sent to {port}");
    *state.status.write() = message.clone();
    Ok(message)
}

fn set_brightness(state: &AppState, brightness: u8) -> Result<(), String> {
    let mut config = state.config.write();
    config.display.brightness = brightness.min(100);
    persistence::save(&state.config_path, &config).map_err(error)?;
    if state.worker.lock().is_none() {
        display_driver::apply_brightness(&config.display).map_err(error)?;
    }
    Ok(())
}

fn argument<T: serde::de::DeserializeOwned>(value: &Value, name: &str) -> Result<T, String> {
    serde_json::from_value(
        value
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Missing argument: {name}"))?,
    )
    .map_err(error)
}

fn value<T: Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(error)
}

fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
