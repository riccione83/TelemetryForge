use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NamedSensor {
    pub id: String,
    pub name: String,
    pub value: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SensorSnapshot {
    pub cpu_temperature: Option<f32>,
    pub cpu_temperature_core: Option<f32>,
    pub cpu_temperature_socket: Option<f32>,
    pub cpu_usage: Option<f32>,
    pub cpu_clock: Option<f32>,
    pub cpu_clock_average: Option<f32>,
    pub cpu_clock_effective: Option<f32>,
    pub gpu_temperature: Option<f32>,
    pub gpu_usage: Option<f32>,
    pub gpu_clock: Option<f32>,
    pub gpu_power: Option<f32>,
    pub ram_usage: Option<f32>,
    pub vram_usage: Option<f32>,
    pub vram_used_mb: Option<f32>,
    pub vram_total_mb: Option<f32>,
    pub disk_usage: Option<f32>,
    pub network_upload: Option<f32>,
    pub network_download: Option<f32>,
    pub fan_speed: Option<f32>,
    pub system_volume: Option<f32>,
    pub weather_temperature: Option<f32>,
    pub weather_humidity: Option<f32>,
    pub weather_wind_speed: Option<f32>,
    pub weather_code: Option<u16>,
    pub weather_condition: Option<String>,
    pub fan_sensors: Vec<NamedSensor>,
    pub history_cpu: Vec<f32>,
    pub history_gpu: Vec<f32>,
    pub history_gpu_power: Vec<f32>,
    pub history_network_download: Vec<f32>,
    pub history_network_upload: Vec<f32>,
    pub history_volume: Vec<f32>,
}
