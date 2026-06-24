use crate::{
    config::schema::{
        AppConfig, AutomationRule, AutomationRuleKind, BootAnimationKind, TransitionKind,
    },
    sensors::model::SensorSnapshot,
};
use image::{Rgb, RgbImage};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use windows::Win32::{
    System::SystemInformation::GetTickCount64,
    UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
};

pub struct RuleEngine {
    processes: System,
    matched_since: HashMap<usize, Instant>,
    active_screen: Option<String>,
    active_last_match: Option<Instant>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            processes: System::new(),
            matched_since: HashMap::new(),
            active_screen: None,
            active_last_match: None,
        }
    }

    pub fn target_screen(
        &mut self,
        config: &AppConfig,
        sensors: &SensorSnapshot,
    ) -> Option<String> {
        if !config.automation.enabled {
            return None;
        }
        self.processes.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing(),
        );
        let now = Instant::now();
        let mut qualified = None;
        for (index, rule) in config.automation.rules.iter().enumerate() {
            if self.matches(rule, sensors) {
                let since = self.matched_since.entry(index).or_insert(now);
                if now.duration_since(*since) >= Duration::from_secs(rule.sustain_seconds) {
                    qualified = Some(rule.screen.trim().to_string());
                    break;
                }
            } else {
                self.matched_since.remove(&index);
            }
        }
        self.matched_since
            .retain(|index, _| *index < config.automation.rules.len());

        if let Some(screen) = qualified.filter(|screen| !screen.is_empty()) {
            self.active_screen = Some(screen.clone());
            self.active_last_match = Some(now);
            return Some(screen);
        }

        if let (Some(screen), Some(last_match)) =
            (self.active_screen.as_ref(), self.active_last_match)
        {
            let release_seconds = config
                .automation
                .rules
                .iter()
                .filter(|rule| rule.screen.trim() == screen)
                .map(|rule| rule.release_seconds)
                .max()
                .unwrap_or_default();
            if now.duration_since(last_match) < Duration::from_secs(release_seconds) {
                return Some(screen.clone());
            }
        }
        self.active_screen = None;
        self.active_last_match = None;
        config
            .automation
            .default_screen
            .clone()
            .filter(|screen| !screen.trim().is_empty())
    }

    pub fn any_rule_matches(&mut self, config: &AppConfig, sensors: &SensorSnapshot) -> bool {
        if !config.automation.enabled {
            return false;
        }
        self.processes.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing(),
        );
        config
            .automation
            .rules
            .iter()
            .any(|rule| self.matches(rule, sensors))
    }

    pub fn reset(&mut self) {
        self.matched_since.clear();
        self.active_screen = None;
        self.active_last_match = None;
    }

    fn matches(&self, rule: &AutomationRule, sensors: &SensorSnapshot) -> bool {
        if !rule.enabled || rule.screen.trim().is_empty() {
            return false;
        }
        match rule.kind {
            AutomationRuleKind::ProcessRunning => {
                let wanted = normalise_process_name(&rule.process_name);
                !wanted.is_empty()
                    && self.processes.processes().values().any(|process| {
                        normalise_process_name(&process.name().to_string_lossy()) == wanted
                    })
            }
            AutomationRuleKind::GpuTemperatureAbove => sensors
                .gpu_temperature
                .is_some_and(|value| value >= rule.threshold),
            AutomationRuleKind::CpuTemperatureAbove => sensors
                .cpu_temperature
                .is_some_and(|value| value >= rule.threshold),
            AutomationRuleKind::GpuUsageAbove => sensors
                .gpu_usage
                .is_some_and(|value| value >= rule.threshold),
            AutomationRuleKind::CpuUsageAbove => sensors
                .cpu_usage
                .is_some_and(|value| value >= rule.threshold),
            AutomationRuleKind::IdleFor => idle_seconds() >= rule.idle_seconds,
        }
    }
}

fn normalise_process_name(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .trim_end_matches(".exe")
        .to_string()
}

