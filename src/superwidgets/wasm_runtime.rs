use crate::{renderer::fonts, sensors::model::SensorSnapshot};
use anyhow::{Context, Result};
use image::{Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_hollow_circle_mut, draw_line_segment_mut, draw_text_mut,
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
    time::{Instant, UNIX_EPOCH},
};
use wasmi::{Caller, Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

#[derive(Clone)]
struct CachedModule {
    modified: u64,
    engine: Engine,
    module: Module,
}

static MODULE_CACHE: LazyLock<Mutex<HashMap<PathBuf, CachedModule>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static ANIMATION_START: LazyLock<Instant> = LazyLock::new(Instant::now);

struct HostState {
    image: RgbaImage,
    sensors: SensorSnapshot,
    limits: StoreLimits,
}

pub fn render(
    module_path: &Path,
    width: u32,
    height: u32,
    sensors: &SensorSnapshot,
) -> Result<RgbaImage> {
    let cached = cached_module(module_path)?;
    let engine = cached.engine;
    let module = cached.module;
    let mut store = Store::new(
        &engine,
        HostState {
            image: RgbaImage::new(width.max(1), height.max(1)),
            sensors: sensors.clone(),
            limits: StoreLimitsBuilder::new()
                .memory_size(8 * 1024 * 1024)
                .table_elements(1024)
                .build(),
        },
    );
    store.limiter(|state| &mut state.limits);
    store.set_fuel(2_000_000)?;
    let mut linker = Linker::new(&engine);
    define_host_functions(&mut linker)?;
    let instance = linker.instantiate_and_start(&mut store, &module)?;
    instance
        .get_typed_func::<(i32, i32), ()>(&store, "tf_render")?
        .call(&mut store, (width as i32, height as i32))?;
    Ok(store.into_data().image)
}

fn cached_module(path: &Path) -> Result<CachedModule> {
    let modified = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    if let Some(cached) = MODULE_CACHE
        .lock()
        .expect("WASM module cache poisoned")
        .get(path)
        .filter(|cached| cached.modified == modified)
        .cloned()
    {
        return Ok(cached);
    }
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let bytes = fs::read(path).with_context(|| format!("Could not read {}", path.display()))?;
    let module = Module::new(&engine, &bytes[..]).context("Invalid Super Widget WebAssembly")?;
    let cached = CachedModule {
        modified,
        engine,
        module,
    };
    MODULE_CACHE
        .lock()
        .expect("WASM module cache poisoned")
        .insert(path.to_path_buf(), cached.clone());
    Ok(cached)
}

fn define_host_functions(linker: &mut Linker<HostState>) -> Result<()> {
    linker.func_wrap(
        "telemetryforge",
        "sensor",
        |caller: Caller<'_, HostState>, id: i32| sensor_value(&caller.data().sensors, id),
    )?;
    linker.func_wrap("telemetryforge", "animation_ms", || {
        ANIMATION_START.elapsed().as_millis() as i32
    })?;
    linker.func_wrap(
        "telemetryforge",
        "clear",
        |mut caller: Caller<'_, HostState>, colour: i32| {
            let colour = colour_value(colour);
            for pixel in caller.data_mut().image.pixels_mut() {
                *pixel = colour;
            }
        },
    )?;
    linker.func_wrap(
        "telemetryforge",
        "line",
        |mut caller: Caller<'_, HostState>,
         x1: f32,
         y1: f32,
         x2: f32,
         y2: f32,
         width: f32,
         colour: i32| {
            thick_line(
                &mut caller.data_mut().image,
                (x1, y1),
                (x2, y2),
                width,
                colour_value(colour),
            );
        },
    )?;
    linker.func_wrap(
        "telemetryforge",
        "circle",
        |mut caller: Caller<'_, HostState>,
         cx: f32,
         cy: f32,
         radius: f32,
         width: f32,
         colour: i32| {
            let image = &mut caller.data_mut().image;
            for offset in 0..width.max(1.0).round() as i32 {
                draw_hollow_circle_mut(
                    image,
                    (cx.round() as i32, cy.round() as i32),
                    (radius - offset as f32).max(1.0).round() as i32,
                    colour_value(colour),
                );
            }
        },
    )?;
    linker.func_wrap(
        "telemetryforge",
        "fill_circle",
        |mut caller: Caller<'_, HostState>, cx: f32, cy: f32, radius: f32, colour: i32| {
            draw_filled_circle_mut(
                &mut caller.data_mut().image,
                (cx.round() as i32, cy.round() as i32),
                radius.max(1.0).round() as i32,
                colour_value(colour),
            );
        },
    )?;
    linker.func_wrap(
        "telemetryforge",
        "arc",
        |mut caller: Caller<'_, HostState>,
         cx: f32,
         cy: f32,
         radius: f32,
         width: f32,
         start: f32,
         sweep: f32,
         colour: i32| {
            draw_arc(
                &mut caller.data_mut().image,
                (cx, cy),
                radius,
                width,
                start,
                sweep,
                colour_value(colour),
            );
        },
    )?;
    linker.func_wrap(
        "telemetryforge",
        "text",
        |mut caller: Caller<'_, HostState>,
         x: f32,
         y: f32,
         size: f32,
         colour: i32,
         pointer: i32,
         length: i32| {
            let Some(memory) = caller
                .get_export("memory")
                .and_then(|item| item.into_memory())
            else {
                return;
            };
            let length = length.clamp(0, 4096) as usize;
            let mut bytes = vec![0; length];
            if memory
                .read(&caller, pointer.max(0) as usize, &mut bytes)
                .is_err()
            {
                return;
            }
            let Ok(value) = std::str::from_utf8(&bytes) else {
                return;
            };
            let Ok(font) = fonts::load("Bahnschrift") else {
                return;
            };
            draw_text_mut(
                &mut caller.data_mut().image,
                colour_value(colour),
                x.round() as i32,
                y.round() as i32,
                size.max(4.0),
                &font,
                value,
            );
        },
    )?;
    linker.func_wrap(
        "telemetryforge",
        "text_center",
        |mut caller: Caller<'_, HostState>,
         x: f32,
         y: f32,
         size: f32,
         colour: i32,
         pointer: i32,
         length: i32| {
            let Some(value) = guest_text(&caller, pointer, length) else {
                return;
            };
            draw_centered_text(
                &mut caller.data_mut().image,
                x,
                y,
                size,
                colour_value(colour),
                &value,
            );
        },
    )?;
    linker.func_wrap(
        "telemetryforge",
        "number_center",
        |mut caller: Caller<'_, HostState>,
         x: f32,
         y: f32,
         size: f32,
         colour: i32,
         value: f32,
         decimals: i32| {
            let value = match decimals.clamp(0, 3) {
                0 => format!("{value:.0}"),
                1 => format!("{value:.1}"),
                2 => format!("{value:.2}"),
                _ => format!("{value:.3}"),
            };
            draw_centered_text(
                &mut caller.data_mut().image,
                x,
                y,
                size,
                colour_value(colour),
                &value,
            );
        },
    )?;
    Ok(())
}

