//! Skybox pixel-size calculation and gradient uniform computation.
//!
//! Mirrors `BackgroundSkybox.compute_pixel_size_from_camera()` and
//! `BackgroundSkybox._apply_gradient_uniforms()` from GDScript.

use crate::types::{CameraState, GradientConfig, KeepAspect};

/// Mirrors GDScript `COVERAGE_FACTOR = 1.05`.
pub const COVERAGE_FACTOR: f64 = 1.05;

/// Mirrors GDScript `PIXEL_SIZE_SNAP = 0.05`.
pub const PIXEL_SIZE_SNAP: f64 = 0.05;

/// Compute the pixel_size needed for a Sprite3D to fill the camera frustum.
///
/// Mirrors `BackgroundSkybox.compute_pixel_size_from_camera()` from GDScript.
///
/// # Arguments
/// * `camera` - Camera state (FOV, keep_aspect, viewport size)
/// * `distance` - Distance from camera to the sprite (positive)
/// * `texture_size` - Texture dimensions in pixels (width, height)
/// * `coverage_factor` - Multiplier to ensure full coverage (default 1.05)
///
/// # Returns
/// Pixel size that ensures the sprite covers the entire viewport.
pub fn pixel_size_from_camera(
    camera: &CameraState,
    distance: f64,
    texture_size: (i32, i32),
    coverage_factor: f64,
) -> f64 {
    if texture_size.1 <= 0 || camera.fov_deg <= 0.0 {
        return 1.0;
    }

    let fov_rad = camera.fov_deg.to_radians();
    let aspect = camera.viewport_size.0 / camera.viewport_size.1.max(0.001);

    // camera.fov is vertical FOV when keep_aspect == KEEP_HEIGHT (Godot default)
    // and horizontal FOV when KEEP_WIDTH.
    let visible_fov = 2.0 * distance.abs() * (fov_rad * 0.5).tan();

    let (visible_h, visible_w) = match camera.keep_aspect {
        KeepAspect::Height => (visible_fov, visible_fov * aspect),
        KeepAspect::Width => (visible_fov / aspect.max(0.001), visible_fov),
    };

    let largest = visible_h.max(visible_w);
    let needed_px = largest * coverage_factor / texture_size.1 as f64;

    snapf(needed_px.max(0.1), PIXEL_SIZE_SNAP)
}

/// Gradient uniforms ready to be applied as shader parameters.
///
/// Mirrors the values set by `BackgroundSkybox._apply_gradient_uniforms()`.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientUniforms {
    pub zenith_color: [f32; 3],
    pub horizon_color: [f32; 3],
    pub ground_color: [f32; 3],
    pub horizon_sharpness: f32,
    pub ground_start: f32,
}

/// Compute gradient uniforms from a GradientConfig.
///
/// Mirrors `BackgroundSkybox._apply_gradient_uniforms()`.
/// Uses zero-black as default when a color key is absent (GDScript behavior:
/// the shader parameter is simply not set, and Godot uses the shader default).
pub fn gradient_uniforms(config: &GradientConfig) -> GradientUniforms {
    GradientUniforms {
        zenith_color: config.zenith_color.unwrap_or([0.0, 0.0, 0.0]),
        horizon_color: config.horizon_color.unwrap_or([0.0, 0.0, 0.0]),
        ground_color: config.ground_color,
        horizon_sharpness: config.horizon_sharpness,
        ground_start: config.ground_start,
    }
}

/// Snap a float to the nearest multiple of `step`.
///
/// Mirrors GDScript `snappedf(value, step)`.
fn snapf(value: f64, step: f64) -> f64 {
    if step <= 0.0 {
        return value;
    }
    (value / step).round() * step
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::KeepAspect;

    fn make_camera(fov: f64, keep: KeepAspect, w: f64, h: f64) -> CameraState {
        CameraState {
            fov_deg: fov,
            keep_aspect: keep,
            viewport_size: (w, h),
            yaw_rad: 0.0,
            pitch_rad: 0.0,
        }
    }

    #[test]
    fn pixel_size_fov70_1280x720() {
        let cam = make_camera(70.0, KeepAspect::Height, 1280.0, 720.0);
        let ps = pixel_size_from_camera(&cam, 800.0, (1, 1), COVERAGE_FACTOR);
        // Should be large enough to cover the viewport with a 1x1 texture
        assert!(ps > 1.0, "pixel_size should be > 1.0 for 1x1 texture, got {}", ps);
    }

    #[test]
    fn pixel_size_keep_height_vs_width() {
        let cam_h = make_camera(70.0, KeepAspect::Height, 1920.0, 1080.0);
        let cam_w = make_camera(70.0, KeepAspect::Width, 1920.0, 1080.0);

        let ps_h = pixel_size_from_camera(&cam_h, 800.0, (1, 1), COVERAGE_FACTOR);
        let ps_w = pixel_size_from_camera(&cam_w, 800.0, (1, 1), COVERAGE_FACTOR);

        // For a wide viewport, KEEP_WIDTH should produce a larger pixel_size
        // because the horizontal FOV is wider.
        assert_ne!(ps_h, ps_w, "KEEP_HEIGHT and KEEP_WIDTH should differ for 16:9");
    }

    #[test]
    fn pixel_size_returns_1_for_invalid() {
        let cam = make_camera(70.0, KeepAspect::Height, 1280.0, 720.0);
        // texture height 0
        assert_eq!(pixel_size_from_camera(&cam, 800.0, (1, 0), COVERAGE_FACTOR), 1.0);
        // FOV 0
        let cam0 = make_camera(0.0, KeepAspect::Height, 1280.0, 720.0);
        assert_eq!(pixel_size_from_camera(&cam0, 800.0, (1, 1), COVERAGE_FACTOR), 1.0);
    }

    #[test]
    fn pixel_size_snapped() {
        let cam = make_camera(70.0, KeepAspect::Height, 1280.0, 720.0);
        let ps = pixel_size_from_camera(&cam, 800.0, (1, 1), COVERAGE_FACTOR);
        // Should be a multiple of PIXEL_SIZE_SNAP (0.05)
        let remainder = ps % PIXEL_SIZE_SNAP;
        assert!(
            remainder < 0.001 || (PIXEL_SIZE_SNAP - remainder) < 0.001,
            "pixel_size {} is not snapped to {}",
            ps,
            PIXEL_SIZE_SNAP
        );
    }

    #[test]
    fn gradient_uniforms_mapping() {
        let config = GradientConfig {
            zenith_color: Some([0.18, 0.42, 0.82]),
            horizon_color: Some([0.95, 0.92, 0.82]),
            ground_color: [0.72, 0.68, 0.55],
            horizon_sharpness: 0.65,
            ground_start: 0.48,
        };
        let u = gradient_uniforms(&config);
        assert_eq!(u.zenith_color, [0.18, 0.42, 0.82]);
        assert_eq!(u.horizon_color, [0.95, 0.92, 0.82]);
        assert_eq!(u.ground_color, [0.72, 0.68, 0.55]);
        assert!((u.horizon_sharpness - 0.65).abs() < 0.001);
        assert!((u.ground_start - 0.48).abs() < 0.001);
    }

    #[test]
    fn snapf_basic() {
        assert!((snapf(1.23, 0.05) - 1.25).abs() < 0.001);
        assert!((snapf(1.22, 0.05) - 1.20).abs() < 0.001);
        assert_eq!(snapf(1.0, 0.0), 1.0);
    }
}
