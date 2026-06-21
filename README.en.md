# TelemetryForge

An open-source Windows desktop application for TURZX/Turing Smart Screen
3.5-inch USB displays normally controlled by `UsbMonitor.exe`.

## Highlights

- Visual drag-and-drop screen editor with live preview
- Reusable YAML screen profiles
- CPU, GPU, RAM, VRAM, disk, network, fan, Windows volume and clock sensors
- Text, bars, circular gauges and historical graphs
- Per-widget fonts, gradients, opacity, glow, shadows and thresholds
- Multi-select, group movement, alignment and distribution
- Smooth animations and partial display updates
- Assignable Gaming, Minimal and Idle quick screens
- Automatic screen rules and configurable screen transitions
- English and Italian user interfaces
- System tray and Windows autostart

## Supported hardware

The current driver targets Turing/UsbMonitor revision A displays, including
devices identified as `USB35INCHIPSV2` or USB VID/PID `1a86:5722`.

- Serial speed: 115200 baud
- Flow control: hardware RTS/CTS
- Pixel format: RGB565 little-endian
- Typical resolutions: 480×320 landscape or 320×480 portrait

Always close `UsbMonitor.exe` before starting TelemetryForge.

## Requirements

1. Windows 10 or Windows 11
2. Microsoft Edge WebView2 Runtime
3. Rust stable with the recommended MSVC toolchain
4. The display serial driver, commonly CH340 or CH552
5. Optional: LibreHardwareMonitor or FanControl

```powershell
rustup default stable-x86_64-pc-windows-msvc
```

Node.js is not required.

## Development

```powershell
cargo run
```

Application data is stored in:

```text
%LOCALAPPDATA%\TelemetryForge
```

This includes `config.yaml`, saved screens and bundled samples. Legacy data
found next to the executable is migrated automatically. Windows autostart
loads the last active configuration and starts rendering in the system tray.
It uses an immediate Task Scheduler logon trigger instead of the delayed
`HKCU\Run` startup queue.

## Release build

```powershell
cargo build --release
```

The executable is written to `target\release\TelemetryForge.exe`.

To create a Tauri installer:

```powershell
cargo install tauri-cli --version "^2"
cargo tauri build
```

## LibreHardwareMonitor

TelemetryForge automatically checks common LibreHardwareMonitor and FanControl
installation folders. You can also set an explicit DLL path:

```yaml
libre_hardware_monitor_dll: 'C:\Tools\LibreHardwareMonitor\LibreHardwareMonitorLib.dll'
```

The bridge runs through `scripts\read-lhm.ps1` in a hidden PowerShell process.
Some sensors may require administrator privileges.

## Screen editor

Screens are stored as YAML files inside the `screens` directory. Widgets can
be dragged, resized, multi-selected, aligned and evenly distributed.

Each widget supports:

- text before and after the sensor value;
- a separate Windows font;
- primary and secondary gradient colours;
- opacity, glow and shadow;
- warning and critical thresholds;
- text, bar, circle or historical graph rendering;
- independent position and dimensions.

Circular gauges also support thickness, start angle and sweep angle.

The **System volume** widget reads the Windows default playback-device volume.
It can be displayed as text, a bar, a circle or a historical graph without
LibreHardwareMonitor.

## Automatic screens and transitions

Enable automation in the editor, choose an optional default screen and add
rules in priority order. The first matching rule wins. Rules can switch screen
when:

- a named process is running, such as `game.exe`;
- GPU temperature reaches a configured threshold;
- CPU temperature reaches a configured threshold;
- GPU or CPU usage stays above a configured percentage;
- the PC has been idle for a configured number of seconds.

Automatic rules continue running while TelemetryForge is hidden in the system
tray. Screen changes can use no transition, fade, slide, dissolve or glitch,
with a configurable duration. Each rule also has activation and return delays
to prevent brief workload spikes from repeatedly switching screens.

## Background slideshow

The background source can be a solid colour, a single image, or a folder.
Folder mode cycles through supported images (`png`, `jpg`, `jpeg`, `bmp`,
`gif`, and `webp`) at a configurable interval in minutes. Images are sorted by
filename and selected automatically.

## Rendering engine

Sensor values are interpolated for smooth animations. TelemetryForge keeps the
COM port open during continuous rendering and sends only the smallest changed
rectangle when that is more efficient than a full-frame refresh.

## Troubleshooting

### Display not detected

- Close `UsbMonitor.exe`, including its tray process.
- Reconnect the USB data cable.
- Check Device Manager for a COM port.
- Install or update the serial driver.
- Enter the port manually, for example `COM3`.

### Missing sensor values

- Confirm LibreHardwareMonitor or FanControl can see the sensor.
- Verify the configured DLL path.
- Try running TelemetryForge as administrator.
- Use **Test sensors** to inspect available values.

## Protocol source

The protocol was researched from
[`turing-smart-screen-python`](https://github.com/mathoudebine/turing-smart-screen-python),
especially its revision A driver. TelemetryForge is an independent Rust
implementation.

## Continuous downloads

Every push to `main` updates the **Continuous Build** prerelease with
`TelemetryForge.exe` and its SHA-256 checksum. Version tags such as `v1.0.0`
create stable GitHub releases.

## License

MIT. See [LICENSE](LICENSE).
