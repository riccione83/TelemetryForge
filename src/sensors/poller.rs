use super::{libre_hardware_monitor, model::SensorSnapshot};
use crate::config::schema::CpuTemperatureSource;
use std::path::Path;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System};

pub fn read_snapshot(
    lhm_dll: Option<&str>,
    cpu_temperature_source: CpuTemperatureSource,
) -> SensorSnapshot {
    let mut system = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );
    let mut networks = Networks::new_with_refreshed_list();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    system.refresh_cpu_usage();
    system.refresh_memory();
    networks.refresh(true);

    let disks = Disks::new_with_refreshed_list();
    let disk_usage = disks
        .list()
        .iter()
        .filter(|disk| disk.total_space() > 0)
        .max_by_key(|disk| disk.total_space())
        .map(|disk| {
            (disk.total_space() - disk.available_space()) as f32 / disk.total_space() as f32 * 100.0
        });
    let interval_seconds = sysinfo::MINIMUM_CPU_UPDATE_INTERVAL
        .as_secs_f32()
        .max(0.001);
    let network_download = networks
        .values()
        .map(|network| network.received())
        .sum::<u64>() as f32
        / 1024.0
        / interval_seconds;
    let network_upload = networks
        .values()
        .map(|network| network.transmitted())
        .sum::<u64>() as f32
        / 1024.0
        / interval_seconds;

    let mut snapshot = SensorSnapshot {
        cpu_usage: Some(system.global_cpu_usage()),
        ram_usage: if system.total_memory() > 0 {
            Some(system.used_memory() as f32 / system.total_memory() as f32 * 100.0)
        } else {
            None
        },
        disk_usage,
        network_upload: Some(network_upload),
        network_download: Some(network_download),
        ..Default::default()
    };

    let detected_lhm = [
        r"C:\Program Files\LibreHardwareMonitor\LibreHardwareMonitorLib.dll",
        r"C:\Program Files (x86)\LibreHardwareMonitor\LibreHardwareMonitorLib.dll",
        r"C:\Program Files\FanControl\LibreHardwareMonitorLib.dll",
        r"C:\Program Files (x86)\FanControl\LibreHardwareMonitorLib.dll",
    ]
    .into_iter()
    .find(|path| Path::new(path).exists());
    if let Some(path) = lhm_dll
        .filter(|path| Path::new(path).exists())
        .or(detected_lhm)
    {
        match libre_hardware_monitor::read(Path::new(path)) {
            Ok(lhm) => {
                snapshot.cpu_temperature_core = lhm.cpu_temperature_core;
                snapshot.cpu_temperature_socket = lhm.cpu_temperature_socket;
                snapshot.cpu_temperature = match cpu_temperature_source {
                    CpuTemperatureSource::Socket => {
                        lhm.cpu_temperature_socket.or(lhm.cpu_temperature_core)
                    }
                    CpuTemperatureSource::Auto | CpuTemperatureSource::Core => lhm
                        .cpu_temperature_core
                        .or(lhm.cpu_temperature_socket)
                        .or(lhm.cpu_temperature),
                };
                snapshot.gpu_temperature = lhm.gpu_temperature;
                snapshot.gpu_usage = lhm.gpu_usage;
                snapshot.gpu_clock = lhm.gpu_clock;
                snapshot.vram_usage = lhm.vram_usage;
                snapshot.fan_speed = lhm.fan_speed;
            }
            Err(error) => tracing::error!(
                dll = %path,
                error = %format!("{error:#}"),
                "LibreHardwareMonitor read failed"
            ),
        }
    }
    snapshot
}
