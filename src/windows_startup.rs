use anyhow::{Context, Result};
use std::{
    os::windows::process::CommandExt,
    path::Path,
    process::{Command, Stdio},
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const TASK_NAME: &str = "TelemetryForge Fast Startup";

pub fn is_enabled() -> Result<bool> {
    let status = hidden_command("schtasks.exe")
        .args(["/Query", "/TN", TASK_NAME])
        .status()
        .context("Could not query the TelemetryForge startup task")?;
    Ok(status.success())
}

pub fn set_enabled(enabled: bool) -> Result<()> {
    remove_legacy_run_entries()?;
    if enabled {
        register_task()
    } else {
        delete_task()
    }
}

fn register_task() -> Result<()> {
    let executable = std::env::current_exe().context("Could not locate TelemetryForge.exe")?;
    let working_directory = executable.parent().unwrap_or_else(|| Path::new("."));
    let script = r#"
$ErrorActionPreference = 'Stop'
$user = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
$action = New-ScheduledTaskAction -Execute $env:TELEMETRYFORGE_EXE -Argument '--minimized' -WorkingDirectory $env:TELEMETRYFORGE_DIR
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $user
$settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew -Priority 4
$principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Limited
Register-ScheduledTask -TaskName $env:TELEMETRYFORGE_TASK -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Description 'Start TelemetryForge immediately when the user signs in.' -Force | Out-Null
"#;
    let output = hidden_powershell()
        .env("TELEMETRYFORGE_EXE", &executable)
        .env("TELEMETRYFORGE_DIR", working_directory)
        .env("TELEMETRYFORGE_TASK", TASK_NAME)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .context("Could not register the TelemetryForge startup task")?;
    if !output.status.success() {
        anyhow::bail!(
            "Could not register fast startup: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn delete_task() -> Result<()> {
    let status = hidden_command("schtasks.exe")
        .args(["/Delete", "/TN", TASK_NAME, "/F"])
        .status()
        .context("Could not remove the TelemetryForge startup task")?;
    if status.success() || !is_enabled()? {
        Ok(())
    } else {
        anyhow::bail!("Could not remove the TelemetryForge startup task")
    }
}

fn remove_legacy_run_entries() -> Result<()> {
    let script = r#"
$path = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
Remove-ItemProperty -Path $path -Name 'TelemetryForge' -ErrorAction SilentlyContinue
Remove-ItemProperty -Path $path -Name 'TurzxControl' -ErrorAction SilentlyContinue
"#;
    let output = hidden_powershell()
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .context("Could not remove legacy startup entries")?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "Could not remove legacy startup entries: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }
}

fn hidden_powershell() -> Command {
    let shell = if hidden_command("pwsh.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "exit 0"])
        .status()
        .is_ok_and(|status| status.success())
    {
        "pwsh.exe"
    } else {
        "powershell.exe"
    };
    hidden_command(shell)
}

fn hidden_command(program: &str) -> Command {
    let mut command = Command::new(program);
    command
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}
