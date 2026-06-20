use super::model::SensorSnapshot;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    io::{BufRead, BufReader, Write},
    os::windows::process::CommandExt,
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

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

pub struct Reader {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Reader {
    pub fn start(dll: &Path) -> Result<Self> {
        let script = include_str!("../../scripts/read-lhm.ps1");
        let shell = powershell();
        let mut command = Command::new(shell);
        command
            .creation_flags(CREATE_NO_WINDOW)
            .env("TURZX_LHM_DLL", dll)
            .env("TELEMETRYFORGE_PERSISTENT", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ]);
        let mut child = command
            .spawn()
            .context("Could not start the LibreHardwareMonitor bridge")?;
        let stdin = child.stdin.take().context("Bridge stdin is unavailable")?;
        let stdout = child
            .stdout
            .take()
            .context("Bridge stdout is unavailable")?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    pub fn read(&mut self) -> Result<SensorSnapshot> {
        self.stdin.write_all(b"read\n")?;
        self.stdin.flush()?;
        let mut line = String::new();
        let size = self.stdout.read_line(&mut line)?;
        if size == 0 {
            anyhow::bail!("LibreHardwareMonitor bridge stopped unexpectedly");
        }
        parse(&line)
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        let _ = self.stdin.write_all(b"quit\n");
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn powershell() -> &'static str {
    let mut probe = Command::new("pwsh.exe");
    probe.creation_flags(CREATE_NO_WINDOW).args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "exit 0",
    ]);
    if probe.status().is_ok() {
        "pwsh.exe"
    } else {
        "powershell.exe"
    }
}

fn parse(json: &str) -> Result<SensorSnapshot> {
    let parsed: LhmOutput = serde_json::from_str(json.trim())?;
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
