# TelemetryForge Code Changes

This document tracks the most important user-facing and architectural changes
made to TelemetryForge.

## Unreleased

### Automatic screens and transitions

- Added ordered automatic rules for running processes, CPU/GPU temperature
  thresholds, CPU/GPU usage thresholds and Windows idle time.
- Added per-rule activation and return delays to prevent screen flapping from
  short CPU/GPU load spikes.
- Added an optional fallback screen when no rule matches.
- Added fade, slide, dissolve and glitch transitions between saved screens.
- Rules run inside the renderer and continue working while the editor is
  hidden in the system tray.

### Windows volume widget

- Added the default Windows playback-device volume as a native sensor.
- Added a Volume widget supporting text, bar, circle and historical graph
  rendering without LibreHardwareMonitor.
- Volume is polled independently at 100 ms and uses faster visual smoothing,
  avoiding the multi-second delay of slower hardware sensors.

### Faster Windows startup

- Replaced the delayed `HKCU\Run` startup entry with an immediate Task
  Scheduler logon trigger.
- Added a priority-4, interactive, no-delay startup task that launches
  TelemetryForge minimized and starts rendering.
- Removed legacy duplicate `TelemetryForge` and `TurzxControl` startup entries.
- Made the original Turing 3.5-inch `HELLO` probe non-blocking, removing up to
  two seconds from display initialization.

### CPU clock source

- Added separate average and average-effective CPU clock readings.
- Made average CPU clock the default dashboard value for intuitive MHz/GHz
  reporting.
- Added a selector for users who prefer the technically accurate effective
  clock that includes sleeping cores.
- CPU and GPU frequency text automatically switches from MHz to GHz at
  1000 MHz.
- New CPU/GPU temperature and frequency widgets receive useful CPU/GPU labels
  and temperature units by default.

### Animated GIF widgets

- Added animated GIF as an independently movable and resizable widget.
- Added GIF file selection, configurable FPS, looping and fit mode.
- Preserved GIF transparency when compositing over dashboard backgrounds.
- GIF animation participates in partial display updates, limiting USB traffic
  to the widget's changed area.

### Portable screen packages

- Added import/export of a single `.telemetryforge` package.
- Packages include YAML configuration, background images and animated GIF
  widget assets.
- Imported assets are extracted into the TelemetryForge local data directory
  and paths are rewritten automatically.

### Quick screens

- Converted Gaming, Minimal and Idle from destructive style presets into
  assignable quick-load slots for saved screens.
- Removed the bundled Neon Sample action from the editor.

### VRAM accuracy

- Fixed VRAM usage accidentally reading GPU memory-controller load.
- VRAM percentage is now calculated from dedicated memory used divided by
  dedicated memory total, with the exact GPU Memory load sensor as fallback.

## Latest changes

These changes are included in the current `main` branch and continuous Windows
release.

### Threshold gradients

- Changed threshold-enabled bars and circles to use a positional colour scale:
  base colour → warning colour → critical colour.
- A critical indicator now preserves all three colour regions instead of
  recolouring the complete bar or circle with the critical colour.
- Historical graphs now colour each line segment from that segment's sensor
  value using the same configurable base → warning → critical scale.
- With thresholds disabled, graphs retain their original horizontal
  primary-to-secondary gradient.

### Editor usability

- Added collapsible widget configuration panels.
- Added **Collapse all** and **Expand all** controls.
- Widget panels start collapsed to keep large dashboards manageable.
- Selecting a widget from the preview automatically expands its editor panel.
- Double-clicking a widget in the live preview selects it, opens its settings
  and smoothly scrolls the editor to the corresponding panel.
- Added a full-screen live-preview editing mode.
- Full-screen mode preserves the display aspect ratio and supports dragging,
  resizing, multi-selection and alignment.
- Added larger selection and resize handles in full-screen mode.
- Pressing `Esc` exits full-screen preview mode.
- Double-clicking a widget while in full-screen mode exits full screen and
  opens that widget's settings.
- Suspended live-preview image refreshes while resizing a widget so periodic
  updates cannot interrupt pointer capture or rebuild the overlay mid-drag.
- The preview is refreshed once, immediately after resizing finishes.

### Colour controls

- Improved the visibility of native colour-picker controls.
- Added a hexadecimal `#RRGGBB` value beside every colour picker.
- Added colour normalization for three-digit and six-digit hexadecimal values.
- Background, foreground and accent colours now update the live preview
  immediately.
- Widget primary, gradient, warning and critical colours remain synchronized
  with their displayed swatches.
- Added an optional graph-only background colour.
- Added independent graph background alpha that does not affect the graph
  line, glow, shadow or general widget opacity.

### Rendering controls

- Replaced separate **Start** and **Stop** buttons with one state-aware button.
- The button displays **Start rendering** while stopped and **Stop rendering**
  while active.
