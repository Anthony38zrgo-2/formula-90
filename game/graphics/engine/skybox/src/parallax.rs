//! Parallax math, pixel snapping, modular wrapping, and tiling offsets.
//!
//! Mirrors `BackgroundLayerInstance.update_parallax()`, `_apply_pixel_snap()`,
//! and `_create_tile_copies()` from GDScript.

use crate::types::{CameraState, LayerConfig, Scale2};

/// Compute the parallax offset for a layer given camera state and sprite width.
///
/// Returns `(x, y)` position in local camera space.
/// Mirrors `BackgroundLayerInstance.update_parallax()`.
///
/// The formula:
/// ```text
/// radius = abs(distance_z)
/// raw_x = -(yaw_rad * parallax_x * radius) + offset_x
/// raw_y = (pitch_rad * parallax_y * radius) + offset_y
/// if repeat_x: raw_x = modular_wrap(raw_x, sprite_w)
/// return pixel_snap((raw_x, raw_y), pixel_size, scale)
/// ```
pub fn compute_parallax(
    layer: &LayerConfig,
    camera: &CameraState,
    sprite_w: f64,
) -> (f64, f64) {
    let radius = layer.distance_z.unwrap_or(-800.0).abs();
    // Negative: when camera turns right (yaw+), mountains shift left in camera-local space.
    let raw_x = -(camera.yaw_rad * layer.parallax_x * radius) + layer.offset_x;
    let raw_y = (camera.pitch_rad * layer.parallax_y * radius) + layer.offset_y;

    let (mut x, y) = if layer.repeat_x && sprite_w > 0.0 {
        (modular_wrap(raw_x, sprite_w), raw_y)
    } else {
        (raw_x, raw_y)
    };

    // Apply pixel snap
    let (snapped_x, snapped_y) = pixel_snap((x, y), layer.pixel_size, layer.scale);
    x = snapped_x;
    (x, snapped_y)
}

/// Pixel snap: round to nearest grid step.
///
/// Mirrors `BackgroundLayerInstance._apply_pixel_snap()`.
/// ```text
/// step = pixel_size * scale
/// value = round(value / step) * step
/// ```
pub fn pixel_snap(value: (f64, f64), pixel_size: f64, scale: Scale2) -> (f64, f64) {
    let step_x = pixel_size * scale.x;
    let step_y = pixel_size * scale.y;

    let x = if step_x > 0.0001 {
        (value.0 / step_x).round() * step_x
    } else {
        value.0
    };
    let y = if step_y > 0.0001 {
        (value.1 / step_y).round() * step_y
    } else {
        value.1
    };

    (x, y)
}

/// Modular wrap for continuous horizontal tiling.
///
/// Mirrors GDScript `fposmod(x + width/2, width) - width/2`.
/// Ensures the value is in `[-width/2, +width/2)`.
pub fn modular_wrap(x: f64, width: f64) -> f64 {
    if width <= 0.0 {
        return x;
    }
    let half = width * 0.5;
    let wrapped = ((x + half) % width + width) % width - half;
    wrapped
}

/// Compute sprite world-space width from texture pixel width and pixel_size.
///
/// Mirrors GDScript: `texture.get_width() * pixel_size`.
pub fn sprite_width(texture_pixel_width: i32, pixel_size: f64) -> f64 {
    texture_pixel_width as f64 * pixel_size
}

