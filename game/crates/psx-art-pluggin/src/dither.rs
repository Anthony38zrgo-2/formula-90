//! Bayer dither matrix generation and LUT helpers.

use crate::types::DitherMatrixType;

/// 2x2 Bayer Matrix (normalized 0.0 to 1.0).
pub const BAYER_2X2: [f32; 4] = [
    0.0 / 4.0, 2.0 / 4.0,
    3.0 / 4.0, 1.0 / 4.0,
];

/// 4x4 Bayer Matrix matching authentic PSX GPU hardware (normalized 0.0 to 1.0).
pub const BAYER_4X4: [f32; 16] = [
    0.0 / 16.0,  8.0 / 16.0,  2.0 / 16.0, 10.0 / 16.0,
   12.0 / 16.0,  4.0 / 16.0, 14.0 / 16.0,  6.0 / 16.0,
    3.0 / 16.0, 11.0 / 16.0,  1.0 / 16.0,  9.0 / 16.0,
   15.0 / 16.0,  7.0 / 16.0, 13.0 / 16.0,  5.0 / 16.0,
];

/// Computes an 8x8 Bayer matrix dynamically using the recursive block formula.
pub fn generate_bayer_8x8() -> [f32; 64] {
    let mut matrix = [0.0f32; 64];
    for y in 0..8usize {
        for x in 0..8usize {
            let (sub_y, sub_x) = (y % 4, x % 4);
            let base_val = (BAYER_4X4[sub_y * 4 + sub_x] * 16.0) as u32;
            let quadrant_offset = match (y >= 4, x >= 4) {
                (false, false) => 0, // Top-left: 4 * M4
                (false, true) => 2,  // Top-right: 4 * M4 + 2
                (true, false) => 3,  // Bottom-left: 4 * M4 + 3
                (true, true) => 1,   // Bottom-right: 4 * M4 + 1
            };
            let val = 4 * base_val + quadrant_offset;
            matrix[y * 8 + x] = (val as f32) / 64.0;
        }
    }
    matrix
}

/// Returns the normalized dither matrix values for a given pattern type.
pub fn get_dither_matrix(dither_type: DitherMatrixType) -> Vec<f32> {
    match dither_type {
        DitherMatrixType::None => vec![0.0],
        DitherMatrixType::Bayer2x2 => BAYER_2X2.to_vec(),
        DitherMatrixType::Bayer4x4 => BAYER_4X4.to_vec(),
        DitherMatrixType::Bayer8x8 => generate_bayer_8x8().to_vec(),
    }
}