- Added visual states for starting, stopping and active rendering.
- The button is synchronized with the renderer's real backend state, including
  rendering started automatically with Windows.

### Additional sensors

- Added a CPU clock widget based on LibreHardwareMonitor's average effective
  core clock, with average clock as a fallback.
- Added discovery of all named fan sensors exposed by LibreHardwareMonitor.
- Added a fan-sensor selector supporting CPU fan, pump, system and GPU fans.
- Added GPU power in watts as a text, bar, circle and historical graph widget.

## 0.1.0 Development History

### Renderer performance

- Added change-driven rendering so frames are generated only when their visible
  output changes.
- Preserved smooth interpolation while avoiding redraws for sub-pixel or
  visually identical value changes.
- Added visual signatures for text, bars, circles and historical graphs.
- Added partial display updates using the smallest changed rectangular region.
- Full-frame transmission is used only when most of the frame changed.
- Cached resized background images.
- Cached loaded Windows fonts.
- Reduced measured TelemetryForge CPU use by approximately 73% in the tested
  configuration, from about 1.07% to 0.29% total CPU.

### Sensor polling

- Reused a persistent `sysinfo` collector instead of rebuilding it for every
  sensor update.
- Windows-native system data is used for CPU usage, RAM, disks and networking.
- LibreHardwareMonitor remains responsible for hardware data that Windows does
  not expose reliably, including CPU/GPU temperatures, GPU load and clock,
  VRAM usage and fan speed.
- Replaced repeated PowerShell process creation with one hidden persistent
  LibreHardwareMonitor bridge.
- Kept network values normalized as transfer rates per second.
- Added selectable CPU temperature sources:
  - Core / AMD Tctl-Tdie
  - CPU socket
- Improved Ryzen CPU temperature sensor selection under load.

### Windows integration

- Moved configuration, saved screens and extracted samples to
  `%LOCALAPPDATA%\TelemetryForge`.
- Added migration from legacy configuration locations.
- Fixed Windows autostart attempting to save configuration in
  `C:\Windows\System32`.
- Windows autostart launches TelemetryForge minimized and starts rendering.
- Prevented console windows from appearing for the release application and
  LibreHardwareMonitor bridge.
- Added an explicit high-contrast TelemetryForge system-tray icon.
- Added tray actions to open or quit TelemetryForge.

### Dashboard editor

- Added reusable screen profiles that can be created, saved, loaded and
  deleted.
- Added drag-and-drop positioning from the live preview.
- Added independent resizing for every widget.
- Added multi-selection using drag selection or `Ctrl`+click.
- Added context-menu actions to align widgets vertically and distribute them
  with equal spacing.
- Fixed visual widget positions reverting after saving configuration.
- Added live preview updates while widget properties are edited.
- Added per-widget font selection.
- Added independent left and right text around sensor values.
- Added free-text widgets.

### Visual widgets

- Added text, bar, circular gauge and historical graph rendering modes.
- Added independently movable and resizable visual widgets.
- Added smooth bar and circle animation.
- Added CPU, GPU and network history graphs.
- Added configurable primary and secondary gradient colours.
- Added opacity, glow and shadow controls.
- Added configurable warning and critical thresholds.
- Added green/yellow/red-style threshold colour transitions.
- Added configurable circular indicator thickness, start angle and sweep angle.
- Added Gaming, Minimal and Idle visual presets.

### Backgrounds and themes

- Added image backgrounds with contain, cover, stretch and centre modes.
- Added optional folder-based background slideshows.
- Added a configurable slideshow interval.
- Added reusable themes and a neon sample dashboard.
- Added live preview rendering for background and theme changes.

### Display and protocol

- Added TURZX/Turing serial-display detection.
- Added automatic COM-port selection for known USB identifiers.
- Ported the minimal Turing 3.5-inch initialization and RGB565 frame protocol.
- Added orientation and display-resolution configuration.
- Added a diagnostic RGB test frame.
- Added brightness commands for supported display revisions.
- Added clear display connection and serial-port errors.

### Sensors and widgets

- Added CPU temperature and usage.
- Added GPU temperature, usage and clock.
- Added RAM and VRAM usage.
- Added disk usage.
- Added network upload and download rates.
- Added fan speed where exposed by LibreHardwareMonitor.
- Added clock, date, FPS placeholder and free text.

### Application and release infrastructure

- Renamed the project to **TelemetryForge**.
- Added English and Italian interfaces.
- Made English the default README language.
- Added English and Italian documentation.
- Added MIT licensing.
- Added Windows release builds.
- Added GitHub Actions for continuous and versioned Windows releases.
- Added logging and user-facing error reporting.

## Related commits

- `7280770` — Initial TelemetryForge release
- `c4bb4ed` — Fix CPU temperature source selection
- `b6621eb` — Fix Windows startup data paths
- `64d0fec` — Fix Windows tray icon
- `88a700c` — Optimize renderer and sensor polling
