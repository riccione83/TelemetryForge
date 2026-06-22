(module
  (import "telemetryforge" "sensor" (func $sensor (param i32) (result f32)))
  (import "telemetryforge" "clear" (func $clear (param i32)))
  (import "telemetryforge" "circle"
    (func $circle (param f32 f32 f32 f32 i32)))
  (import "telemetryforge" "arc"
    (func $arc (param f32 f32 f32 f32 f32 f32 i32)))

  (func (export "tf_render") (param $width i32) (param $height i32)
    (local $cx f32)
    (local $cy f32)
    (local $radius f32)
    (local $usage f32)

    (local.set $cx (f32.div (f32.convert_i32_s (local.get $width)) (f32.const 2)))
    (local.set $cy (f32.div (f32.convert_i32_s (local.get $height)) (f32.const 2)))
    (local.set $radius
      (f32.mul
        (f32.min
          (f32.convert_i32_s (local.get $width))
          (f32.convert_i32_s (local.get $height)))
        (f32.const 0.42)))
    (local.set $usage (call $sensor (i32.const 1)))

    (call $clear (i32.const -603191293))
    (call $circle
      (local.get $cx) (local.get $cy) (local.get $radius)
      (f32.const 2) (i32.const -923391961))
    (call $arc
      (local.get $cx) (local.get $cy)
      (f32.sub (local.get $radius) (f32.const 7))
      (f32.const 5) (f32.const -220)
      (f32.mul (f32.const 2.6) (local.get $usage))
      (i32.const -14691805))
  )
)
