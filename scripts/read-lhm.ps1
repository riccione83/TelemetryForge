param([string]$DllPath = $env:TURZX_LHM_DLL)

$ErrorActionPreference = "Stop"
if ([string]::IsNullOrWhiteSpace($DllPath)) {
    throw "LibreHardwareMonitorLib.dll path was not provided"
}
$dllDirectory = Split-Path -Parent $DllPath
Set-Location $dllDirectory
Add-Type -Path $DllPath
$computer = [LibreHardwareMonitor.Hardware.Computer]::new()
$computer.IsCpuEnabled = $true
$computer.IsGpuEnabled = $true
$computer.IsMemoryEnabled = $true
$computer.IsMotherboardEnabled = $true
$computer.IsControllerEnabled = $true
$computer.Open()

$values = @{
    cpu_temperature = $null
    cpu_temperature_core = $null
    cpu_temperature_socket = $null
    gpu_temperature = $null
    gpu_usage = $null
    gpu_clock = $null
    vram_usage = $null
    fan_speed = $null
}
$cpuSocketTemperature = $null
$cpuControlTemperature = $null

function Visit-Hardware($hardware) {
    $hardware.Update()
    foreach ($sub in $hardware.SubHardware) { Visit-Hardware $sub }
    foreach ($sensor in $hardware.Sensors) {
        if ($null -eq $sensor.Value) { continue }
        $type = $sensor.SensorType.ToString()
        $name = $sensor.Name
        $hardwareType = $hardware.HardwareType.ToString()
        if ($type -eq "Temperature" -and $hardwareType -eq "SuperIO" -and $name -match "^CPU Socket$") {
            $script:cpuSocketTemperature = [single]$sensor.Value
        }
        if ($type -eq "Temperature" -and $hardwareType -like "Cpu*" -and $name -match "Tctl|Tdie|Package|Core" -and $null -eq $script:cpuControlTemperature) {
            $script:cpuControlTemperature = [single]$sensor.Value
        }
        if ($type -eq "Temperature" -and $hardwareType -like "Gpu*" -and $null -eq $values.gpu_temperature) {
            $values.gpu_temperature = [single]$sensor.Value
        }
        if ($type -eq "Load" -and $hardwareType -like "Gpu*" -and $name -match "Core|GPU" -and $null -eq $values.gpu_usage) {
            $values.gpu_usage = [single]$sensor.Value
        }
        if ($type -eq "Clock" -and $hardwareType -like "Gpu*" -and $name -match "Core|GPU" -and $null -eq $values.gpu_clock) {
            $values.gpu_clock = [single]$sensor.Value
        }
        if ($type -eq "Load" -and $hardwareType -like "Gpu*" -and $name -match "Memory" -and $null -eq $values.vram_usage) {
            $values.vram_usage = [single]$sensor.Value
        }
        if ($type -eq "Fan" -and $null -eq $values.fan_speed) {
            $values.fan_speed = [single]$sensor.Value
        }
    }
}

try {
    foreach ($hardware in $computer.Hardware) { Visit-Hardware $hardware }
    if ($null -ne $cpuSocketTemperature) {
        $values.cpu_temperature_socket = $cpuSocketTemperature
    }
    if ($null -ne $cpuControlTemperature) {
        $values.cpu_temperature_core = $cpuControlTemperature
    }
    $values.cpu_temperature = $cpuControlTemperature
    $values | ConvertTo-Json -Compress
}
finally {
    $computer.Close()
}
