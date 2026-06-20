use super::{libre_hardware_monitor::Reader as HardwareReader, model::SensorSnapshot};
use crate::config::schema::CpuTemperatureSource;
use std::{
    path::{Path, PathBuf},
    time::Instant,
};
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System};

pub struct SensorPoller {
    system: System,
    networks: Networks,
    disks: Disks,
    hardware_reader: Option<HardwareReader>,
    hardware_path: Option<PathBuf>,
    hardware_snapshot: SensorSnapshot,
    last_network_refresh: Instant,
}

impl SensorPoller {
    pub fn new() -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        system.refresh_cpu_usage();
        system.refresh_memory();
        let networks = Networks::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();
        Self {
            system,
            networks,
            disks,
            hardware_reader: None,
            hardware_path: None,
            hardware_snapshot: SensorSnapshot::default(),
            last_network_refresh: Instant::now(),
        }
    }

    pub fn read(
        &mut self,
        lhm_dll: Option<&str>,
        cpu_temperature_source: CpuTemperatureSource,
    ) -> SensorSnapshot {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.networks.refresh(true);
        self.disks.refresh(false);
        let network_interval = self.last_network_refresh.elapsed().as_secs_f32().max(0.001);
        self.last_network_refresh = Instant::now();

        let disk_usage = self
            .disks
            .list()
            .iter()
            .filter(|disk| disk.total_space() > 0)
            .max_by_key(|disk| disk.total_space())
            .map(|disk| {
                (disk.total_space() - disk.available_space()) as f32 / disk.total_space() as f32
                    * 100.0
            });
        let network_download = self
            .networks
            .values()
            .map(|network| network.received())
            .sum::<u64>() as f32
            / 1024.0
            / network_interval;
        let network_upload = self
            .networks
            .values()
            .map(|network| network.transmitted())
            .sum::<u64>() as f32
            / 1024.0
            / network_interval;

        self.refresh_hardware(lhm_dll);
        let hardware = &self.hardware_snapshot;
        SensorSnapshot {
            cpu_temperature: select_cpu_temperature(hardware, cpu_temperature_source),
            cpu_temperature_core: hardware.cpu_temperature_core,
            cpu_temperature_socket: hardware.cpu_temperature_socket,
            cpu_usage: Some(self.system.global_cpu_usage()),
            gpu_temperature: hardware.gpu_temperature,
            gpu_usage: hardware.gpu_usage,
            gpu_clock: hardware.gpu_clock,
            ram_usage: if self.system.total_memory() > 0 {
                Some(self.system.used_memory() as f32 / self.system.total_memory() as f32 * 100.0)
            } else {
                None
            },
            vram_usage: hardware.vram_usage,
            disk_usage,
            network_upload: Some(network_upload),
            network_download: Some(network_download),
            fan_speed: hardware.fan_speed,
            ..Default::default()
        }
    }

    fn refresh_hardware(&mut self, configured_path: Option<&str>) {
        let path = resolve_lhm_path(configured_path);
        if path != self.hardware_path {
            self.hardware_reader = None;
            self.hardware_path = path.clone();
        }
        let Some(path) = path else {
            return;
        };
        if self.hardware_reader.is_none() {
            match HardwareReader::start(&path) {
                Ok(reader) => self.hardware_reader = Some(reader),
                Err(error) => {
                    tracing::error!(
                        dll = %path.display(),
                        error = %format!("{error:#}"),
                        "LibreHardwareMonitor bridge startup failed"
                    );
                    return;
                }
            }
        }
        let result = self
            .hardware_reader
            .as_mut()
            .expect("hardware reader initialized")
            .read();
        match result {
            Ok(snapshot) => self.hardware_snapshot = snapshot,
            Err(error) => {
                tracing::error!(
                    dll = %path.display(),
                    error = %format!("{error:#}"),
                    "LibreHardwareMonitor read failed"
                );
                self.hardware_reader = None;
            }
        }
    }
}

impl Default for SensorPoller {
    fn default() -> Self {
        Self::new()
    }
}

pub fn read_snapshot(
    lhm_dll: Option<&str>,
    cpu_temperature_source: CpuTemperatureSource,
) -> SensorSnapshot {
    let mut poller = SensorPoller::new();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    poller.read(lhm_dll, cpu_temperature_source)
}

fn resolve_lhm_path(configured_path: Option<&str>) -> Option<PathBuf> {
    configured_path
        .filter(|path| Path::new(path).exists())
        .map(PathBuf::from)
        .or_else(|| {
            [
                r"C:\Program Files\LibreHardwareMonitor\LibreHardwareMonitorLib.dll",
                r"C:\Program Files (x86)\LibreHardwareMonitor\LibreHardwareMonitorLib.dll",
                r"C:\Program Files\FanControl\LibreHardwareMonitorLib.dll",
                r"C:\Program Files (x86)\FanControl\LibreHardwareMonitorLib.dll",
            ]
            .into_iter()
            .find(|path| Path::new(path).exists())
            .map(PathBuf::from)
        })
}

fn select_cpu_temperature(snapshot: &SensorSnapshot, source: CpuTemperatureSource) -> Option<f32> {
    match source {
        CpuTemperatureSource::Socket => snapshot
            .cpu_temperature_socket
            .or(snapshot.cpu_temperature_core),
        CpuTemperatureSource::Auto | CpuTemperatureSource::Core => snapshot
            .cpu_temperature_core
            .or(snapshot.cpu_temperature_socket)
            .or(snapshot.cpu_temperature),
    }
}
