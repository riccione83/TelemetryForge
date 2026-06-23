use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub display: DisplayConfig,
    pub background: BackgroundConfig,
    pub theme: ThemeConfig,
    pub widgets: Vec<WidgetConfig>,
    pub sensor_poll_ms: u64,
    pub frame_interval_ms: u64,
    pub libre_hardware_monitor_dll: Option<String>,
    pub cpu_temperature_source: CpuTemperatureSource,
    pub cpu_clock_source: CpuClockSource,
    pub fan_sensor: Option<String>,
    pub weather: WeatherConfig,
    pub automation: AutomationConfig,
    pub transition: TransitionConfig,
    pub remote: RemoteConfig,
    pub quick_screens: QuickScreensConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WeatherConfig {
    pub enabled: bool,
    pub latitude: f64,
    pub longitude: f64,
    pub refresh_minutes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct QuickScreensConfig {
    pub gaming: Option<String>,
    pub minimal: Option<String>,
    pub idle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoteConfig {
    pub enabled: bool,
    pub authentication_enabled: bool,
    pub username: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutomationConfig {
    pub enabled: bool,
    pub default_screen: Option<String>,
    pub rules: Vec<AutomationRule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutomationRule {
    pub enabled: bool,
    pub kind: AutomationRuleKind,
    pub process_name: String,
    pub threshold: f32,
    pub idle_seconds: u64,
    pub sustain_seconds: u64,
    pub release_seconds: u64,
    pub screen: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationRuleKind {
    ProcessRunning,
    GpuTemperatureAbove,
    CpuTemperatureAbove,
    GpuUsageAbove,
    CpuUsageAbove,
    IdleFor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TransitionConfig {
    pub kind: TransitionKind,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    None,
    Fade,
    Slide,
    Dissolve,
    Glitch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuTemperatureSource {
    Auto,
    Core,
    Socket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuClockSource {
    Average,
    Effective,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub model: String,
    pub port: String,
    pub width: u32,
    pub height: u32,
    pub orientation: Orientation,
    pub brightness: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Portrait,
    ReversePortrait,
    Landscape,
    ReverseLandscape,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BackgroundConfig {
    pub source: BackgroundSource,
    pub path: Option<String>,
    pub folder: Option<String>,
    pub slideshow_interval_minutes: u64,
    pub mode: BackgroundMode,
    pub colour: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundSource {
    Colour,
    File,
    Folder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundMode {
    Contain,
    Cover,
    Stretch,
    Centre,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub name: String,
    pub foreground: String,
    pub accent: String,
    pub panel: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WidgetConfig {
    pub editor_name: String,
    pub locked: bool,
    pub group_id: Option<String>,
    pub kind: WidgetKind,
    pub render_mode: WidgetRenderMode,
    pub enabled: bool,
    pub left_text: String,
    pub right_text: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub font: String,
    pub font_size: f32,
    pub colour: String,
    pub secondary_colour: String,
    pub opacity: f32,
    pub graph_background_colour: String,
    pub graph_background_opacity: f32,
    pub gif_path: Option<String>,
    pub gif_fps: u16,
    pub gif_loop: bool,
    pub gif_fit: BackgroundMode,
    pub superwidget_id: Option<String>,
    pub superwidget_background_colour: String,
    pub superwidget_background_opacity: f32,
    pub superwidget_bindings: HashMap<String, String>,
    pub glow: u8,
    pub shadow: u8,
    pub use_thresholds: bool,
    pub warning_threshold: f32,
    pub critical_threshold: f32,
    pub warning_colour: String,
    pub critical_colour: String,
    pub circle_thickness: f32,
    pub circle_start_angle: f32,
    pub circle_sweep_angle: f32,
    pub refresh_interval_ms: u64,
    pub label_format: String,
}

#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WidgetKind {
    CpuTemperature,
    CpuUsage,
    CpuClock,
    GpuTemperature,
    GpuUsage,
    GpuClock,
    GpuPower,
    RamUsage,
    VramUsage,
    DiskUsage,
    NetworkUpload,
    NetworkDownload,
    FanSpeed,
    Clock,
    Date,
    Fps,
    Text,
    Gif,
    Volume,
    WeatherTemperature,
    WeatherHumidity,
    WeatherWind,
    WeatherCondition,
    WeatherIcon,
    SuperWidget,
}

#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WidgetRenderMode {
    Text,
    Bar,
    Circle,
    Graph,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            display: DisplayConfig::default(),
            background: BackgroundConfig::default(),
            theme: ThemeConfig::default(),
            widgets: vec![
                WidgetConfig::new(WidgetKind::CpuTemperature, 24, 40, "{value}"),
                WidgetConfig::new(WidgetKind::GpuTemperature, 250, 40, "{value}"),
                WidgetConfig::new(WidgetKind::RamUsage, 24, 235, "{value}"),
                WidgetConfig::new(WidgetKind::Clock, 300, 235, "{value}"),
            ],
            sensor_poll_ms: 1000,
            frame_interval_ms: 1000,
            libre_hardware_monitor_dll: None,
            cpu_temperature_source: CpuTemperatureSource::Core,
            cpu_clock_source: CpuClockSource::Average,
            fan_sensor: None,
            weather: WeatherConfig::default(),
            automation: AutomationConfig::default(),
            transition: TransitionConfig::default(),
            remote: RemoteConfig::default(),
            quick_screens: QuickScreensConfig::default(),
        }
    }
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            authentication_enabled: false,
            username: "admin".into(),
            password_hash: String::new(),
        }
    }
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            latitude: 51.5074,
            longitude: -0.1278,
            refresh_minutes: 15,
        }
    }
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_screen: None,
            rules: Vec::new(),
        }
    }
}

impl Default for AutomationRule {
    fn default() -> Self {
        Self {
            enabled: true,
            kind: AutomationRuleKind::ProcessRunning,
            process_name: String::new(),
            threshold: 80.0,
            idle_seconds: 300,
            sustain_seconds: 3,
            release_seconds: 8,
            screen: String::new(),
        }
    }
}

impl Default for AutomationRuleKind {
    fn default() -> Self {
        Self::ProcessRunning
    }
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            kind: TransitionKind::Fade,
            duration_ms: 450,
        }
    }
}

