#!/usr/bin/env python3
"""Build the standalone Grand Prix sampler bank (schema 1) for f1_2030_v10.

Reads the 15 immutable WAV sources in the V10 GP3 source bank, verifies every
SHA-256 against the frozen inventory, prepares five seamless engine loops and
eight one-shot events at 44.1 kHz mono PCM16, calibrates a proportional
reference-RPM ladder over the physical 4500-18000 RPM range, measures pairwise
boundary level compensation and writes:

  game/audio/formula_one_2030_grand_prix_sampler/          runtime bank + manifest.json
  reports/audio-v10/grand-prix-sampler/source_inventory.json
  reports/audio-v10/grand-prix-sampler/calibration.json
  reports/audio-v10/grand-prix-sampler/seam_evidence.json

The build is deterministic: identical sources produce byte-identical outputs.
Run with --check to validate an existing bank without rewriting anything.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import math
import sys
import wave
from fractions import Fraction
from pathlib import Path

import numpy as np
from scipy.signal import butter, resample_poly, sosfiltfilt

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE_DIR = ROOT / "game/sounds/banks/v10-gp3"
DEFAULT_BANK_DIR = ROOT / "game/audio/formula_one_2030_grand_prix_sampler"
DEFAULT_REPORTS_DIR = ROOT / "reports/audio-v10/grand-prix-sampler"

SCHEMA_VERSION = 1
TOOL_REVISION = 5
OUTPUT_SAMPLE_RATE = 44100
EVENT_SELECTION_SEED = 1

PHYSICAL_IDLE_REVOLUTIONS_PER_MINUTE = 4500.0
PHYSICAL_MAX_REVOLUTIONS_PER_MINUTE = 18000.0

LOOP_CROSSFADE_SECONDS = 0.020
TRANSITION_HALF_WIDTH_RATIO = 0.075
TRANSITION_BLEND_LAW = "smoothstep"
TRANSITION_GAIN_LAW = "equal_power"
BOUNDARY_LEVEL_STEP_DB = 1.5
LEVEL_BAND_HZ = (300.0, 6000.0)

SOURCE_INVENTORY = {
    "98_int_idle.wav": {
        "role": "engine_loop",
        "asset_id": "engine_idle_loop",
        "sha256": "f0021afba4048c59ae237325c932abef76433f03c437794d73de0e417384ddea",
    },
    "98_int_low.wav": {
        "role": "engine_loop",
        "asset_id": "engine_low_loop",
        "sha256": "2eac0ef746c818ecac50fe77ca0099894f4762d55428a3cddfeb3007a6092cc5",
    },
    "98_int_med.wav": {
        "role": "engine_loop",
        "asset_id": "engine_medium_loop",
        "sha256": "d1a527d5ddf3ec6d4f38127d1c6a3b00d7b7771d5473f68315e4352c98db4007",
    },
    "98_int_high_1.wav": {
        "role": "engine_loop",
        "asset_id": "engine_high_loop",
        "sha256": "fc68bc279e3f8098be009be24eb4249c4ab76ab805e595af7bc5c509b1413477",
    },
    "98_int_max_5.wav": {
        "role": "engine_loop",
        "asset_id": "engine_maximum_loop",
        "sha256": "f6b6cd4389fee32cb5f4b41a550e7aa146d4a5d0912fda86551dabfd598aa50b",
    },
    "gearup.wav": {
        "role": "gearbox_upshift",
        "asset_id": "upshift_event",
        "sha256": "c2d6c7c8441c10439625dc7dfbcd48d8dd8fd7d1b5004fa80da19ebda64a663a",
        "original_source_sha256": "f656d22c4b161d16e70bda277066246b99e73d83d0dd5833fa5c2d63ae9d0657",
        "preparation_recipe": "prepared_gearbox_body_and_1996_mechanical_composite_v1",
        "already_prepared": True,
    },
    "geardn.wav": {
        "role": "gearbox_downshift",
        "asset_id": "downshift_event",
        "sha256": "52b5d14b33522167caff9508e48aae66bf8c7bcce3e8aac73a71831004cab279",
        "original_source_sha256": "421a7136b22b1624fda39fe9c0540de73b8a9b8c170e328bc44c4db44e09ee3e",
        "preparation_recipe": "prepared_gearbox_body_and_1996_mechanical_composite_v1",
        "already_prepared": True,
    },
    "96_gear_change_up_1.wav": {
        "role": "gearbox_upshift_component",
        "asset_id": "upshift_event_1996_component",
        "sha256": "8332c4815bd4d26378600a5592e93bc764ec667377b912db5a43c04906470a59",
        "component_only": True,
    },
    "96_gear_change_down_2.wav": {
        "role": "gearbox_downshift_component",
        "asset_id": "downshift_event_1996_component",
        "sha256": "efaf45f46722051b7f454e421657cfe72ef7ece3412ae0cb9a7a91099106c032",
        "component_only": True,
    },
    "500_backfire3.wav": {
        "role": "lift_backfire",
        "asset_id": "backfire_burst_3",
        "sha256": "e107717d2fe4f4fccd5d10d910b7cff71cd36d11403ef2a1277d36418986d044",
    },
    "500_backfire4.wav": {
        "role": "lift_backfire",
        "asset_id": "backfire_burst_4",
        "sha256": "e903a055cf8b5ca6a79b44862681f12bca5a11e95878f091095d613e7b63171e",
    },
    "500_backfire5.wav": {
        "role": "lift_backfire",
        "asset_id": "backfire_burst_5",
        "sha256": "a471a47bc8094a38e50894713b47b7847d81e1c518830b804e25dc87a69ceec1",
    },
    "500_backfire6.wav": {
        "role": "lift_backfire",
        "asset_id": "backfire_burst_6",
        "sha256": "6179b1460c81f58ce6ed69dd48498e0efcb3ec66eb18054cf1d97ac444426797",
    },
    "500_backfire7.wav": {
        "role": "lift_backfire",
        "asset_id": "backfire_burst_7",
        "sha256": "ed324b6935950738a6cb4eb0ba99eab74b58bd24fe8b6a9dc75973b47269c42c",
    },
    "500_limiter.wav": {
        "role": "limiter_event",
        "asset_id": "limiter_event",
        "sha256": "1bdb7a2cc3eb6d95e7fa8d56bec55fd4df8c9b307902056b93f5c30b32751758",
    },
}

GEARBOX_EVENT_COMPOSITE_RECIPES = {
    "upshift_event": {
        "layer_source_filename": "96_gear_change_up_1.wav",
        "layer_high_pass_hertz": 160.0,
        "layer_filter_order": 2,
        "layer_gain_decibels": -8.0,
        "layer_offset_seconds": 0.0,
        "layer_edge_fade_seconds": 0.001,
        "true_peak_ceiling_decibels_full_scale": -3.0,
    },
    "downshift_event": {
        "layer_source_filename": "96_gear_change_down_2.wav",
        "layer_high_pass_hertz": 140.0,
        "layer_filter_order": 2,
        "layer_gain_decibels": -12.0,
        "layer_offset_seconds": 0.0,
        "layer_edge_fade_seconds": 0.001,
        "true_peak_ceiling_decibels_full_scale": -3.0,
    },
}

def load_source_inventory(path: Path) -> dict:
    payload = json.loads(path.read_text(encoding="utf-8"))
    table = {}
    for source in payload["sources"]:
        table[source["filename"]] = source
    return table


LOOP_ORDER = [
    "engine_idle_loop",
    "engine_low_loop",
    "engine_medium_loop",
    "engine_high_loop",
    "engine_maximum_loop",
]

EVENT_GROUP_DEFINITIONS = [
    {
        "id": "upshift",
        "role": "gearbox_upshift",
        "variant_asset_ids": ["upshift_event"],
        "selection": "single_variant",
        "voice_limit": 2,
        "minimum_retrigger_seconds": 0.030,
        "timing_offset_seconds": 0.0,
        "trigger_event": "upshift_cut_entry",
    },
    {
        "id": "downshift",
        "role": "gearbox_downshift",
        "variant_asset_ids": ["downshift_event"],
        "selection": "single_variant",
        "voice_limit": 2,
        "minimum_retrigger_seconds": 0.100,
        "timing_offset_seconds": 0.0,
        "trigger_event": "downshift_cut_entry",
    },
    {
        "id": "lift_backfire",
        "role": "lift_backfire",
        "variant_asset_ids": [
            "backfire_burst_3",
            "backfire_burst_4",
            "backfire_burst_5",
            "backfire_burst_6",
            "backfire_burst_7",
        ],
        "selection": "seeded_cycle",
        "voice_limit": 1,
        "minimum_retrigger_seconds": 1.000,
        "timing_offset_seconds": 0.0,
        "trigger_event": "lift_edge",
    },
    {
        "id": "limiter",
        "role": "limiter_event",
        "variant_asset_ids": ["limiter_event"],
        "selection": "single_variant",
        "voice_limit": 1,
        "minimum_retrigger_seconds": 0.250,
        "timing_offset_seconds": 0.0,
        "trigger_event": "limiter_entry_edge",
    },
]

EVENT_TRIGGER_POLICY = {
    "lift_edge": {
        "previous_throttle_min": 0.80,
        "throttle_max": 0.15,
        "revolutions_per_minute_min": 13500.0,
        "cooldown_seconds": 1.0,
    },
    "limiter_entry_edge": {"cooldown_seconds": 0.250},
}


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def read_wav(path: Path) -> tuple[np.ndarray, int, int, int]:
    with wave.open(str(path), "rb") as handle:
        channels = handle.getnchannels()
        width = handle.getsampwidth()
        rate = handle.getframerate()
        frames = handle.getnframes()
        raw = handle.readframes(frames)
    dtype = {1: np.int8, 2: np.int16, 3: np.int32, 4: np.int32}[width]
    data = np.frombuffer(raw, dtype=dtype)
    if channels > 1:
        data = data.reshape(-1, channels).mean(axis=1)
    scale = float(1 << (8 * width - 1))
    return data.astype(np.float64) / scale, rate, channels, width * 8


def wav_bytes(samples: np.ndarray, rate: int) -> bytes:
    clipped = np.clip(samples, -1.0, 32767.0 / 32768.0)
    pcm = np.round(clipped * 32768.0).astype(np.int64)
    pcm = np.clip(pcm, -32768, 32767).astype("<i2")
    buffer = io.BytesIO()
    with wave.open(buffer, "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(rate)
        handle.writeframes(pcm.tobytes())
    return buffer.getvalue()


def rms_value(samples: np.ndarray) -> float:
    if samples.size == 0:
        return 0.0
    return float(np.sqrt(np.mean(samples.astype(np.float64) ** 2)))


def dbfs(value: float) -> float:
    return 20.0 * math.log10(max(value, 1e-12))


def amplitude_from_decibels(decibels: float) -> float:
    return 10.0 ** (decibels / 20.0)


def estimate_true_peak_amplitude(samples: np.ndarray) -> float:
    if samples.size == 0:
        return 0.0
    reconstructed = resample_poly(
        samples,
        8,
        1,
        window=("kaiser", 12.0),
        padtype="line",
    )
    return float(np.max(np.abs(reconstructed)))


def prepare_gearbox_event_composite(
    base_samples: np.ndarray,
    layer_samples: np.ndarray,
    rate: int,
    recipe: dict,
) -> tuple[np.ndarray, dict]:
    centered_layer = layer_samples - float(np.mean(layer_samples))
    filter_sections = butter(
        int(recipe["layer_filter_order"]),
        float(recipe["layer_high_pass_hertz"]),
        btype="highpass",
        fs=rate,
        output="sos",
    )
    filtered_layer = sosfiltfilt(filter_sections, centered_layer)
    edge_fade_frames = min(
        int(round(float(recipe["layer_edge_fade_seconds"]) * rate)),
        filtered_layer.size // 2,
    )
    if edge_fade_frames > 1:
        fade = np.linspace(0.0, 1.0, edge_fade_frames, endpoint=True)
        filtered_layer = filtered_layer.copy()
        filtered_layer[:edge_fade_frames] *= fade
        filtered_layer[-edge_fade_frames:] *= fade[::-1]
    layer_gain = amplitude_from_decibels(float(recipe["layer_gain_decibels"]))
    filtered_layer *= layer_gain
    layer_offset_frames = max(0, int(round(float(recipe["layer_offset_seconds"]) * rate)))
    composite_frames = max(base_samples.size, layer_offset_frames + filtered_layer.size)
    composite = np.zeros(composite_frames, dtype=np.float64)
    composite[:base_samples.size] += base_samples
    composite[layer_offset_frames:layer_offset_frames + filtered_layer.size] += filtered_layer
    true_peak_before_gain = estimate_true_peak_amplitude(composite)
    true_peak_before_gain_decibels = dbfs(true_peak_before_gain)
    ceiling = float(recipe["true_peak_ceiling_decibels_full_scale"])
    final_gain_decibels = min(0.0, ceiling - true_peak_before_gain_decibels)
    composite *= amplitude_from_decibels(final_gain_decibels)
    details = {
        "alignment": "shared_start",
        "base_gain_decibels": 0.0,
        "layer_downmix": "arithmetic_mean_to_mono",
        "layer_direct_current_removal": True,
        "layer_high_pass_hertz": float(recipe["layer_high_pass_hertz"]),
        "layer_filter_order": int(recipe["layer_filter_order"]),
        "layer_gain_decibels": float(recipe["layer_gain_decibels"]),
        "layer_offset_frames": layer_offset_frames,
        "layer_offset_seconds": float(recipe["layer_offset_seconds"]),
        "layer_edge_fade_frames": edge_fade_frames,
        "true_peak_before_final_gain_decibels_full_scale": true_peak_before_gain_decibels,
        "true_peak_ceiling_decibels_full_scale": ceiling,
        "final_gain_decibels": final_gain_decibels,
        "estimated_true_peak_after_final_gain_decibels_full_scale": dbfs(
            estimate_true_peak_amplitude(composite)
        ),
    }
    return composite, details


def band_rms(samples: np.ndarray, rate: int, low_hz: float, high_hz: float) -> float:
    segment = samples[: 1 << int(math.log2(min(len(samples), 1 << 16)))]
    window = np.hanning(len(segment))
    spectrum = np.fft.rfft(segment * window)
    freqs = np.fft.rfftfreq(len(segment), 1.0 / rate)
    mask = (freqs >= low_hz) & (freqs <= high_hz)
    power = (np.abs(spectrum[mask]) ** 2).sum() / (len(segment) ** 2)
    return float(np.sqrt(power * 2.0))


def spectral_peaks(samples: np.ndarray, rate: int, count: int = 16) -> list[float]:
    from scipy.signal import welch

    frequencies, power = welch(samples, fs=rate, nperseg=min(len(samples), 1 << 15), nfft=1 << 17)
    band = (frequencies >= 25.0) & (frequencies <= 1500.0)
    frequencies = frequencies[band]
    power = power[band]
    local_maxima = [
        index
        for index in range(1, len(power) - 1)
        if power[index] >= power[index - 1] and power[index] >= power[index + 1]
    ]
    local_maxima.sort(key=lambda index: power[index], reverse=True)
    picked: list[float] = []
    for index in local_maxima:
        frequency = float(frequencies[index])
        if any(abs(frequency - existing) / existing < 0.03 for existing in picked):
            continue
        picked.append(frequency)
        if len(picked) >= count:
            break
    return picked


def harmonic_comb_spacing(peaks: list[float], minimum_hz: float, maximum_hz: float) -> tuple[float, float, int]:
    strong = peaks[:10]
    for candidate in np.arange(maximum_hz, minimum_hz, -0.01):
        covered = 0
        total_error = 0.0
        for peak in strong:
            harmonic = max(1, int(round(peak / candidate)))
            predicted = harmonic * candidate
            error = abs(peak - predicted) / peak
            if error <= 0.015:
                covered += 1
                total_error += error
        coverage = covered / max(len(strong), 1)
        mean_error = total_error / max(covered, 1)
        if coverage >= 0.8 and mean_error <= 0.01:
            harmonic_span = max(int(round(max(strong) / max(float(candidate), 1e-9))), 1)
            return float(candidate), coverage, harmonic_span
    raise ValueError("no harmonic comb spacing fits the measured peaks")


def loop_level(samples: np.ndarray) -> float:
    margin = len(samples) // 10
    core = samples[margin: len(samples) - margin] if len(samples) > 4 * margin else samples
    return rms_value(core)


def best_loop_seam(samples: np.ndarray, rate: int, crossfade_frames: int, minimum_loop_seconds: float) -> tuple[int, float]:
    end = len(samples)
    tail = samples[end - crossfade_frames:end]
    tail_energy = float(np.sum(tail * tail))
    latest_start = min(end - int(minimum_loop_seconds * rate), end - 2 * crossfade_frames)
    earliest_start = int(0.1 * rate) + crossfade_frames
    if latest_start <= earliest_start:
        raise ValueError("source too short for the requested loop preparation")
    best_start = earliest_start
    best_correlation = -2.0
    for start in range(earliest_start, latest_start):
        head = samples[start - crossfade_frames:start]
        denominator = math.sqrt(float(np.sum(head * head)) * tail_energy) + 1e-12
        correlation = float(np.sum(head * tail) / denominator)
        if correlation > best_correlation:
            best_correlation = correlation
            best_start = start
    return best_start, best_correlation


def prepare_loop(samples: np.ndarray, rate: int) -> tuple[np.ndarray, dict]:
    crossfade = max(int(LOOP_CROSSFADE_SECONDS * rate), 32)
    start, correlation = best_loop_seam(samples, rate, crossfade, 1.0)
    segment = samples[start:].copy()
    length = len(segment)
    fade = min(crossfade, length // 4)
    ramp = np.linspace(0.0, 1.0, fade, endpoint=False)
    preceding = samples[start - fade:start]
    segment[length - fade:] = segment[length - fade:] * (1.0 - ramp) + preceding * ramp
    seam_metrics = {
        "source_loop_start_frame": int(start),
        "source_loop_end_frame_exclusive": int(len(samples)),
        "loop_crossfade_frames": int(fade),
        "naive_wrap_step_before_preparation": float(abs(samples[0] - samples[-1])),
        "endpoint_correlation": float(correlation),
        "prepared_wrap_step": float(abs(segment[-1] - segment[0])),
    }
    return segment, seam_metrics


def apply_rate(samples: np.ndarray, rate: int, ratio: float) -> np.ndarray:
    fraction = Fraction(ratio).limit_denominator(4096)
    return resample_poly(samples, fraction.numerator, fraction.denominator)


def build_manifest(
    source_dir: Path,
    bank_dir: Path,
    reports_dir: Path,
    check_only: bool,
    source_inventory: dict | None = None,
) -> int:
    definition_table = source_inventory if source_inventory is not None else SOURCE_INVENTORY
    missing = [name for name in definition_table if not (source_dir / name).is_file()]
    if missing:
        print("missing sources: " + ", ".join(sorted(missing)), file=sys.stderr)
        return 1

    inventory_entries = []
    source_audio: dict[str, tuple[np.ndarray, int]] = {}
    for name in sorted(definition_table):
        entry = definition_table[name]
        path = source_dir / name
        digest = sha256_file(path)
        expected = entry["sha256"]
        if digest != expected:
            print(f"sha256 mismatch for {name}: {digest} != {expected}", file=sys.stderr)
            return 1
        samples, rate, channels, bits = read_wav(path)
        source_audio[name] = (samples, rate)
        inventory_entry = {
            "filename": name,
            "sha256": digest,
            "role": entry["role"],
            "asset_id": entry["asset_id"],
            "sample_rate_hz": rate,
            "channels": channels,
            "bits_per_sample": bits,
            "frames": int(len(samples)),
            "duration_seconds": len(samples) / rate,
            "rms_dbfs": dbfs(rms_value(samples)),
            "peak_dbfs": dbfs(float(np.max(np.abs(samples))) if len(samples) else 0.0),
            "dc_offset": float(np.mean(samples)) if len(samples) else 0.0,
        }
        if "original_source_sha256" in entry:
            inventory_entry["original_source_sha256"] = entry["original_source_sha256"]
            inventory_entry["original_source_filename"] = entry.get(
                "original_source_filename", name
            )
        if "preparation_recipe" in entry:
            inventory_entry["preparation_recipe"] = entry["preparation_recipe"]
        if entry.get("component_only"):
            inventory_entry["component_only"] = True
        inventory_entries.append(inventory_entry)
    inventory = {
        "schema_version": SCHEMA_VERSION,
        "tool_revision": TOOL_REVISION,
        "sources": inventory_entries,
    }
    inventory_hash = sha256_bytes(json.dumps(inventory, sort_keys=True, indent=2).encode("utf-8"))

    loop_assets: list[dict] = []
    loop_audio: dict[str, np.ndarray] = {}
    seam_evidence: list[dict] = []
    for name in LOOP_ORDER:
        source_name = next(
            filename for filename, entry in definition_table.items() if entry["asset_id"] == name
        )
        source_entry = definition_table[source_name]
        samples, rate = source_audio[source_name]
        if source_entry.get("already_prepared"):
            prepared = samples
            seam = {
                "source_loop_start_frame": 0,
                "source_loop_end_frame_exclusive": int(len(samples)),
                "loop_crossfade_frames": int(source_entry.get("loop_crossfade_frames", 0)),
                "naive_wrap_step_before_preparation": float(abs(samples[0] - samples[-1])),
                "endpoint_correlation": float(source_entry.get("endpoint_correlation", 0.0)),
                "prepared_wrap_step": float(abs(samples[-1] - samples[0])),
            }
        else:
            prepared, seam = prepare_loop(samples, rate)
        peaks = spectral_peaks(prepared, rate)
        spacing, score, harmonic = harmonic_comb_spacing(peaks, 20.0, 180.0)
        loop_audio[name] = prepared
        loop_assets.append(
            {
                "id": name,
                "role": "engine_loop",
                "source_filename": source_name,
                "source_sha256": source_entry["sha256"],
                "derived_filename": f"{name}.wav",
                "derived_frames": int(len(prepared)),
                "derived_rms_dbfs": dbfs(rms_value(prepared)),
                "derived_peak_dbfs": dbfs(float(np.max(np.abs(prepared)))),
                "derived_band_rms_300_6000": band_rms(prepared, rate, *LEVEL_BAND_HZ),
                "comb_spacing_hz": spacing,
                "comb_fit_confidence": float(score / max(harmonic, 1)),
                "comb_harmonic_span": harmonic,
                "preparation_recipe": source_entry.get(
                    "preparation_recipe", "prepare_loop_crossfade_v1"
                ),
            }
        )
        seam_evidence.append({"asset_id": name, **seam})

    spacings = [asset["comb_spacing_hz"] for asset in loop_assets]
    ladder_span = max(spacings) / min(spacings)
    center_rpm = math.sqrt(
        PHYSICAL_IDLE_REVOLUTIONS_PER_MINUTE * PHYSICAL_MAX_REVOLUTIONS_PER_MINUTE
    )
    base_rpm_per_hz = center_rpm / math.sqrt(ladder_span) / min(spacings)
    references = [spacing * base_rpm_per_hz for spacing in spacings]

    authored_references = [
        definition_table[asset["source_filename"]].get("reference_revolutions_per_minute")
        for asset in loop_assets
    ]
    references_are_authored = all(
        reference is not None and reference > 0.0 for reference in authored_references
    ) and all(
        authored_references[index] < authored_references[index + 1]
        for index in range(len(authored_references) - 1)
    )
    if references_are_authored:
        references = [float(reference) for reference in authored_references]
        base_rpm_per_hz = sum(references) / sum(spacings)
    reference_method = (
        "authored_progression_preserved_with_measured_spacing_evidence"
        if references_are_authored
        else "measured_comb_spacing_proportional_ladder"
    )

    for asset, spacing, reference in zip(loop_assets, spacings, references):
        asset["reference_revolutions_per_minute"] = reference
        asset["reference_method"] = reference_method
        asset["reference_rpm_per_hz"] = base_rpm_per_hz

    boundaries = [
        math.sqrt(references[index] * references[index + 1])
        for index in range(len(references) - 1)
    ]
    transitions = []
    for index, center in enumerate(boundaries):
        half_width = center * TRANSITION_HALF_WIDTH_RATIO
        transitions.append(
            {
                "from_loop_id": loop_assets[index]["id"],
                "to_loop_id": loop_assets[index + 1]["id"],
                "start_revolutions_per_minute": center - half_width,
                "end_revolutions_per_minute": center + half_width,
                "center_revolutions_per_minute": center,
                "half_width_revolutions_per_minute": half_width,
                "blend_law": TRANSITION_BLEND_LAW,
                "gain_law": TRANSITION_GAIN_LAW,
                "measured_level_compensation_db": 0.0,
            }
        )

    coverage_min = PHYSICAL_IDLE_REVOLUTIONS_PER_MINUTE
    coverage_max = PHYSICAL_MAX_REVOLUTIONS_PER_MINUTE

    gains = [1.0]
    level_step = 10.0 ** (BOUNDARY_LEVEL_STEP_DB / 20.0)
    for index, transition in enumerate(transitions):
        center = transition["center_revolutions_per_minute"]
        left_rate = center / references[index]
        right_rate = center / references[index + 1]
        left_audio = apply_rate(loop_audio[loop_assets[index]["id"]], OUTPUT_SAMPLE_RATE, left_rate)
        right_audio = apply_rate(loop_audio[loop_assets[index + 1]["id"]], OUTPUT_SAMPLE_RATE, right_rate)
        left_level = loop_level(left_audio)
        right_level = loop_level(right_audio)
        ratio = left_level / max(right_level, 1e-9)
        gains.append(gains[index] * ratio * level_step)
        transition["measured_level_compensation_db"] = float(20.0 * math.log10(max(ratio, 1e-9)))
        transition["applied_level_step_db"] = BOUNDARY_LEVEL_STEP_DB
        loop_assets[index]["boundary_level_rms"] = left_level
        loop_assets[index + 1]["boundary_level_rms"] = right_level

    normalization = max(gains) if max(gains) > 0.0 else 1.0
    calibrated_gains = [gain / normalization for gain in gains]
    for asset, gain in zip(loop_assets, calibrated_gains):
        asset["calibrated_gain"] = float(gain)

    for index, asset in enumerate(loop_assets):
        transition_before = transitions[index - 1] if index > 0 else None
        transition_after = transitions[index] if index < len(transitions) else None
        start = coverage_min if transition_before is None else transition_before["start_revolutions_per_minute"]
        end = coverage_max if transition_after is None else transition_after["end_revolutions_per_minute"]
        reference = asset["reference_revolutions_per_minute"]
        asset["active_coverage_revolutions_per_minute"] = [start, end]
        asset["valid_playback_rate_min"] = float(min(start / reference, end / reference))
        asset["valid_playback_rate_max"] = float(max(start / reference, end / reference))
        asset["loop_start_frame"] = 0
        asset["loop_end_frame_exclusive"] = int(asset["derived_frames"])
        asset["loop_crossfade_frames"] = int(
            next(seam["loop_crossfade_frames"] for seam in seam_evidence if seam["asset_id"] == asset["id"])
        )

    event_assets: list[dict] = []
    event_audio: dict[str, np.ndarray] = {}
    for name in sorted(definition_table):
        entry = definition_table[name]
        if entry["role"] == "engine_loop" or entry.get("component_only"):
            continue
        samples, rate = source_audio[name]
        source_rate = rate
        if rate != OUTPUT_SAMPLE_RATE:
            samples = resample_poly(samples, OUTPUT_SAMPLE_RATE, rate)
            rate = OUTPUT_SAMPLE_RATE
        event_metadata = {}
        composite_recipe = GEARBOX_EVENT_COMPOSITE_RECIPES.get(entry["asset_id"])
        if entry.get("already_prepared"):
            preparation_recipe = entry.get("preparation_recipe", "prepared_event_v1")
        elif composite_recipe is not None:
            layer_source_filename = composite_recipe["layer_source_filename"]
            layer_entry = definition_table[layer_source_filename]
            layer_samples, layer_rate = source_audio[layer_source_filename]
            if layer_rate != OUTPUT_SAMPLE_RATE:
                layer_samples = resample_poly(
                    layer_samples,
                    OUTPUT_SAMPLE_RATE,
                    layer_rate,
                )
            samples, composite_processing = prepare_gearbox_event_composite(
                samples,
                layer_samples,
                rate,
                composite_recipe,
            )
            preparation_recipe = "gearbox_body_and_1996_mechanical_composite_v1"
            event_metadata = {
                "source_components": [
                    {
                        "purpose": "original_body_and_low_frequency_weight",
                        "source_filename": name,
                        "source_sha256": entry["sha256"],
                        "gain_decibels": 0.0,
                    },
                    {
                        "purpose": "1996_mechanical_attack_and_midrange_detail",
                        "source_filename": layer_source_filename,
                        "source_sha256": layer_entry["sha256"],
                        "gain_decibels": float(composite_recipe["layer_gain_decibels"]),
                    },
                ],
                "composite_processing": composite_processing,
            }
        elif entry["role"] in ("lift_backfire", "limiter_event"):
            samples = samples - float(np.mean(samples))
            fade_in = min(int(0.002 * rate), len(samples))
            fade_out = min(int(0.012 * rate), len(samples))
            if fade_in > 1:
                samples[:fade_in] *= np.linspace(0.0, 1.0, fade_in)
            if fade_out > 1:
                samples[len(samples) - fade_out:] *= np.linspace(1.0, 0.0, fade_out)
            peak = float(np.max(np.abs(samples))) if len(samples) else 0.0
            if peak > 1e-12:
                samples = samples * (0.89 / peak)
            samples = np.clip(samples, -0.999, 0.999)
            preparation_recipe = (
                "backfire_overlay_recipe_v1"
                if entry["role"] == "lift_backfire"
                else "limiter_overlay_recipe_v1"
            )
        else:
            tail = samples[-1] if len(samples) else 0.0
            if abs(tail) > 0.01 and len(samples) > 64:
                fade = min(int(0.003 * rate), len(samples) // 4)
                samples = samples.copy()
                samples[-fade:] *= np.linspace(1.0, 0.0, fade)
            preparation_recipe = (
                "resampled_to_44100_band_limited"
                if source_rate != OUTPUT_SAMPLE_RATE
                else "native_44100_copy"
            )
        event_audio[entry["asset_id"]] = samples
        event_assets.append(
            {
                "id": entry["asset_id"],
                "role": entry["role"],
                "source_filename": name,
                "source_sha256": entry["sha256"],
                "derived_filename": f"{entry['asset_id']}.wav",
                "derived_frames": int(len(samples)),
                "derived_rms_dbfs": dbfs(rms_value(samples)),
                "derived_peak_dbfs": dbfs(float(np.max(np.abs(samples))) if len(samples) else 0.0),
                "derived_band_rms_300_6000": band_rms(samples, rate, *LEVEL_BAND_HZ),
                "preparation_recipe": preparation_recipe,
                "duration_seconds": len(samples) / rate,
                **event_metadata,
            }
        )

    event_groups = []
    for definition in EVENT_GROUP_DEFINITIONS:
        variants = []
        for asset_id in definition["variant_asset_ids"]:
            asset = next(asset for asset in event_assets if asset["id"] == asset_id)
            variants.append(
                {
                    "asset_id": asset_id,
                    "event_gain": 1.0,
                    "trigger_pitch_reference_revolutions_per_minute": None,
                }
            )
        event_groups.append(
            {
                "id": definition["id"],
                "role": definition["role"],
                "selection": definition["selection"],
                "voice_limit": definition["voice_limit"],
                "minimum_retrigger_seconds": definition["minimum_retrigger_seconds"],
                "timing_offset_seconds": definition["timing_offset_seconds"],
                "trigger_event": definition["trigger_event"],
                "variants": variants,
            }
        )

    manifest = {
        "schema_version": SCHEMA_VERSION,
        "bank_id": "formula_one_2030_grand_prix_sampler",
        "preparation_tool": "scripts/audio/build_grand_prix_sampler_bank.py",
        "preparation_tool_revision": TOOL_REVISION,
        "source_inventory_sha256": inventory_hash,
        "output_sample_rate": OUTPUT_SAMPLE_RATE,
        "event_selection_seed": EVENT_SELECTION_SEED,
        "coverage": {
            "minimum_revolutions_per_minute": coverage_min,
            "maximum_revolutions_per_minute": coverage_max,
        },
        "loops": loop_assets,
        "transitions": transitions,
        "events": event_assets,
        "event_groups": event_groups,
        "event_trigger_policy": EVENT_TRIGGER_POLICY,
        "reference_ladder": {
            "method": reference_method,
            "revolutions_per_minute_per_hertz": base_rpm_per_hz,
            "anchor_center_revolutions_per_minute": center_rpm,
            "measured_spacings_hz": spacings,
            "reference_revolutions_per_minute": references,
        },
    }

    calibration = {
        "schema_version": SCHEMA_VERSION,
        "tool_revision": TOOL_REVISION,
        "loops": [
            {
                "asset_id": asset["id"],
                "comb_spacing_hz": asset["comb_spacing_hz"],
                "reference_revolutions_per_minute": asset["reference_revolutions_per_minute"],
                "calibrated_gain": asset["calibrated_gain"],
                "valid_playback_rate_min": asset["valid_playback_rate_min"],
                "valid_playback_rate_max": asset["valid_playback_rate_max"],
                "active_coverage_revolutions_per_minute": asset[
                    "active_coverage_revolutions_per_minute"
                ],
            }
            for asset in loop_assets
        ],
        "transitions": transitions,
        "coverage": manifest["coverage"],
        "reference_ladder": manifest["reference_ladder"],
    }

    bank_dir.mkdir(parents=True, exist_ok=True)
    reports_dir.mkdir(parents=True, exist_ok=True)
    expected_names = {asset["derived_filename"] for asset in loop_assets + event_assets}
    if not check_only:
        for existing in bank_dir.iterdir():
            if existing.is_file() and existing.name.endswith(".wav") and existing.name not in expected_names:
                existing.unlink()
                companion = existing.with_name(existing.name + ".import")
                if companion.is_file():
                    companion.unlink()
    failures: list[str] = []
    for asset in loop_assets + event_assets:
        target = bank_dir / asset["derived_filename"]
        audio = loop_audio.get(asset["id"], event_audio.get(asset["id"]))
        expected_bytes = wav_bytes(audio, OUTPUT_SAMPLE_RATE)
        if check_only:
            if not target.is_file() or target.read_bytes() != expected_bytes:
                failures.append(str(target))
            else:
                asset["derived_sha256"] = sha256_bytes(expected_bytes)
        else:
            target.write_bytes(expected_bytes)
            asset["derived_sha256"] = sha256_bytes(expected_bytes)

    payloads = {
        "manifest.json": json.dumps(manifest, sort_keys=True, indent=2) + "\n",
        "source_inventory.json": json.dumps(inventory, sort_keys=True, indent=2) + "\n",
        "calibration.json": json.dumps(calibration, sort_keys=True, indent=2) + "\n",
        "seam_evidence.json": json.dumps(seam_evidence, sort_keys=True, indent=2) + "\n",
    }

    if check_only:
        for name, payload in payloads.items():
            target = (bank_dir / name) if name == "manifest.json" else (reports_dir / name)
            if not target.is_file() or target.read_text(encoding="utf-8") != payload:
                failures.append(str(target))
        if failures:
            print("bank check failed for: " + ", ".join(failures), file=sys.stderr)
            return 1
        print("bank check ok")
        return 0

    (bank_dir / "manifest.json").write_text(payloads["manifest.json"], encoding="utf-8")
    (reports_dir / "source_inventory.json").write_text(payloads["source_inventory.json"], encoding="utf-8")
    (reports_dir / "calibration.json").write_text(payloads["calibration.json"], encoding="utf-8")
    (reports_dir / "seam_evidence.json").write_text(payloads["seam_evidence.json"], encoding="utf-8")

    print(f"bank={bank_dir} loops={len(loop_assets)} events={len(event_assets)}")
    for asset in loop_assets:
        print(
            f"  {asset['id']:24s} ref={asset['reference_revolutions_per_minute']:8.1f}rpm "
            f"gain={asset['calibrated_gain']:.4f} rate=[{asset['valid_playback_rate_min']:.3f},"
            f"{asset['valid_playback_rate_max']:.3f}] frames={asset['derived_frames']}"
        )
    for transition in transitions:
        print(
            f"  {transition['from_loop_id']} -> {transition['to_loop_id']}: "
            f"[{transition['start_revolutions_per_minute']:.0f}, "
            f"{transition['end_revolutions_per_minute']:.0f}] rpm "
            f"comp={transition['measured_level_compensation_db']:+.2f}dB"
        )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-dir", type=Path, default=DEFAULT_SOURCE_DIR)
    parser.add_argument("--bank-dir", type=Path, default=DEFAULT_BANK_DIR)
    parser.add_argument("--reports-dir", type=Path, default=DEFAULT_REPORTS_DIR)
    parser.add_argument("--source-inventory", type=Path, default=None)
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()
    source_inventory = (
        load_source_inventory(arguments.source_inventory)
        if arguments.source_inventory is not None
        else None
    )
    return build_manifest(
        arguments.source_dir,
        arguments.bank_dir,
        arguments.reports_dir,
        arguments.check,
        source_inventory,
    )


if __name__ == "__main__":
    raise SystemExit(main())
