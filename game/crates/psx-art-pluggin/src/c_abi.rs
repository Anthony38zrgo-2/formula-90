//! C-ABI surface for integrating the psx-art core with Godot / C++.

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::dither::get_dither_matrix;
use crate::preset::parse_preset_json;
use crate::types::{DitherMatrixType, UpscaleMode};

/// Flat C-compatible representation of a validated retro art preset.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CPsxArtConfig {
    pub internal_width: u32,
    pub internal_height: u32,
    pub upscale_mode: u32, // 0 = PixelPerfect, 1 = SharpBilinear, 2 = CrtSmooth, 3 = NativeHiresPsx
    pub fps_cap: u32,

    pub vertex_snap_distance: f32,
    pub affine_texture_strength: f32,

    pub bit_depth: u32,
    pub dither_matrix_type: u32, // 0 = None, 1 = Bayer2x2, 2 = Bayer4x4, 3 = Bayer8x8
    pub dither_strength: f32,

    pub fog_enabled: bool,
    pub fog_color: [f32; 4],
    pub fog_near: f32,
    pub fog_far: f32,

    pub scanlines_enabled: bool,
    pub scanline_opacity: f32,
    pub composite_bleed: f32,
}

impl Default for CPsxArtConfig {
    fn default() -> Self {
        Self {
            internal_width: 320,
            internal_height: 240,
            upscale_mode: 1,
            fps_cap: 30,
            vertex_snap_distance: 0.35,
            affine_texture_strength: 1.0,
            bit_depth: 5,
            dither_matrix_type: 2,
            dither_strength: 1.0,
            fog_enabled: true,
            fog_color: [0.5, 0.5, 0.5, 0.0],
            fog_near: 10.0,
            fog_far: 40.0,
            scanlines_enabled: false,
            scanline_opacity: 0.0,
            composite_bleed: 0.0,
        }
    }
}

/// Parses and validates a JSON preset string, writing the result into `out_config`.
///
/// Returns:
///  0 = Success
/// -1 = Null pointer
/// -2 = Invalid UTF-8
/// -3 = JSON Parse Error
/// -4 = Validation Error
#[no_mangle]
pub unsafe extern "C" fn psx_art_parse_preset(
    json_ptr: *const c_char,
    out_config: *mut CPsxArtConfig,
) -> i32 {
    if json_ptr.is_null() || out_config.is_null() {
        return -1;
    }

    // SAFETY: `json_ptr` is non-null (checked above) and, per `# Safety`, points to a valid NUL-terminated C string.
    let c_str = unsafe { CStr::from_ptr(json_ptr) };
    let json_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    match parse_preset_json(json_str) {
        Ok(preset) => {
            let upscale_mode_u32 = match preset.display.upscale_mode {
                UpscaleMode::PixelPerfect => 0,
                UpscaleMode::SharpBilinear => 1,
                UpscaleMode::CrtSmooth => 2,
                UpscaleMode::NativeHiresPsx => 3,
            };

            let dither_type_u32 = match preset.color.dither_matrix_type {
                DitherMatrixType::None => 0,
                DitherMatrixType::Bayer2x2 => 1,
                DitherMatrixType::Bayer4x4 => 2,
                DitherMatrixType::Bayer8x8 => 3,
            };

            // SAFETY: `out_config` is non-null (checked above) and, per `# Safety`, points to a writable `CPsxArtConfig`.
            unsafe {
                *out_config = CPsxArtConfig {
                internal_width: preset.display.internal_width,
                internal_height: preset.display.internal_height,
                upscale_mode: upscale_mode_u32,
                fps_cap: preset.display.fps_cap,

                vertex_snap_distance: preset.geometry.vertex_snap_distance,
                affine_texture_strength: preset.geometry.affine_texture_strength,

                bit_depth: preset.color.bit_depth,
                dither_matrix_type: dither_type_u32,
                dither_strength: preset.color.dither_strength,

                fog_enabled: preset.fog.enabled,
                fog_color: preset.fog.color,
                fog_near: preset.fog.near,
                fog_far: preset.fog.far,

                scanlines_enabled: preset.crt_effects.scanlines_enabled,
                scanline_opacity: preset.crt_effects.scanline_opacity,
                composite_bleed: preset.crt_effects.composite_bleed,
                };
            }
            0
        }
        Err(crate::preset::PresetError::JsonParse(_)) => -3,
        Err(crate::preset::PresetError::Validation(_)) => -4,
    }
}

/// Copies normalized dither matrix floats into a user-provided buffer.
///
/// Returns:
///  0 = Success
/// -1 = Null pointer
/// -2 = Buffer too small
#[no_mangle]
pub unsafe extern "C" fn psx_art_get_dither_matrix(
    matrix_type: u32,
    out_buffer: *mut f32,
    max_len: usize,
    out_len: *mut usize,
) -> i32 {
    if out_buffer.is_null() || out_len.is_null() {
        return -1;
    }

    let dither_type = match matrix_type {
        0 => DitherMatrixType::None,
        1 => DitherMatrixType::Bayer2x2,
        2 => DitherMatrixType::Bayer4x4,
        3 => DitherMatrixType::Bayer8x8,
        _ => DitherMatrixType::Bayer4x4,
    };

    let matrix = get_dither_matrix(dither_type);
    if matrix.len() > max_len {
        return -2;
    }

    // SAFETY: both pointers are non-null (checked above) and `matrix.len() <= max_len`
    // (checked above); caller guarantees `out_buffer`/`out_len` are valid for writes.
    unsafe {
        std::ptr::copy_nonoverlapping(matrix.as_ptr(), out_buffer, matrix.len());
        *out_len = matrix.len();
    }
    0
}

/// Returns the library ABI version number (e.g. 1).
#[no_mangle]
pub extern "C" fn psx_art_version() -> u32 {
    1
}
