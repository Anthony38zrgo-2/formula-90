use psx_art_plugin::{
    generate_bayer_8x8, get_dither_matrix, parse_preset_json, psx_art_get_dither_matrix,
    psx_art_parse_preset, to_preset_json, CPsxArtConfig, ColorProfile, CrtProfile, DisplayProfile,
    DitherMatrixType, FogProfile, GeometryProfile, PsxArtPreset, UpscaleMode,
    BAYER_2X2, BAYER_4X4,
};
use std::ffi::CString;

#[test]
fn test_default_preset_serialization_roundtrip() {
    let preset = PsxArtPreset {
        profile_name: "Test Profile".into(),
        display: DisplayProfile {
            internal_width: 320,
            internal_height: 240,
            upscale_mode: UpscaleMode::SharpBilinear,
            fps_cap: 30,
        },
        geometry: GeometryProfile {
            vertex_snap_distance: 0.35,
            affine_texture_strength: 1.0,
        },
        color: ColorProfile {
            bit_depth: 5,
            dither_matrix_type: DitherMatrixType::Bayer4x4,
            dither_strength: 1.0,
        },
        fog: FogProfile {
            enabled: true,
            color: [0.1, 0.2, 0.3, 1.0],
            near: 10.0,
            far: 50.0,
        },
        crt_effects: CrtProfile {
            scanlines_enabled: true,
            scanline_opacity: 0.15,
            composite_bleed: 0.02,
        },
    };

    let json = to_preset_json(&preset).expect("Serialization failed");
    let deserialized = parse_preset_json(&json).expect("Deserialization failed");
    assert_eq!(preset, deserialized);
}

#[test]
fn test_validation_errors() {
    let mut preset = PsxArtPreset {
        profile_name: "".into(), // invalid
        display: DisplayProfile::default(),
        geometry: GeometryProfile::default(),
        color: ColorProfile::default(),
        fog: FogProfile::default(),
        crt_effects: CrtProfile::default(),
    };

    assert_eq!(
        parse_preset_json(&serde_json::to_string(&preset).unwrap()).unwrap_err().to_string(),
        "Validation failed: Profile name cannot be empty"
    );

    preset.profile_name = "Valid".into();
    preset.color.bit_depth = 10; // invalid > 8
    assert_eq!(
        parse_preset_json(&serde_json::to_string(&preset).unwrap()).unwrap_err().to_string(),
        "Validation failed: Bit depth must be between 1 and 8 (got 10)"
    );

    preset.color.bit_depth = 5;
    preset.fog.enabled = true;
    preset.fog.near = 50.0;
    preset.fog.far = 10.0; // near >= far invalid
    assert_eq!(
        parse_preset_json(&serde_json::to_string(&preset).unwrap()).unwrap_err().to_string(),
        "Validation failed: Fog far distance (10) must be greater than near distance (50)"
    );
}

#[test]
fn test_dither_matrix_generation() {
    let bayer2 = get_dither_matrix(DitherMatrixType::Bayer2x2);
    assert_eq!(bayer2.len(), 4);
    assert_eq!(bayer2, BAYER_2X2);

    let bayer4 = get_dither_matrix(DitherMatrixType::Bayer4x4);
    assert_eq!(bayer4.len(), 16);
    assert_eq!(bayer4, BAYER_4X4);

    let bayer8 = generate_bayer_8x8();
    assert_eq!(bayer8.len(), 64);
    assert_eq!(bayer8[0], 0.0);
    assert_eq!(bayer8[56], 63.0 / 64.0);
    // Verify all 64 fractions k/64 for k in 0..64 are present
    for k in 0..64 {
        let target = (k as f32) / 64.0;
        assert!(bayer8.iter().any(|&v| (v - target).abs() < 1e-6));
    }
}

#[test]
fn test_c_abi_parse_preset() {
    let json = r#"{
        "profile_name": "Authentic PSX",
        "display": {
            "internal_width": 320,
            "internal_height": 240,
            "upscale_mode": "sharp_bilinear",
            "fps_cap": 30
        },
        "geometry": {
            "vertex_snap_distance": 0.4,
            "affine_texture_strength": 0.85
        },
        "color": {
            "bit_depth": 5,
            "dither_matrix_type": "bayer4x4",
            "dither_strength": 1.0
        },
        "fog": {
            "enabled": true,
            "color": [0.5, 0.5, 0.5, 1.0],
            "near": 10.0,
            "far": 60.0
        },
        "crt_effects": {
            "scanlines_enabled": false,
            "scanline_opacity": 0.0,
            "composite_bleed": 0.0
        }
    }"#;

    let c_json = CString::new(json).unwrap();
    let mut config = CPsxArtConfig::default();

    let res = unsafe { psx_art_parse_preset(c_json.as_ptr(), &mut config) };
    assert_eq!(res, 0);
    assert_eq!(config.internal_width, 320);
    assert_eq!(config.internal_height, 240);
    assert_eq!(config.upscale_mode, 1);
    assert_eq!(config.fps_cap, 30);
    assert_eq!(config.bit_depth, 5);
    assert_eq!(config.dither_matrix_type, 2);
    assert_eq!(config.vertex_snap_distance, 0.4);
    assert_eq!(config.affine_texture_strength, 0.85);
    assert!(config.fog_enabled);
    assert_eq!(config.fog_near, 10.0);
    assert_eq!(config.fog_far, 60.0);
}

#[test]
fn test_c_abi_get_dither_matrix() {
    let mut buf = [0.0f32; 16];
    let mut len = 0usize;

    let res = unsafe { psx_art_get_dither_matrix(2, buf.as_mut_ptr(), buf.len(), &mut len) };
    assert_eq!(res, 0);
    assert_eq!(len, 16);
    assert_eq!(buf, BAYER_4X4);
}