/// Compute tile copy offsets for horizontal tiling.
///
/// Returns `[-width, +width]` when repeat_x is active.
/// Mirrors `BackgroundLayerInstance._create_tile_copies()`.
pub fn tile_offsets(sprite_w: f64) -> Vec<f64> {
    if sprite_w > 0.0 {
        vec![-sprite_w, sprite_w]
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Scale2;

    fn make_layer(
        parallax_x: f64,
        parallax_y: f64,
        distance_z: f64,
        offset_x: f64,
        offset_y: f64,
        repeat_x: bool,
    ) -> LayerConfig {
        LayerConfig {
            id: "test".to_string(),
            texture_path: String::new(),
            depth: 0,
            parallax_x,
            parallax_y,
            scale: Scale2 { x: 1.0, y: 1.0 },
            offset_x,
            offset_y,
            repeat_x,
            repeat_y: false,
            pixel_snap: false, // disable for raw math tests
            distance_z: Some(distance_z),
            pixel_size: 1.0,
            procedural: false,
            shader_path: String::new(),
            uniforms: Default::default(),
        }
    }

    fn make_camera(yaw: f64, pitch: f64) -> CameraState {
        CameraState {
            fov_deg: 70.0,
            keep_aspect: crate::types::KeepAspect::Height,
            viewport_size: (1280.0, 720.0),
            yaw_rad: yaw,
            pitch_rad: pitch,
        }
    }

    #[test]
    fn parallax_at_zero_yaw() {
        let layer = make_layer(0.08, 0.01, -700.0, 0.0, 0.0, false);
        let camera = make_camera(0.0, 0.0);
        let (x, y) = compute_parallax(&layer, &camera, 0.0);
        assert!((x - 0.0).abs() < 0.001, "x should be 0 at zero yaw, got {}", x);
        assert!((y - 0.0).abs() < 0.001, "y should be 0 at zero pitch, got {}", y);
    }

    #[test]
    fn parallax_hierarchy_at_30_deg() {
        // Far layer: parallax_x = 0.08, distance = -700
        let far = make_layer(0.08, 0.01, -700.0, 0.0, 0.0, false);
        // Near layer: parallax_x = 0.18, distance = -600
        let near = make_layer(0.18, 0.02, -600.0, 0.0, 0.0, false);
        // Clouds: parallax_x = 0.28, distance = -500
        let clouds = make_layer(0.28, 0.0, -500.0, 0.0, 0.0, false);

        let camera = make_camera(0.5236, 0.0); // ~30 degrees

        let (far_x, _) = compute_parallax(&far, &camera, 0.0);
        let (near_x, _) = compute_parallax(&near, &camera, 0.0);
        let (clouds_x, _) = compute_parallax(&clouds, &camera, 0.0);

        let abs_far = far_x.abs();
        let abs_near = near_x.abs();
        let abs_clouds = clouds_x.abs();

        assert!(
            abs_far < abs_near && abs_near < abs_clouds,
            "Parallax hierarchy violated: far={:.2}, near={:.2}, clouds={:.2}",
            abs_far,
            abs_near,
            abs_clouds
        );
    }

    #[test]
    fn modular_wrap_continuity() {
        let width = 1280.0;
        let half = width * 0.5;

        // Values near the boundary should wrap to opposite sides
        let v1 = modular_wrap(width * 0.49, width);
        let v2 = modular_wrap(width * 0.51, width);
        // v1 should be near +half, v2 should be near -half
        assert!(v1 > 0.0, "v1 should be positive: {}", v1);
        assert!(v2 < 0.0, "v2 should be negative: {}", v2);

        // Both should be close to half-width from the boundary
        assert!(
            (v1 - half * 0.98).abs() < 1.0,
            "v1 should be near +half: {}",
            v1
        );
        assert!(
            (v2 + half * 0.98).abs() < 1.0,
            "v2 should be near -half: {}",
            v2
        );

        // Values just inside the boundary should not wrap
        let v3 = modular_wrap(width * 0.4, width);
        assert!(v3 > 0.0 && v3 < half, "v3 should be in range: {}", v3);

        // Wrapping twice should be idempotent
        let v4 = modular_wrap(v1, width);
        assert!((v1 - v4).abs() < 0.001, "Idempotent wrap failed");
    }

    #[test]
    fn modular_wrap_zero_width() {
        assert_eq!(modular_wrap(5.0, 0.0), 5.0);
    }

    #[test]
    fn pixel_snap_consistency() {
        let pixel_size = 1.5;
        let scale = Scale2 { x: 1.0, y: 1.0 };
        let (x, y) = pixel_snap((10.3, 20.7), pixel_size, scale);
        // 10.3 / 1.5 = 6.867 -> round to 7 -> 7 * 1.5 = 10.5
        assert!((x - 10.5).abs() < 0.001, "x snapped: {}", x);
        // 20.7 / 1.5 = 13.8 -> round to 14 -> 14 * 1.5 = 21.0
        assert!((y - 21.0).abs() < 0.001, "y snapped: {}", y);
    }

    #[test]
    fn pixel_snap_tiny_step() {
        // Very small step should not snap (step < 0.0001)
        let (x, _) = pixel_snap((1.23456, 0.0), 0.00001, Scale2 { x: 1.0, y: 1.0 });
        assert_eq!(x, 1.23456);
    }

    #[test]
    fn sprite_width_calculation() {
        assert!((sprite_width(1280, 1.5) - 1920.0).abs() < 0.001);
        assert!((sprite_width(720, 0.5) - 360.0).abs() < 0.001);
    }

    #[test]
    fn tile_offsets_for_positive_width() {
        let offsets = tile_offsets(1920.0);
        assert_eq!(offsets.len(), 2);
        assert!((offsets[0] - (-1920.0)).abs() < 0.001);
        assert!((offsets[1] - 1920.0).abs() < 0.001);
    }

    #[test]
    fn tile_offsets_for_zero_width() {
        let offsets = tile_offsets(0.0);
        assert!(offsets.is_empty());
    }
}