fn guest_text(caller: &Caller<'_, HostState>, pointer: i32, length: i32) -> Option<String> {
    let memory = caller.get_export("memory")?.into_memory()?;
    let length = length.clamp(0, 4096) as usize;
    let mut bytes = vec![0; length];
    memory
        .read(caller, pointer.max(0) as usize, &mut bytes)
        .ok()?;
    String::from_utf8(bytes).ok()
}

fn draw_centered_text(
    image: &mut RgbaImage,
    x: f32,
    y: f32,
    size: f32,
    colour: Rgba<u8>,
    value: &str,
) {
    use ab_glyph::{Font, ScaleFont};
    let Ok(font) = fonts::load("Bahnschrift") else {
        return;
    };
    let size = size.max(4.0);
    let scaled = font.as_scaled(size);
    let width: f32 = value
        .chars()
        .map(|character| scaled.h_advance(scaled.glyph_id(character)))
        .sum();
    draw_text_mut(
        image,
        colour,
        (x - width / 2.0).round() as i32,
        y.round() as i32,
        size,
        &font,
        value,
    );
}

fn sensor_value(sensors: &SensorSnapshot, id: i32) -> f32 {
    match id {
        1 => sensors.cpu_usage,
        2 => sensors.cpu_temperature,
        3 => sensors.cpu_clock,
        4 => sensors.fan_speed,
        10 => sensors.gpu_usage,
        11 => sensors.gpu_temperature,
        12 => sensors.gpu_clock,
        13 => sensors.gpu_power,
        20 => sensors.ram_usage,
        21 => sensors.vram_usage,
        30 => sensors.system_volume,
        _ => None,
    }
    .unwrap_or_default()
}