impl Default for TransitionKind {
    fn default() -> Self {
        Self::Fade
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            model: "Turing Smart Screen 3.5".into(),
            port: "AUTO".into(),
            width: 480,
            height: 320,
            orientation: Orientation::Landscape,
            brightness: 80,
        }
    }
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            source: BackgroundSource::Colour,
            path: None,
            folder: None,
            slideshow_interval_minutes: 5,
            mode: BackgroundMode::Cover,
            colour: "#10151d".into(),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "Midnight".into(),
            foreground: "#f3f7ff".into(),
            accent: "#64d8cb".into(),
            panel: "#18212d".into(),
        }
    }
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self::new(WidgetKind::Clock, 20, 20, "{value}")
    }
}

impl WidgetConfig {
    pub fn new(kind: WidgetKind, x: i32, y: i32, label: &str) -> Self {
        Self {
            editor_name: String::new(),
            locked: false,
            group_id: None,
            kind,
            render_mode: WidgetRenderMode::Text,
            enabled: true,
            left_text: String::new(),
            right_text: String::new(),
            x,
            y,
            width: 210,
            height: 54,
            font: "Segoe UI".into(),
            font_size: 30.0,
            colour: "#f3f7ff".into(),
            secondary_colour: "#64d8cb".into(),
            opacity: 1.0,
            graph_background_colour: "#000000".into(),
            graph_background_opacity: 0.0,
            gif_path: None,
            gif_fps: 8,
            gif_loop: true,
            gif_fit: BackgroundMode::Contain,
            superwidget_id: None,
            superwidget_background_colour: "#000000".into(),
            superwidget_background_opacity: 0.0,
            superwidget_bindings: HashMap::new(),
            glow: 0,
            shadow: 0,
            use_thresholds: false,
            warning_threshold: 70.0,
            critical_threshold: 90.0,
            warning_colour: "#ffd166".into(),
            critical_colour: "#ff4d6d".into(),
            circle_thickness: 16.0,
            circle_start_angle: -90.0,
            circle_sweep_angle: 360.0,
            refresh_interval_ms: 1000,
            label_format: label.into(),
        }
    }
}

