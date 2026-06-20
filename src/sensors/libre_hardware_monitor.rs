use super::model::SensorSnapshot;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{os::windows::process::CommandExt, path::Path, process::Command};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Deserialize)]
struct LhmOutput {
    cpu_temperature: Option<f32>,
    cpu_temperature_core: Option<f32>,
    cpu_temperature_socket: Option<f32>,
    gpu_temperature: Option<f32>,
    gpu_usage: Option<f32>,
    gpu_clock: Option<f32>,
    vram_usage: Option<f32>,
    fan_speed: Option<f32>,
}

pub fn read(dll: &Path) -> Result<SensorSnapshot> {
    let script = include_str!("../../scripts/read-lhm.ps1");
    let mut probe = Command::new("pwsh.exe");
    probe.creation_flags(CREATE_NO_WINDOW).args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "exit 0",
    ]);
    let shell = if probe.status().is_ok() {
        "pwsh.exe"
    } else {
        "powershell.exe"
    };
    let mut command = Command::new(shell);
    command
        .creation_flags(CREATE_NO_WINDOW)
        .env("TURZX_LHM_DLL", dll)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ]);
    let output = command
        .output()
        .context("Could not start the LibreHardwareMonitor bridge")?;
    if !output.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = stdout
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('{'))
        .context("LibreHardwareMonitor did not return JSON")?;
    let parsed: LhmOutput = serde_json::from_str(json)?;
    Ok(SensorSnapshot {
        cpu_temperature: parsed.cpu_temperature,
        cpu_temperature_core: parsed.cpu_temperature_core,
        cpu_temperature_socket: parsed.cpu_temperature_socket,
        gpu_temperature: parsed.gpu_temperature,
        gpu_usage: parsed.gpu_usage,
        gpu_clock: parsed.gpu_clock,
        vram_usage: parsed.vram_usage,
        fan_speed: parsed.fan_speed,
        ..Default::default()
    })
}