fn idle_seconds() -> u64 {
    unsafe {
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if !GetLastInputInfo(&mut info).as_bool() {
            return 0;
        }
        (GetTickCount64() as u32).wrapping_sub(info.dwTime) as u64 / 1000
    }
}

pub struct Transition {
    from: RgbImage,
    kind: TransitionKind,
    started: Instant,
    duration: Duration,
}

pub fn boot_frame(target: &RgbImage, kind: BootAnimationKind, progress: f32) -> RgbImage {
    let progress = smoothstep(progress.clamp(0.0, 1.0));
    let black = Rgb([0, 0, 0]);
    let mut output = RgbImage::new(target.width(), target.height());
    for y in 0..target.height() {
        for x in 0..target.width() {
            let target_pixel = *target.get_pixel(x, y);
            let pixel = match kind {
                BootAnimationKind::Fade => blend(black, target_pixel, progress),
                BootAnimationKind::Scanlines => {
                    let reveal = ((y as f32 / target.height() as f32) - progress).abs();
                    let line = if y % 4 == 0 { 0.72 } else { 1.0 };
                    let amount = if y as f32 <= target.height() as f32 * progress {
                        line
                    } else if reveal < 0.035 {
                        1.0 - reveal / 0.035
                    } else {
                        0.0
                    };
                    scale(target_pixel, amount)
                }
                BootAnimationKind::Forge => {
                    let cx = target.width() as f32 / 2.0;
                    let cy = target.height() as f32 / 2.0;
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let distance = (dx * dx + dy * dy).sqrt();
                    let max_distance = (cx * cx + cy * cy).sqrt();
                    let ring = (distance / max_distance - progress).abs();
                    let reveal = distance / max_distance <= progress;
                    let pulse = if ring < 0.055 {
                        1.0 - ring / 0.055
                    } else {
                        0.0
                    };
                    let glitch = hash(x / 8, y / 6) as f32 / u32::MAX as f32;
                    let base: f32 = if reveal { 0.92 } else { 0.0 };
                    let glow = pulse * (0.75 + glitch * 0.25);
                    let mut pixel = scale(target_pixel, base.max(glow));
                    if pulse > 0.0 {
                        pixel = blend(pixel, Rgb([100, 216, 203]), pulse * 0.45);
                    }
                    pixel
                }
            };
            output.put_pixel(x, y, pixel);
        }
    }
    output
}

impl Transition {
    pub fn new(from: RgbImage, kind: TransitionKind, duration_ms: u64) -> Option<Self> {
        (kind != TransitionKind::None && duration_ms > 0).then(|| Self {
            from,
            kind,
            started: Instant::now(),
            duration: Duration::from_millis(duration_ms.clamp(100, 3000)),
        })
    }

    pub fn frame(&self, target: &RgbImage) -> (RgbImage, bool) {
        let progress =
            (self.started.elapsed().as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0);
        let frame = transition_frame(&self.from, target, self.kind, progress);
        (frame, progress >= 1.0)
    }
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn transition_frame(
    from: &RgbImage,
    target: &RgbImage,
    kind: TransitionKind,
    progress: f32,
) -> RgbImage {
    if from.dimensions() != target.dimensions() || progress >= 1.0 {
        return target.clone();
    }
    let mut output = RgbImage::new(target.width(), target.height());
    for y in 0..target.height() {
        for x in 0..target.width() {
            let old = *from.get_pixel(x, y);
            let new = *target.get_pixel(x, y);
            let pixel = match kind {
                TransitionKind::None => new,
                TransitionKind::Fade => blend(old, new, progress),
                TransitionKind::Slide => {
                    let edge = (target.width() as f32 * (1.0 - progress)) as u32;
                    if x >= edge {
                        *target.get_pixel(x - edge, y)
                    } else {
                        let source_x = x + target.width() - edge;
                        *from.get_pixel(source_x.min(from.width() - 1), y)
                    }
                }
                TransitionKind::Dissolve => {
                    let noise = hash(x, y) as f32 / u32::MAX as f32;
                    if noise <= progress {
                        new
                    } else {
                        old
                    }
                }
                TransitionKind::Glitch => {
                    let band = hash(y, (progress * 100.0) as u32);
                    let reveal = band as f32 / u32::MAX as f32 <= progress;
                    if reveal {
                        let offset = ((band % 17) as i32 - 8) as i64;
                        let source_x =
                            (x as i64 + offset).clamp(0, target.width() as i64 - 1) as u32;
                        *target.get_pixel(source_x, y)
                    } else {
                        old
                    }
                }
            };
            output.put_pixel(x, y, pixel);
        }
    }
    output
}

fn blend(a: Rgb<u8>, b: Rgb<u8>, amount: f32) -> Rgb<u8> {
    Rgb([
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * amount) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * amount) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * amount) as u8,
    ])
}

