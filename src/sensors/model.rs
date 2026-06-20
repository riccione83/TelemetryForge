use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SensorSnapshot {
    pub cpu_temperature: Option<f32>,
    pub cpu_usage: Option<f32>,
    pub gpu_temperature: Option<f32>,
    pub gpu_usage: Option<f32>,
    pub gpu_clock: Option<f32>,
    pub ram_usage: Option<f32>,
    pub vram_usage: Option<f32>,
    pub disk_usage: Option<f32>,
    pub network_upload: Option<f32>,
    pub network_download: Option<f32>,
    pub fan_speed: Option<f32>,
    pub history_cpu: Vec<f32>,
    pub history_gpu: Vec<f32>,
    pub history_network_download: Vec<f32>,
    pub history_network_upload: Vec<f32>,
}