impl Default for Orientation {
    fn default() -> Self {
        Self::Landscape
    }
}

impl Default for BackgroundMode {
    fn default() -> Self {
        Self::Cover
    }
}

impl Default for BackgroundSource {
    fn default() -> Self {
        // Keeps older configs with only `path` working after the slideshow
        // fields were introduced.
        Self::File
    }
}

impl Default for WidgetKind {
    fn default() -> Self {
        Self::Clock
    }
}

impl Default for WidgetRenderMode {
    fn default() -> Self {
        Self::Text
    }
}

impl Default for CpuTemperatureSource {
    fn default() -> Self {
        Self::Core
    }
}

impl Default for CpuClockSource {
    fn default() -> Self {
        Self::Average
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_widget_round_trips_through_yaml() {
        let mut widget = WidgetConfig::new(WidgetKind::CpuUsage, 10, 20, "{value}");
        widget.render_mode = WidgetRenderMode::Graph;
        widget.opacity = 0.75;
        widget.graph_background_colour = "#123456".into();
        widget.graph_background_opacity = 0.45;
        widget.glow = 6;
        widget.shadow = 3;
        let yaml = serde_yaml::to_string(&widget).unwrap();
        let decoded: WidgetConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(decoded.render_mode, WidgetRenderMode::Graph);
        assert_eq!(decoded.glow, 6);
        assert_eq!(decoded.shadow, 3);
        assert!((decoded.opacity - 0.75).abs() < f32::EPSILON);
        assert_eq!(decoded.graph_background_colour, "#123456");
        assert!((decoded.graph_background_opacity - 0.45).abs() < f32::EPSILON);
    }

    #[test]
    fn legacy_widget_gets_transparent_graph_background_defaults() {
        let yaml = r#"
kind: cpu_usage
render_mode: graph
enabled: true
"#;
        let decoded: WidgetConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(decoded.graph_background_colour, "#000000");
        assert_eq!(decoded.graph_background_opacity, 0.0);
    }

    #[test]
    fn legacy_widget_gets_editor_metadata_defaults() {
        let yaml = r#"
kind: cpu_usage
enabled: true
x: 12
y: 34
"#;
        let decoded: WidgetConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(decoded.editor_name.is_empty());
        assert!(!decoded.locked);
        assert_eq!(decoded.group_id, None);
        assert_eq!(decoded.x, 12);
        assert_eq!(decoded.y, 34);
    }

    #[test]
    fn gif_widget_round_trips_through_yaml() {
        let mut widget = WidgetConfig::new(WidgetKind::Gif, 30, 40, "");
        widget.width = 96;
        widget.height = 96;
        widget.gif_path = Some(r"C:\animations\mascot.gif".into());
        widget.gif_fps = 12;
        widget.gif_loop = false;
        widget.gif_fit = BackgroundMode::Cover;
        let yaml = serde_yaml::to_string(&widget).unwrap();
        let decoded: WidgetConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(decoded.kind, WidgetKind::Gif);
        assert_eq!(decoded.gif_path, widget.gif_path);
        assert_eq!(decoded.gif_fps, 12);
        assert!(!decoded.gif_loop);
        assert_eq!(decoded.gif_fit, BackgroundMode::Cover);
    }
}
