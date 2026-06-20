use crate::{
    config::{persistence, schema::AppConfig},
    sensors::model::SensorSnapshot,
};
use parking_lot::{Mutex, RwLock};
use std::sync::atomic::AtomicBool;
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
}

impl AppState {
    pub fn load() -> Self {
        let config_path = persistence::default_config_path();
        let config = match persistence::load_or_create(&config_path) {
            Ok(config) => config,
            Err(error) => {
                tracing::error!(error = %format!("{error:#}"), "could not load configuration");
                AppConfig::default()
            }
        };
        Self {
            config_path,
            config: Arc::new(RwLock::new(config)),
            sensors: Arc::new(RwLock::new(SensorSnapshot::default())),
            worker: Mutex::new(None),
            status: Arc::new(RwLock::new("Stopped".into())),
        }
    }
}
