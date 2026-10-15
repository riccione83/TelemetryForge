use serde::{Deserialize, Serialize};

pub fn timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SensorInfo {
    pub key: String,
    pub source: String,
    pub value: Option<f32>,
    pub unit: String,
    pub sampled_at_ms: Option<u64>,
    pub stale_after_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NamedSensor {
    pub id: String,
    pub name: String,
    pub value: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SensorSnapshot {
    pub sensor_info: Vec<SensorInfo>,
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
    /// Bytes / 1024 / second (KiB/s), historically labelled KB/s in the UI/API.
    pub network_upload: Option<f32>,
    /// Same raw unit as network_upload; histories and thresholds use it too.
    pub network_download: Option<f32>,
    pub fan_speed: Option<f32>,
    pub system_volume: Option<f32>,
    pub weather_temperature: Option<f32>,
    pub weather_humidity: Option<f32>,
    pub weather_wind_speed: Option<f32>,
    pub weather_code: Option<u16>,
    pub weather_condition: Option<String>,
    pub fan_sensors: Vec<NamedSensor>,
    pub disk_sensors: Vec<NamedSensor>,
    pub history_cpu: Vec<f32>,
    pub history_gpu: Vec<f32>,
    pub history_cpu_temperature: Vec<f32>,
    pub history_cpu_clock: Vec<f32>,
    pub history_gpu_temperature: Vec<f32>,
    pub history_gpu_clock: Vec<f32>,
    pub history_gpu_power: Vec<f32>,
    pub history_network_download: Vec<f32>,
    pub history_network_upload: Vec<f32>,
    pub history_volume: Vec<f32>,
}

impl SensorSnapshot {
    pub fn history(&self, kind: crate::config::schema::WidgetKind) -> &[f32] {
        use crate::config::schema::WidgetKind::*;
        match kind {
            CpuUsage => &self.history_cpu,
            CpuTemperature => &self.history_cpu_temperature,
            CpuClock => &self.history_cpu_clock,
            GpuUsage => &self.history_gpu,
            GpuTemperature => &self.history_gpu_temperature,
            GpuClock => &self.history_gpu_clock,
            GpuPower => &self.history_gpu_power,
            NetworkUpload => &self.history_network_upload,
            NetworkDownload => &self.history_network_download,
            Volume => &self.history_volume,
            _ => &[],
        }
    }
}
