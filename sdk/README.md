# TelemetryForge Super Widget SDK

The SDK lets third-party developers create self-contained TelemetryForge
components in Rust. A Super Widget can combine multiple live sensors, custom
graphics, text and animation inside one movable and resizable editor object.

Components compile to sandboxed WebAssembly and are distributed as a single
`.superwidget` package. Users can install them from **Widgets → Import
component** without rebuilding TelemetryForge.

## What can be created?

The current SDK is suitable for:

- circular CPU/GPU command dials;
- multi-sensor dashboard cards;
- animated scanner, radar and reactor-style components;
- temperature, usage, frequency and power gauges;
- custom bars, rings, scales, labels and numeric readouts.

See:

- [`examples/hello-dial`](examples/hello-dial) for the smallest example;
- [`examples/reactor-core`](examples/reactor-core) for a complete animated
  multi-sensor component.

## Requirements

- Rust stable;
- the `wasm32-unknown-unknown` target.

```powershell
rustup target add wasm32-unknown-unknown
```

## Create a project

A component directory contains:

```text
my-widget/
├── Cargo.toml
├── manifest.yaml
└── src/
    └── lib.rs
```

Example `Cargo.toml`:

```toml
[package]
name = "my-super-widget"
version = "0.1.0"
edition = "2021"
license = "MIT"

[lib]
crate-type = ["cdylib"]
test = false
doctest = false

[dependencies]
telemetryforge-superwidget-sdk = { path = "../../telemetryforge-superwidget-sdk" }
```

## Manifest

Example `manifest.yaml`:

```yaml
id: com.example.command-dial
name: Command Dial
description: An animated CPU command dial.
runtime: wasm
abi_version: 1
entry: widget.wasm
width: 240
height: 240
animated_fps: 8
sensors:
  - cpu_usage
  - cpu_temperature
  - cpu_clock
  - fan_speed
```

Important fields:

- `id` must be unique and may contain letters, numbers, `.`, `-` and `_`;
- `width` and `height` are the default editor dimensions;
- `animated_fps` is optional and should be `0` for static widgets;
- use the lowest acceptable animation rate to reduce USB traffic and CPU use;
- `sensors` documents the data required by the component.

## Minimal Rust component

Every component exports `tf_render`:

```rust
use telemetryforge_superwidget_sdk::{rgba, Canvas, Sensor};

#[no_mangle]
pub extern "C" fn tf_render(width: i32, height: i32) {
    let canvas = Canvas::new(width, height);
    let centre = (canvas.width / 2.0, canvas.height / 2.0);
    let radius = canvas.width.min(canvas.height) * 0.42;
    let usage = canvas.sensor(Sensor::CpuUsage).clamp(0.0, 100.0);

    canvas.clear(rgba(2, 6, 12, 240));
    canvas.circle(centre, radius, 2.0, rgba(45, 90, 120, 255));
    canvas.arc(
        centre,
        radius - 8.0,
        6.0,
        -220.0,
        usage * 2.6,
        rgba(30, 225, 255, 255),
    );
    canvas.number_center(
        (centre.0, centre.1 - 15.0),
        34.0,
        rgba(245, 250, 255, 255),
        usage,
        0,
    );
    canvas.text_center(
        (centre.0, centre.1 + 24.0),
        12.0,
        rgba(30, 225, 255, 255),
        "CPU LOAD",
    );
}
```

Coordinates are relative to the dimensions supplied to `tf_render`. Scale
sizes from `canvas.width.min(canvas.height)` so the component remains clean
when resized in the editor.

## Available sensors

```rust
Sensor::CpuUsage
Sensor::CpuTemperature
Sensor::CpuClock
Sensor::FanSpeed
Sensor::GpuUsage
Sensor::GpuTemperature
Sensor::GpuClock
Sensor::GpuPower
Sensor::RamUsage
Sensor::VramUsage
Sensor::Volume
```

Unavailable sensors return `0.0`.

## Drawing API

`Canvas` currently provides:

```rust
canvas.clear(colour);
canvas.line(from, to, width, colour);
canvas.circle(centre, radius, width, colour);
canvas.fill_circle(centre, radius, colour);
canvas.arc(centre, radius, width, start_degrees, sweep_degrees, colour);
canvas.text(position, size, colour, value);
canvas.text_center(position, size, colour, value);
canvas.number_center(position, size, colour, value, decimals);
canvas.animation_ms();
canvas.sensor(sensor);
```

Colours use:

```rust
rgba(red, green, blue, alpha)
```

## Animation

Set `animated_fps` in the manifest and use `animation_ms()`:

```rust
let rotation = canvas.animation_ms() as f32 * 0.04;
canvas.arc(centre, radius, 3.0, rotation, 10.0, rgba(255, 255, 255, 255));
```

For TURZX displays, prefer small moving indicators over long rotating arcs.
Keep static rings static and animate only the smallest possible area. This lets
TelemetryForge send small changed regions instead of repeatedly transferring a
large part of the display.

Recommended rates:

- `5–8 FPS` for subtle gauges;
- `8–12 FPS` for scanners and dials;
- avoid `20+ FPS` on serial USB displays.

## Build

```powershell
cargo build -p my-super-widget --release --target wasm32-unknown-unknown
```

Copy the generated module into the component directory as `widget.wasm`:

```powershell
Copy-Item `
  target\wasm32-unknown-unknown\release\my_super_widget.wasm `
  sdk\examples\my-widget\widget.wasm
```

## Package

Use the included CLI:

```powershell
cargo run -p telemetryforge-superwidget-cli -- pack `
  sdk\examples\my-widget `
  target\my-widget.superwidget
```

The package is a ZIP archive containing:

```text
manifest.yaml
widget.wasm
```

`pack-wat` is available for low-level ABI development and automated tests:

```powershell
cargo run -p telemetryforge-superwidget-cli -- pack-wat `
  sdk\examples\reactor-core `
  target\reactor-core.superwidget
```

Normal third-party components should be authored in Rust and packaged with
`pack`.

## Install and test

1. Open TelemetryForge.
2. Open the Widget panel.
3. Select **Import component**.
4. Choose the `.superwidget` file.
5. Select the installed component from the normal widget picker.
6. Add, move and resize it like any other widget.

Installed components are stored in:

```text
%LOCALAPPDATA%\TelemetryForge\superwidgets
```

## Sandbox

Plugins run without WASI and have no filesystem, network, process or shell
access. TelemetryForge currently enforces:

- an 8 MB WebAssembly memory limit;
- a two-million-instruction fuel budget per render;
- host-controlled sensor and drawing access only.

## Current SDK limitations

The SDK can recreate the visual structure of the bundled CPU/GPU Command Dial,
but the native renderer still has some capabilities not yet exposed through
ABI version 1:

- embedded PNG/SVG/GIF assets;
- polygons and filled triangles;
- selectable fonts;
- gradients and blur/glow primitives;
- historical sensor arrays;
- per-instance sensor bindings, such as selecting one specific fan or CPU
  temperature source inside an external component;
- mouse interaction or custom settings panels.

These are planned ABI additions. Components using ABI version 1 will remain
compatible when new optional APIs are introduced.

## Design recommendations

- Scale every coordinate from the supplied component size.
- Keep the background transparent when the screen design should show through.
- Use static geometry for scales and tracks.
- Animate short indicators rather than entire rings.
- Keep text inside safe margins because Windows font metrics may vary slightly.
- Use unique reverse-domain IDs such as `com.username.widget-name`.
