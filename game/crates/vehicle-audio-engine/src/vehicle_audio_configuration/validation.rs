use super::document::VehicleAudioDocument;

pub const DEFAULT_CONTINUOUS_SOURCE: &str = "legacy";
pub const SUPPORTED_CONTINUOUS_SOURCES: [&str; 3] =
    ["legacy", "v10_gf509", "grand_prix_sampler"];

pub const V10_GF509_KNOWN_KEYS: [&str; 20] = [
    "diagnostic_mode",
    "fallback_to_legacy_on_initialization_error",
    "gain",
    "manifest",
    "schema_version",
    "engine_mode",
    "sampled_engine_gain",
    "sampled_gearbox_gain",
    "v10_filter",
    "gearbox_filter",
    "disable_sample_rasp",
    "residual_gain_scale",
    "scene_gains",
    "cover_radiation_lowpass_hz",
    "upper_mid_shelf_gain",
    "transmission",
    "gear_shift_gain",
    "gear_shift_reference_rpm",
    "sample_blend_weight",
    "sample_zone_trim_db",
];

pub fn validate(document: &VehicleAudioDocument) -> Result<(), String> {
    let continuous_source = document
        .continuous_source
        .as_deref()
        .unwrap_or(DEFAULT_CONTINUOUS_SOURCE);
    if !SUPPORTED_CONTINUOUS_SOURCES.contains(&continuous_source) {
        return Err(format!(
            "unsupported continuous_source {continuous_source:?} (supported: {SUPPORTED_CONTINUOUS_SOURCES:?})"
        ));
    }
    if let Some(cpp_dsp) = &document.cpp_dsp {
        if !cpp_dsp.gain.is_finite() || cpp_dsp.gain < 0.0 {
            return Err("cpp_dsp.gain must be finite and non-negative".to_string());
        }
    }
    if continuous_source == "grand_prix_sampler" && document.grand_prix_sampler.is_none() {
        return Err(
            "continuous_source grand_prix_sampler requires the grand_prix_sampler section"
                .to_string(),
        );
    }
    if continuous_source == "v10_gf509" && document.gf509.is_none() {
        return Err("continuous_source v10_gf509 requires the gf509 section".to_string());
    }
    if let Some(grand_prix_sampler) = &document.grand_prix_sampler {
        validate_manifest_reference(&grand_prix_sampler.manifest, "grand_prix_sampler.manifest")?;
        if grand_prix_sampler.schema_version != 1 {
            return Err(format!(
                "unsupported grand_prix_sampler.schema_version {} (supported: 1)",
                grand_prix_sampler.schema_version
            ));
        }
        for (name, value) in [
            ("engine_gain", grand_prix_sampler.engine_gain),
            ("gearbox_gain", grand_prix_sampler.gearbox_gain),
            ("gearbox_whine_gain", grand_prix_sampler.gearbox_whine_gain),
            (
                "gearbox_whine_tone_gain",
                grand_prix_sampler.gearbox_whine_tone_gain,
            ),
            (
                "gearbox_whine_noise_gain",
                grand_prix_sampler.gearbox_whine_noise_gain,
            ),
            ("backfire_gain", grand_prix_sampler.backfire_gain),
            ("limiter_gain", grand_prix_sampler.limiter_gain),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!(
                    "grand_prix_sampler.{name} must be finite and non-negative"
                ));
            }
        }
        if let Some(coast_gain) = grand_prix_sampler.coast_gain {
            if !coast_gain.is_finite() || coast_gain < 0.0 {
                return Err(
                    "grand_prix_sampler.coast_gain must be finite and non-negative".to_string(),
                );
            }
        }
    }
    if let Some(gf509) = &document.gf509 {
        validate_gf509(gf509)?;
    }
    if let Some(powertrain_synthesis) = &document.powertrain_synthesis {
        let wrapped = serde_json::json!({ "audio": { "powertrain_synthesis": powertrain_synthesis } });
        match crate::powertrain::from_profile_json(&wrapped.to_string()) {
            Ok(_) => {}
            Err(error) => return Err(format!("powertrain_synthesis is invalid: {error}")),
        }
    }
    Ok(())
}

fn validate_gf509(gf509: &serde_json::Value) -> Result<(), String> {
    let object = gf509
        .as_object()
        .ok_or_else(|| "gf509 must be a JSON object".to_string())?;
    for key in object.keys() {
        if !V10_GF509_KNOWN_KEYS.contains(&key.as_str()) {
            return Err(format!("gf509.{key} is not a known key"));
        }
    }
    if let Some(manifest) = object.get("manifest") {
        let manifest = manifest
            .as_str()
            .ok_or_else(|| "gf509.manifest must be a string".to_string())?;
        validate_manifest_reference(manifest, "gf509.manifest")?;
    }
    if let Some(schema_version) = object.get("schema_version") {
        let schema_version = schema_version
            .as_u64()
            .ok_or_else(|| "gf509.schema_version must be an integer".to_string())?;
        if schema_version != 1 && schema_version != 2 {
            return Err(format!(
                "unsupported gf509.schema_version {schema_version} (supported: 1, 2)"
            ));
        }
    }
    if let Some(gain) = object.get("gain") {
        let gain = gain
            .as_f64()
            .ok_or_else(|| "gf509.gain must be a number".to_string())?;
        if !gain.is_finite() || gain < 0.0 {
            return Err("gf509.gain must be finite and non-negative".to_string());
        }
    }
    Ok(())
}

fn validate_manifest_reference(manifest: &str, field_name: &str) -> Result<(), String> {
    if manifest.trim().is_empty() {
        return Err(format!("{field_name} must not be empty"));
    }
    if manifest.contains("://") {
        return Err(format!("{field_name} must be a relative filesystem path"));
    }
    Ok(())
}
