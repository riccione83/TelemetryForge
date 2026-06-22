#![forbid(unsafe_op_in_unsafe_fn)]

pub const ABI_VERSION: u32 = 1;

#[repr(i32)]
#[derive(Clone, Copy)]
pub enum Sensor {
    CpuUsage = 1,
    CpuTemperature = 2,
    CpuClock = 3,
    FanSpeed = 4,
    GpuUsage = 10,
    GpuTemperature = 11,
    GpuClock = 12,
    GpuPower = 13,
    RamUsage = 20,
    VramUsage = 21,
    Volume = 30,
}

#[link(wasm_import_module = "telemetryforge")]
extern "C" {
    fn sensor(id: i32) -> f32;
    fn animation_ms() -> u32;
    fn clear(colour: u32);
    fn line(x1: f32, y1: f32, x2: f32, y2: f32, width: f32, colour: u32);
    fn circle(cx: f32, cy: f32, radius: f32, width: f32, colour: u32);
    fn fill_circle(cx: f32, cy: f32, radius: f32, colour: u32);
    fn arc(
        cx: f32,
        cy: f32,
        radius: f32,
        width: f32,
        start_degrees: f32,
        sweep_degrees: f32,
        colour: u32,
    );
    fn text(x: f32, y: f32, size: f32, colour: u32, ptr: *const u8, len: usize);
    fn text_center(x: f32, y: f32, size: f32, colour: u32, ptr: *const u8, len: usize);
    fn number_center(x: f32, y: f32, size: f32, colour: u32, value: f32, decimals: i32);
}

pub struct Canvas {
    pub width: f32,
    pub height: f32,
}

impl Canvas {
    pub const fn new(width: i32, height: i32) -> Self {
        Self {
            width: width as f32,
            height: height as f32,
        }
    }

    pub fn sensor(&self, sensor_id: Sensor) -> f32 {
        unsafe { sensor(sensor_id as i32) }
    }

    pub fn animation_ms(&self) -> u32 {
        unsafe { animation_ms() }
    }

    pub fn clear(&self, colour: u32) {
        unsafe { clear(colour) }
    }

    pub fn line(&self, from: (f32, f32), to: (f32, f32), width: f32, colour: u32) {
        unsafe { line(from.0, from.1, to.0, to.1, width, colour) }
    }

    pub fn circle(&self, centre: (f32, f32), radius: f32, width: f32, colour: u32) {
        unsafe { circle(centre.0, centre.1, radius, width, colour) }
    }

    pub fn fill_circle(&self, centre: (f32, f32), radius: f32, colour: u32) {
        unsafe { fill_circle(centre.0, centre.1, radius, colour) }
    }

    pub fn arc(
        &self,
        centre: (f32, f32),
        radius: f32,
        width: f32,
        start_degrees: f32,
        sweep_degrees: f32,
        colour: u32,
    ) {
        unsafe {
            arc(
                centre.0,
                centre.1,
                radius,
                width,
                start_degrees,
                sweep_degrees,
                colour,
            )
        }
    }

    pub fn text(&self, position: (f32, f32), size: f32, colour: u32, value: &str) {
        unsafe {
            text(
                position.0,
                position.1,
                size,
                colour,
                value.as_ptr(),
                value.len(),
            )
        }
    }

    pub fn text_center(&self, position: (f32, f32), size: f32, colour: u32, value: &str) {
        unsafe {
            text_center(
                position.0,
                position.1,
                size,
                colour,
                value.as_ptr(),
                value.len(),
            )
        }
    }

    pub fn number_center(
        &self,
        position: (f32, f32),
        size: f32,
        colour: u32,
        value: f32,
        decimals: i32,
    ) {
        unsafe { number_center(position.0, position.1, size, colour, value, decimals) }
    }
}

pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> u32 {
    u32::from_le_bytes([red, green, blue, alpha])
}