fn colour_value(value: i32) -> Rgba<u8> {
    Rgba((value as u32).to_le_bytes())
}

fn thick_line(
    image: &mut RgbaImage,
    from: (f32, f32),
    to: (f32, f32),
    width: f32,
    colour: Rgba<u8>,
) {
    let width = width.max(1.0).round() as i32;
    for offset in -(width / 2)..=(width / 2) {
        draw_line_segment_mut(
            image,
            (from.0 + offset as f32, from.1),
            (to.0 + offset as f32, to.1),
            colour,
        );
    }
}

fn draw_arc(
    image: &mut RgbaImage,
    centre: (f32, f32),
    radius: f32,
    width: f32,
    start: f32,
    sweep: f32,
    colour: Rgba<u8>,
) {
    let steps = (radius.abs() * sweep.abs().to_radians()).max(24.0) as usize;
    for step in 1..=steps {
        let previous = (step - 1) as f32 / steps as f32;
        let current = step as f32 / steps as f32;
        let from_angle = (start + sweep * previous).to_radians();
        let to_angle = (start + sweep * current).to_radians();
        thick_line(
            image,
            (
                centre.0 + radius * from_angle.cos(),
                centre.1 + radius * from_angle.sin(),
            ),
            (
                centre.0 + radius * to_angle.cos(),
                centre.1 + radius * to_angle.sin(),
            ),
            width,
            colour,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_sandboxed_wasm_widget() {
        let path = std::env::temp_dir().join("telemetryforge-sdk-test.wat");
        fs::write(
            &path,
            r#"(module
                (import "telemetryforge" "sensor" (func $sensor (param i32) (result f32)))
                (import "telemetryforge" "clear" (func $clear (param i32)))
                (import "telemetryforge" "fill_circle"
                    (func $fill_circle (param f32 f32 f32 i32)))
                (func (export "tf_render") (param i32 i32)
                    (call $clear (i32.const -16777216))
                    (call $fill_circle
                        (f32.const 20) (f32.const 20)
                        (call $sensor (i32.const 1))
                        (i32.const -16711681)))
            )"#,
        )
        .unwrap();
        let sensors = SensorSnapshot {
            cpu_usage: Some(8.0),
            ..Default::default()
        };
        let image = render(&path, 40, 40, &sensors).unwrap();
        assert_eq!(image.dimensions(), (40, 40));
        assert!(image.pixels().any(|pixel| pixel[1] > 0));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn renders_reactor_core_example() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("sdk/examples/reactor-core/widget.wat");
        let sensors = SensorSnapshot {
            cpu_usage: Some(68.0),
            cpu_temperature: Some(64.0),
            cpu_clock: Some(5175.0),
            gpu_usage: Some(91.0),
            gpu_temperature: Some(72.0),
            gpu_clock: Some(2760.0),
            gpu_power: Some(287.0),
            ram_usage: Some(43.0),
            ..Default::default()
        };
        let image = render(&path, 300, 300, &sensors).unwrap();
        assert_eq!(image.dimensions(), (300, 300));
        assert!(image.pixels().any(|pixel| pixel[0] > 240 && pixel[2] > 180));
    }
}
