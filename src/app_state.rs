use crate::{
    config::{persistence, schema::AppConfig},
    sensors::model::SensorSnapshot,
};
use parking_lot::{Mutex, RwLock};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::{path::PathBuf, sync::Arc};

pub struct RenderWorker {
    pub stop: Arc<AtomicBool>,
}

pub struct AppState {
    pub config_path: PathBuf,
    pub config: Arc<RwLock<AppConfig>>,
    pub sensors: Arc<RwLock<SensorSnapshot>>,
    pub worker: Mutex<Option<RenderWorker>>,
    pub status: Arc<RwLock<String>>,
    pub scene_revision: Arc<AtomicU64>,
    pub active_screen: Arc<RwLock<Option<String>>>,
}

impl AppState {
    pub fn load() -> Self {
        if let Err(error) = crate::superwidgets::ensure_bundled() {
            tracing::error!(error = %format!("{error:#}"), "could not install bundled superwidgets");
        }
        let config_path = persistence::default_config_path();
        let config = match persistence::load_or_create(&config_path) {
            Ok(config) => config,
            Err(error) => {
                tracing::error!(error = %format!("{error:#}"), "could not load configuration");
                AppConfig::default()
            }
        };
        let active_screen = persistence::load_active_screen(&config_path)
            .or_else(|| persistence::infer_active_screen(&config_path, &config));
        if let Some(name) = active_screen.as_deref() {
            let _ = persistence::save_active_screen(&config_path, Some(name));
        }
        Self {
            config_path,
            config: Arc::new(RwLock::new(config)),
            sensors: Arc::new(RwLock::new(SensorSnapshot::default())),
            worker: Mutex::new(None),
            status: Arc::new(RwLock::new("Stopped".into())),
            scene_revision: Arc::new(AtomicU64::new(0)),
            active_screen: Arc::new(RwLock::new(active_screen)),
        }
    }
}