fn scale(pixel: Rgb<u8>, amount: f32) -> Rgb<u8> {
    let amount = amount.clamp(0.0, 1.35);
    Rgb([
        (pixel[0] as f32 * amount).clamp(0.0, 255.0) as u8,
        (pixel[1] as f32 * amount).clamp(0.0, 255.0) as u8,
        (pixel[2] as f32 * amount).clamp(0.0, 255.0) as u8,
    ])
}

fn hash(x: u32, y: u32) -> u32 {
    let mut value = x.wrapping_mul(0x45d9f3b) ^ y.wrapping_mul(0x119de1f3);
    value ^= value >> 16;
    value = value.wrapping_mul(0x45d9f3b);
    value ^ (value >> 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_names_ignore_case_and_exe_suffix() {
        assert_eq!(normalise_process_name("CINEBENCH.EXE"), "cinebench");
    }

    #[test]
    fn fade_reaches_target() {
        let from = RgbImage::from_pixel(2, 2, Rgb([0, 0, 0]));
        let target = RgbImage::from_pixel(2, 2, Rgb([255, 100, 50]));
        assert_eq!(
            transition_frame(&from, &target, TransitionKind::Fade, 1.0),
            target
        );
    }

    #[test]
    fn boot_frame_reaches_visible_target_size() {
        let target = RgbImage::from_pixel(8, 6, Rgb([80, 160, 220]));
        let frame = boot_frame(&target, BootAnimationKind::Forge, 0.5);
        assert_eq!(frame.dimensions(), target.dimensions());
        assert!(frame
            .pixels()
            .any(|pixel| pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0));
    }

    #[test]
    fn gpu_rule_selects_screen_without_a_default_screen() {
        let mut config = AppConfig::default();
        config.automation.enabled = true;
        config.automation.default_screen = None;
        config.automation.rules = vec![AutomationRule {
            enabled: true,
            kind: AutomationRuleKind::GpuUsageAbove,
            process_name: String::new(),
            threshold: 80.0,
            idle_seconds: 300,
            sustain_seconds: 0,
            release_seconds: 8,
            screen: "msi".into(),
        }];
        let sensors = SensorSnapshot {
            gpu_usage: Some(90.0),
            ..Default::default()
        };
        assert_eq!(
            RuleEngine::new().target_screen(&config, &sensors),
            Some("msi".into())
        );
    }

    #[test]
    fn reset_requires_a_rule_to_sustain_again() {
        let mut config = AppConfig::default();
        config.automation.enabled = true;
        config.automation.rules = vec![AutomationRule {
            enabled: true,
            kind: AutomationRuleKind::GpuUsageAbove,
            process_name: String::new(),
            threshold: 80.0,
            idle_seconds: 300,
            sustain_seconds: 10,
            release_seconds: 8,
            screen: "msi".into(),
        }];
        let sensors = SensorSnapshot {
            gpu_usage: Some(90.0),
            ..Default::default()
        };
        let mut engine = RuleEngine::new();
        assert_eq!(engine.target_screen(&config, &sensors), None);
        engine.reset();
        assert_eq!(engine.target_screen(&config, &sensors), None);
    }
}
