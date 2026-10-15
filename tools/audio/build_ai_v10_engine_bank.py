"""Build the AI-only V10 engine sample bank (schema 1) for artificial drivers.

Reads 48000 hertz raw mono recordings from
``game/sounds/bank_sources/v10-bank-ai`` plus an optional per-sample sidecar
``v10_ai_source_inventory.json`` carrying authored revolutions and individual
crossfade widths, then prepares deterministic 44100 hertz runtime loops with
integer engine-cycle phase coherence into ``game/sounds/banks/v10-bank-ai``.

The bank is engine-only: no transmission whine, no gearbox events, no
backfire, no limiter. Artificial-driver voices select engine timbre alone.
The player bank ``v10-v2-bank`` is untouched. Prepared runtime WAVs are never
rewritten once promoted: reruns with identical sources produce byte-identical
outputs.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import wave
from fractions import Fraction
from pathlib import Path

import numpy as np
from scipy.signal import butter, resample_poly, sosfiltfilt

from tools.common.output_policy import OutputMode, validate_output_path

ROOT = Path(__file__).resolve().parents[2]
SOURCE_DIRECTORY = ROOT / "game/sounds/bank_sources/v10-bank-ai"
SIDECAR_FILENAME = "v10_ai_source_inventory.json"
DEFAULT_BANK_DIRECTORY = ROOT / "game/sounds/banks/v10-bank-ai"
DEFAULT_REPORT_DIRECTORY = ROOT / "scratch/audio/v10-bank-ai"

SAMPLE_RATE = 44100
SCHEMA_VERSION = 1
TOOL_NAME = "tools/audio/build_ai_v10_engine_bank.py"
TOOL_REVISION = 1
BANK_ID = "v10_ai_engine_bank"
EVENT_SELECTION_SEED = 1

MINIMUM_REVOLUTIONS_PER_MINUTE = 4500.0
MAXIMUM_REVOLUTIONS_PER_MINUTE = 18000.0
REVOLUTIONS_PER_MINUTE_PER_HERTZ = 120.0
CROSSFADE_CYCLE_COUNT = 4
TRANSITION_HALF_WIDTH_RATIO = 0.075
TRANSITION_NARROW_HALF_WIDTH_RATIO = 0.04
TRANSITION_BLEND_LAW = "smoothstep"
TRANSITION_GAIN_LAW = "equal_power"
LEVEL_BAND_HZ = (300.0, 6000.0)
PREPARATION_RECIPE = "ai_mono16_cycle_integer_wrap_equal_power_v1"

SOURCE_TABLE = [
    {"stem": "ex_v10_idle", "role": "engine_loop", "loop_id": "ai_engine_idle_loop",
     "authored_revolutions_per_minute": 4500.0, "zone_revolutions_per_minute": 4500.0},
    {"stem": "ex_v10_on_low_7104", "role": "engine_loop", "loop_id": "ai_engine_powered_low_loop",
     "authored_revolutions_per_minute": 7104.0, "zone_revolutions_per_minute": 7104.0},
    {"stem": "ex_v10_on_mid_12924", "role": "engine_loop", "loop_id": "ai_engine_powered_middle_loop",
     "authored_revolutions_per_minute": 12924.0, "zone_revolutions_per_minute": 12924.0},
    {"stem": "ex_v10_on_high_13260", "role": "engine_loop", "loop_id": "ai_engine_powered_high_loop",
     "authored_revolutions_per_minute": 13260.0, "zone_revolutions_per_minute": 13260.0},
    {"stem": "ex_v10_on_mid_14808", "role": "engine_loop", "loop_id": "ai_engine_powered_upper_loop",
     "authored_revolutions_per_minute": 14808.0, "zone_revolutions_per_minute": 14808.0},
    {"stem": "ex_v10_on_high_19068", "role": "engine_loop", "loop_id": "ai_engine_powered_maximum_loop",
     "authored_revolutions_per_minute": 19068.0, "zone_revolutions_per_minute": 19068.0},
    {"stem": "ex_v10_off_mid_10740", "role": "engine_coast_loop", "loop_id": "ai_engine_coast_middle_loop",
     "authored_revolutions_per_minute": 10740.0, "zone_revolutions_per_minute": 10740.0},
    {"stem": "ex_v10_off_mid_12384", "role": "engine_coast_loop", "loop_id": "ai_engine_coast_upper_middle_loop",
     "authored_revolutions_per_minute": 12384.0, "zone_revolutions_per_minute": 12384.0},
    {"stem": "ex_v10_off_high_13800", "role": "engine_coast_loop", "loop_id": "ai_engine_coast_high_loop",
     "authored_revolutions_per_minute": 13800.0, "zone_revolutions_per_minute": 13800.0},
    {"stem": "ex_v10_off_high_16704", "role": "engine_coast_loop", "loop_id": "ai_engine_coast_maximum_loop",
     "authored_revolutions_per_minute": 16704.0, "zone_revolutions_per_minute": 16704.0},
]


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_wave_mono(path: Path) -> tuple[int, np.ndarray]:
    with wave.open(str(path), "rb") as reader:
        channels = reader.getnchannels()
        width = reader.getsampwidth()
        rate = reader.getframerate()
        frames = reader.getnframes()
        raw = reader.readframes(frames)
    if channels != 1:
        raise ValueError(f"{path} must be mono, found {channels} channels")
    if width != 2:
        raise ValueError(f"{path} must be 16-bit, found width {width}")
    octets = np.frombuffer(raw, dtype=np.uint8).reshape(-1, 2)
    values = octets[:, 0].astype(np.int32) | (octets[:, 1].astype(np.int32) << 8)
    values = np.where(values >= (1 << 15), values - (1 << 16), values).astype(np.float64)
    samples = values / 32768.0
    return rate, samples


def write_wave_mono16(path: Path, samples: np.ndarray) -> bytes:
    clipped = np.clip(samples, -1.0, 1.0 - 1.0 / 32768.0)
    quantized = np.clip(np.round(clipped * 32768.0).astype(np.int64), -32768, 32767).astype(np.int32)
    low_octets = (quantized & 0xFF).astype(np.uint8)
    high_octets = ((quantized >> 8) & 0xFF).astype(np.uint8)
    payload = np.empty(quantized.size * 2, dtype=np.uint8)
    payload[0::2] = low_octets
    payload[1::2] = high_octets
    raw = payload.tobytes()
    with wave.open(str(path), "wb") as writer:
        writer.setnchannels(1)
        writer.setsampwidth(2)
        writer.setframerate(SAMPLE_RATE)
        writer.writeframes(raw)
    return Path(path).read_bytes()


def rms_value(samples: np.ndarray) -> float:
    if samples.size == 0:
        return 0.0
    return float(math.sqrt(float(np.mean(np.square(samples)))))


def band_rms(samples: np.ndarray, low_hz: float, high_hz: float) -> float:
    if samples.size < 16:
        return rms_value(samples)
    sos = butter(4, [low_hz, high_hz], btype="bandpass", fs=SAMPLE_RATE, output="sos")
    filtered = sosfiltfilt(sos, samples)
    return rms_value(np.asarray(filtered, dtype=np.float64))


def loop_level(samples: np.ndarray) -> float:
    margin = len(samples) // 10
    core = samples[margin:len(samples) - margin] if len(samples) > 4 * margin else samples
    return rms_value(core)


def resample_to_runtime(rate: int, samples: np.ndarray) -> np.ndarray:
    if rate == SAMPLE_RATE:
        return samples.astype(np.float64)
    ratio = Fraction(SAMPLE_RATE, rate)
    return np.asarray(
        resample_poly(samples, ratio.numerator, ratio.denominator), dtype=np.float64
    )


def load_sidecar_overrides() -> dict:
    sidecar = SOURCE_DIRECTORY / SIDECAR_FILENAME
    if not sidecar.is_file():
        return {}
    document = json.loads(sidecar.read_text(encoding="utf-8"))
    if not isinstance(document, dict):
        raise ValueError("AI source sidecar must hold a JSON object")
    return document


def transition_half_widths(zones: list[float], override_ratios: list[float]) -> list[float]:
    centers = [
        math.sqrt(zones[index] * zones[index + 1]) for index in range(len(zones) - 1)
    ]
    widths = []
    for index, center in enumerate(centers):
        ratio = override_ratios[index]
        if ratio <= 0.0:
            ratio = TRANSITION_HALF_WIDTH_RATIO
        neighbours_close = (
            abs(zones[index + 1] - zones[index]) / max(zones[index], 1e-9) < 0.08
        )
        if neighbours_close:
            ratio = min(ratio, TRANSITION_NARROW_HALF_WIDTH_RATIO)
        widths.append(center * ratio)
    for index, center in enumerate(centers):
        neighbour_gap = math.inf
        if index > 0:
            neighbour_gap = min(neighbour_gap, center - centers[index - 1])
        if index + 1 < len(centers):
            neighbour_gap = min(neighbour_gap, centers[index + 1] - center)
        if neighbour_gap < math.inf:
            widths[index] = min(widths[index], 0.45 * neighbour_gap)
    return widths


def transition_record(from_id: str, to_id: str, start: float, end: float) -> dict:
    center = (start + end) * 0.5
    return {
        "from_loop_id": from_id,
        "to_loop_id": to_id,
        "start_revolutions_per_minute": float(start),
        "end_revolutions_per_minute": float(end),
        "center_revolutions_per_minute": float(center),
        "half_width_revolutions_per_minute": float((end - start) * 0.5),
        "blend_law": TRANSITION_BLEND_LAW,
        "gain_law": TRANSITION_GAIN_LAW,
        "measured_level_compensation_db": 0.0,
        "applied_level_step_db": 0.0,
    }


def trim_to_integer_cycles(
    samples: np.ndarray, authored_revolutions_per_minute: float
) -> tuple[np.ndarray, int, float]:
    cycle_hertz = authored_revolutions_per_minute / REVOLUTIONS_PER_MINUTE_PER_HERTZ
    cycles = max(1, round(samples.size * cycle_hertz / SAMPLE_RATE))
    while cycles > 1 and round(cycles * SAMPLE_RATE / cycle_hertz) > samples.size:
        cycles -= 1
    target_frames = int(round(cycles * SAMPLE_RATE / cycle_hertz))
    start_frame = (samples.size - target_frames) // 2
    trimmed = samples[start_frame:start_frame + target_frames]
    reference = cycles * SAMPLE_RATE / trimmed.size * REVOLUTIONS_PER_MINUTE_PER_HERTZ
    return np.asarray(trimmed, dtype=np.float64), cycles, float(reference)


def apply_wrap_crossfade(samples: np.ndarray, reference_revolutions_per_minute: float) -> tuple[np.ndarray, int]:
    cycle_hertz = reference_revolutions_per_minute / REVOLUTIONS_PER_MINUTE_PER_HERTZ
    crossfade_frames = max(64, int(round(CROSSFADE_CYCLE_COUNT * SAMPLE_RATE / cycle_hertz)))
    if crossfade_frames * 4 > samples.size:
        raise ValueError("AI loop is too short for its wrap crossfade")
    prepared = samples.copy()
    ramp = np.linspace(0.0, math.pi * 0.5, crossfade_frames)
    head_gain = np.sin(ramp) ** 2
    tail_gain = np.cos(ramp) ** 2
    prepared[:crossfade_frames] = (
        samples[:crossfade_frames] * head_gain + samples[-crossfade_frames:] * tail_gain
    )
    return prepared, crossfade_frames


def measure_seam(samples: np.ndarray) -> dict:
    deltas = np.abs(np.diff(np.round(samples * 32768.0).astype(np.int64)))
    if deltas.size == 0:
        return {"wrap_step": 0, "local_slope": 0, "seam_contract_holds": True}
    local_slope = int(np.sort(deltas)[int(deltas.size * 0.99)])
    wrap_step = abs(round(float(samples[0]) * 32768.0) - round(float(samples[-1]) * 32768.0))
    return {
        "wrap_step": wrap_step,
        "local_slope": local_slope,
        "seam_contract_holds": wrap_step <= max(2 * local_slope, 64),
    }


def prepare_loop(
    source_filename: str,
    authored_revolutions_per_minute: float,
) -> tuple[dict, np.ndarray]:
    source_path = SOURCE_DIRECTORY / source_filename
    if not source_path.is_file():
        raise FileNotFoundError(f"missing AI engine source: {source_path}")
    source_hash = sha256_file(source_path)
    source_rate, source_samples = read_wave_mono(source_path)
    runtime_samples = resample_to_runtime(source_rate, source_samples)
    trimmed, cycle_count, reference = trim_to_integer_cycles(
        runtime_samples, authored_revolutions_per_minute
    )
    prepared, crossfade_frames = apply_wrap_crossfade(trimmed, reference)
    seam_after = measure_seam(prepared)
    peak_amplitude = float(np.max(np.abs(prepared))) if prepared.size else 0.0
    record = {
        "source_filename": source_filename,
        "source_sha256": source_hash,
        "source_sample_rate": source_rate,
        "reference_revolutions_per_minute": reference,
        "integer_cycle_count": cycle_count,
        "loop_crossfade_frames": crossfade_frames,
        "derived_frames": int(prepared.size),
        "derived_band_rms_300_6000": band_rms(prepared, *LEVEL_BAND_HZ),
        "derived_rms": rms_value(prepared),
        "derived_peak_dbfs": float(20.0 * math.log10(max(peak_amplitude, 1e-9))),
        "comb_spacing_hz": float(reference / REVOLUTIONS_PER_MINUTE_PER_HERTZ),
        "seam_after_preparation": seam_after,
        "preparation_recipe": PREPARATION_RECIPE,
    }
    return record, prepared


def build_collection(
    assets: list[dict],
    audio: dict[str, np.ndarray],
    coverage: dict,
    width_ratios: list[float],
) -> list[dict]:
    widths = transition_half_widths(
        [asset["zone_revolutions_per_minute"] for asset in assets], width_ratios
    )
    transitions: list[dict] = []
    for index in range(len(assets) - 1):
        center = math.sqrt(
            assets[index]["zone_revolutions_per_minute"]
            * assets[index + 1]["zone_revolutions_per_minute"]
        )
        half_width = widths[index]
        start = max(center - half_width, MINIMUM_REVOLUTIONS_PER_MINUTE)
        end = min(center + half_width, MAXIMUM_REVOLUTIONS_PER_MINUTE)
        if not end > start:
            raise ValueError("AI transition window collapsed inside coverage")
        transitions.append(
            transition_record(
                assets[index]["id"], assets[index + 1]["id"], start, end
            )
        )
    for index, record in enumerate(transitions):
        left_level = loop_level(audio[assets[index]["id"]])
        right_level = loop_level(audio[assets[index + 1]["id"]])
        ratio = left_level / max(right_level, 1e-9)
        record["measured_level_compensation_db"] = float(20.0 * math.log10(max(ratio, 1e-9)))
        assets[index]["boundary_level_rms"] = float(left_level)
        assets[index + 1]["boundary_level_rms"] = float(right_level)
    for index, asset in enumerate(assets):
        start = (
            coverage["minimum_revolutions_per_minute"]
            if index == 0
            else transitions[index - 1]["start_revolutions_per_minute"]
        )
        end = (
            coverage["maximum_revolutions_per_minute"]
            if index + 1 == len(assets)
            else transitions[index]["end_revolutions_per_minute"]
        )
        reference = asset["reference_revolutions_per_minute"]
        asset["active_coverage_revolutions_per_minute"] = [float(start), float(end)]
        asset["valid_playback_rate_min"] = float(min(start, end) / reference * (1.0 - 1e-6))
        asset["valid_playback_rate_max"] = float(max(start, end) / reference * (1.0 + 1e-6))
    return transitions


def apply_presence_curves(assets: list[dict], transitions: list[dict]) -> None:
    for index, asset in enumerate(assets):
        if index == 0:
            points = [
                (MINIMUM_REVOLUTIONS_PER_MINUTE, 1.0),
                (transitions[0]["start_revolutions_per_minute"], 1.0),
                (transitions[0]["end_revolutions_per_minute"], 0.0),
            ]
        elif index + 1 == len(assets):
            points = [
                (transitions[index - 1]["start_revolutions_per_minute"], 0.0),
                (transitions[index - 1]["end_revolutions_per_minute"], 1.0),
                (MAXIMUM_REVOLUTIONS_PER_MINUTE, 1.0),
            ]
        else:
            points = [
                (transitions[index - 1]["start_revolutions_per_minute"], 0.0),
                (transitions[index - 1]["end_revolutions_per_minute"], 1.0),
                (transitions[index]["start_revolutions_per_minute"], 1.0),
                (transitions[index]["end_revolutions_per_minute"], 0.0),
            ]
        deduped: list[tuple[float, float]] = []
        for point in points:
            if deduped and point[0] == deduped[-1][0]:
                if point[1] != deduped[-1][1]:
                    raise ValueError("AI presence curve holds conflicting duplicate points")
                continue
            deduped.append(point)
        asset["presence_curve"] = [
            {"revolutions_per_minute": float(point[0]), "relative_weight": float(point[1])}
            for point in deduped
        ]


def calibrate_group_to_median(assets: list[dict]) -> None:
    levels = sorted(asset["derived_band_rms_300_6000"] for asset in assets)
    median = levels[len(levels) // 2]
    for asset in assets:
        asset["calibrated_gain"] = float(median / max(asset["derived_band_rms_300_6000"], 1e-9))


def build_manifest(
    powered: list[dict],
    powered_transitions: list[dict],
    coast: list[dict],
    coast_transitions: list[dict],
    audio: dict[str, np.ndarray],
    source_records: dict[str, dict],
) -> dict:
    loops = []
    for asset in powered:
        record = source_records[asset["id"]]
        loops.append(
            {
                "id": asset["id"],
                "role": "engine_loop",
                "source_filename": asset["source_filename"],
                "source_sha256": record["source_sha256"],
                "derived_filename": asset["derived_filename"],
                "derived_sha256": asset["derived_sha256"],
                "derived_frames": asset["derived_frames"],
                "derived_rms_dbfs": float(20.0 * math.log10(max(record["derived_rms"], 1e-9))),
                "derived_peak_dbfs": record["derived_peak_dbfs"],
                "derived_band_rms_300_6000": record["derived_band_rms_300_6000"],
                "comb_spacing_hz": record["comb_spacing_hz"],
                "reference_method": "authored_revolutions_integer_cycles",
                "reference_rpm_per_hz": REVOLUTIONS_PER_MINUTE_PER_HERTZ,
                "reference_revolutions_per_minute": record["reference_revolutions_per_minute"],
                "integer_cycle_count": record["integer_cycle_count"],
                "preparation_recipe": PREPARATION_RECIPE,
                "loop_start_frame": 0,
                "loop_end_frame_exclusive": asset["derived_frames"],
                "loop_crossfade_frames": record["loop_crossfade_frames"],
                "seam_after_preparation": record["seam_after_preparation"],
                "zone_revolutions_per_minute": asset["zone_revolutions_per_minute"],
                "boundary_level_rms": asset["boundary_level_rms"],
                "calibrated_gain": asset["calibrated_gain"],
                "active_coverage_revolutions_per_minute": asset[
                    "active_coverage_revolutions_per_minute"
                ],
                "valid_playback_rate_min": asset["valid_playback_rate_min"],
                "valid_playback_rate_max": asset["valid_playback_rate_max"],
                "presence_curve": asset["presence_curve"],
            }
        )
    coast_loops = []
    for asset in coast:
        record = source_records[asset["id"]]
        coast_loops.append(
            {
                "id": asset["id"],
                "role": "engine_coast_loop",
                "source_filename": asset["source_filename"],
                "source_sha256": record["source_sha256"],
                "derived_filename": asset["derived_filename"],
                "derived_sha256": asset["derived_sha256"],
                "derived_frames": asset["derived_frames"],
                "derived_rms_dbfs": float(20.0 * math.log10(max(record["derived_rms"], 1e-9))),
                "derived_peak_dbfs": record["derived_peak_dbfs"],
                "derived_band_rms_300_6000": record["derived_band_rms_300_6000"],
                "comb_spacing_hz": record["comb_spacing_hz"],
                "reference_method": "authored_revolutions_integer_cycles",
                "reference_rpm_per_hz": REVOLUTIONS_PER_MINUTE_PER_HERTZ,
                "reference_revolutions_per_minute": record["reference_revolutions_per_minute"],
                "integer_cycle_count": record["integer_cycle_count"],
                "preparation_recipe": PREPARATION_RECIPE,
                "loop_start_frame": 0,
                "loop_end_frame_exclusive": asset["derived_frames"],
                "loop_crossfade_frames": record["loop_crossfade_frames"],
                "seam_after_preparation": record["seam_after_preparation"],
                "zone_revolutions_per_minute": asset["zone_revolutions_per_minute"],
                "boundary_level_rms": asset["boundary_level_rms"],
                "calibrated_gain": asset["calibrated_gain"],
                "active_coverage_revolutions_per_minute": asset[
                    "active_coverage_revolutions_per_minute"
                ],
                "valid_playback_rate_min": asset["valid_playback_rate_min"],
                "valid_playback_rate_max": asset["valid_playback_rate_max"],
            }
        )
    references = {
        asset["id"]: source_records[asset["id"]]["reference_revolutions_per_minute"]
        for asset in powered + coast
    }
    zones = {
        asset["id"]: asset["zone_revolutions_per_minute"] for asset in powered + coast
    }
    return {
        "schema_version": SCHEMA_VERSION,
        "bank_id": BANK_ID,
        "preparation_tool": TOOL_NAME,
        "preparation_tool_revision": TOOL_REVISION,
        "source_inventory_sha256": "",
        "output_sample_rate": SAMPLE_RATE,
        "event_selection_seed": EVENT_SELECTION_SEED,
        "coverage": {
            "minimum_revolutions_per_minute": MINIMUM_REVOLUTIONS_PER_MINUTE,
            "maximum_revolutions_per_minute": MAXIMUM_REVOLUTIONS_PER_MINUTE,
        },
        "loops": loops,
        "transitions": powered_transitions,
        "coast_loops": coast_loops,
        "coast_transitions": coast_transitions,
        "events": [],
        "event_groups": [],
        "event_trigger_policy": {
            "lift_edge": {
                "previous_throttle_min": 0.8,
                "throttle_max": 0.15,
                "revolutions_per_minute_min": 13500.0,
                "cooldown_seconds": 1.0,
            },
            "limiter_entry_edge": {"cooldown_seconds": 0.25},
        },
        "reference_ladder": {
            "method": "authored_revolutions_integer_cycles",
            "revolutions_per_minute_per_hertz": REVOLUTIONS_PER_MINUTE_PER_HERTZ,
            "anchor_center_revolutions_per_minute": 9000.0,
            "engine_references_revolutions_per_minute": references,
            "engine_zones_revolutions_per_minute": zones,
        },
        "engine_source_selection": [
            {
                "role": asset["role"] if "role" in asset else "engine_loop",
                "source_path": f"game/sounds/bank_sources/v10-bank-ai/{asset['source_filename']}",
                "source_sha256": source_records[asset["id"]]["source_sha256"],
                "runtime_asset_id": asset["id"],
                "runtime_filename": asset["derived_filename"],
                "runtime_sha256": asset["derived_sha256"],
                "derived_frames": asset["derived_frames"],
                "reference_revolutions_per_minute": source_records[asset["id"]][
                    "reference_revolutions_per_minute"
                ],
                "zone_revolutions_per_minute": asset["zone_revolutions_per_minute"],
                "integer_cycle_count": source_records[asset["id"]]["integer_cycle_count"],
                "loop_crossfade_frames": source_records[asset["id"]]["loop_crossfade_frames"],
                "preparation_recipe": PREPARATION_RECIPE,
            }
            for asset in powered + coast
        ],
    }


def prepare_bank() -> tuple[dict, dict[str, bytes], dict]:
    sidecar = load_sidecar_overrides()
    per_sample = sidecar.get("samples", {}) if isinstance(sidecar, dict) else {}
    powered_entries = [entry for entry in SOURCE_TABLE if entry["role"] == "engine_loop"]
    coast_entries = [entry for entry in SOURCE_TABLE if entry["role"] == "engine_coast_loop"]
    audio: dict[str, np.ndarray] = {}
    source_records: dict[str, dict] = {}
    powered: list[dict] = []
    coast: list[dict] = []
    for entry in powered_entries + coast_entries:
        overrides = per_sample.get(entry["stem"], {})
        authored = float(
            overrides.get(
                "authored_revolutions_per_minute", entry["authored_revolutions_per_minute"]
            )
        )
        record, samples = prepare_loop(f"{entry['stem']}.wav", authored)
        derived_filename = f"{entry['loop_id']}.wav"
        payload = write_wave_mono16(DEFAULT_REPORT_DIRECTORY / derived_filename, samples)
        asset = {
            "id": entry["loop_id"],
            "source_filename": f"{entry['stem']}.wav",
            "derived_filename": derived_filename,
            "derived_frames": record["derived_frames"],
            "derived_sha256": sha256_bytes(payload),
            "derived_band_rms_300_6000": record["derived_band_rms_300_6000"],
            "reference_revolutions_per_minute": record["reference_revolutions_per_minute"],
            "zone_revolutions_per_minute": float(
                overrides.get("zone_revolutions_per_minute", entry["zone_revolutions_per_minute"])
            ),
            "width_ratio_override": float(overrides.get("transition_half_width_ratio", 0.0)),
        }
        audio[asset["id"]] = samples
        source_records[asset["id"]] = record
        (powered if entry["role"] == "engine_loop" else coast).append(asset)
    powered_widths = [asset.pop("width_ratio_override") for asset in powered]
    coast_widths = [asset.pop("width_ratio_override") for asset in coast]
    coverage = {
        "minimum_revolutions_per_minute": MINIMUM_REVOLUTIONS_PER_MINUTE,
        "maximum_revolutions_per_minute": MAXIMUM_REVOLUTIONS_PER_MINUTE,
    }
    powered_transitions = build_collection(powered, audio, coverage, powered_widths)
    coast_transitions = build_collection(coast, audio, coverage, coast_widths)
    apply_presence_curves(powered, powered_transitions)
    calibrate_group_to_median(powered)
    calibrate_group_to_median(coast)
    manifest = build_manifest(
        powered, powered_transitions, coast, coast_transitions, audio, source_records
    )
    inventory = {
        "bank_id": BANK_ID,
        "preparation_tool": TOOL_NAME,
        "preparation_tool_revision": TOOL_REVISION,
        "sources": [
            {
                "source_filename": asset["source_filename"],
                "source_sha256": source_records[asset["id"]]["source_sha256"],
                "source_sample_rate": source_records[asset["id"]]["source_sample_rate"],
                "runtime_asset_id": asset["id"],
                "runtime_filename": asset["derived_filename"],
                "runtime_sha256": asset["derived_sha256"],
                "reference_revolutions_per_minute": source_records[asset["id"]][
                    "reference_revolutions_per_minute"
                ],
                "integer_cycle_count": source_records[asset["id"]]["integer_cycle_count"],
            }
            for asset in powered + coast
        ],
    }
    manifest["source_inventory_sha256"] = sha256_bytes(
        json.dumps(inventory, sort_keys=True).encode("utf-8")
    )
    prepared_audio = {
        asset["derived_filename"]: (
            SOURCE_DIRECTORY,
            audio[asset["id"]],
        )
        for asset in powered + coast
    }
    return manifest, prepared_audio, inventory


def write_preview(report_directory: Path, manifest: dict, inventory: dict) -> None:
    report_directory.mkdir(parents=True, exist_ok=True)
    (report_directory / "manifest.preview.json").write_text(
        json.dumps(manifest, indent=2), encoding="utf-8"
    )
    (report_directory / "source_inventory.preview.json").write_text(
        json.dumps(inventory, indent=2), encoding="utf-8"
    )


def promote_to_bank(bank_directory: Path, manifest: dict, inventory: dict) -> None:
    decision = validate_output_path(ROOT, bank_directory, OutputMode.PROMOTE)
    decision.path.mkdir(parents=True, exist_ok=True)
    for asset in manifest["loops"] + manifest["coast_loops"]:
        preview_path = DEFAULT_REPORT_DIRECTORY / asset["derived_filename"]
        if not preview_path.is_file():
            raise FileNotFoundError(f"missing prepared loop preview: {preview_path}")
        payload = preview_path.read_bytes()
        if sha256_bytes(payload) != asset["derived_sha256"]:
            raise ValueError(f"prepared loop preview diverged: {asset['derived_filename']}")
        target = decision.path / asset["derived_filename"]
        target.write_bytes(payload)
    (decision.path / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    (decision.path / "source_inventory.json").write_text(
        json.dumps(inventory, indent=2), encoding="utf-8"
    )
    previous = sorted(
        path.name
        for path in decision.path.iterdir()
        if path.suffix.lower() == ".wav" and path.name.startswith("ex_v10_")
    )
    if previous:
        raise ValueError(
            "raw AI sources must not remain inside the bank; "
            f"relocate before promoting: {previous}"
        )


def check_sources() -> list[str]:
    problems = []
    for entry in SOURCE_TABLE:
        candidate = SOURCE_DIRECTORY / f"{entry['stem']}.wav"
        if not candidate.is_file():
            problems.append(f"missing source: {candidate}")
            continue
        try:
            rate, samples = read_wave_mono(candidate)
        except ValueError as error:
            problems.append(str(error))
            continue
        if samples.size < SAMPLE_RATE:
            problems.append(f"source shorter than one second: {candidate}")
    sidecar = SOURCE_DIRECTORY / SIDECAR_FILENAME
    if sidecar.is_file():
        try:
            document = json.loads(sidecar.read_text(encoding="utf-8"))
            if not isinstance(document, dict):
                problems.append("sidecar must hold a JSON object")
        except ValueError as error:
            problems.append(f"sidecar is invalid: {error}")
    return problems


def main(argv: list[str] | None = None) -> int:
    global DEFAULT_REPORT_DIRECTORY
    parser = argparse.ArgumentParser(description="Build the AI-only V10 engine bank")
    parser.add_argument("--check", action="store_true", help="validate sources without writing")
    parser.add_argument(
        "--promote",
        action="store_true",
        help="promote prepared loops plus manifest into game/sounds/banks/v10-bank-ai",
    )
    parser.add_argument("--bank", type=Path, default=DEFAULT_BANK_DIRECTORY)
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT_DIRECTORY)
    args = parser.parse_args(argv)
    problems = check_sources()
    if problems:
        for problem in problems:
            print(f"AI_BANK_PROBLEM {problem}")
        return 1
    if args.check:
        print(f"AI_BANK_OK sources={len(SOURCE_TABLE)}")
        return 0
    DEFAULT_REPORT_DIRECTORY = args.report
    validate_output_path(ROOT, args.report, OutputMode.PREVIEW)
    Path(args.report).mkdir(parents=True, exist_ok=True)
    manifest, _, inventory = prepare_bank()
    write_preview(args.report, manifest, inventory)
    print(f"AI_BANK_PREVIEW loops={len(manifest['loops'])} coast={len(manifest['coast_loops'])}")
    if args.promote:
        promote_to_bank(args.bank, manifest, inventory)
        print(f"AI_BANK_PROMOTED {args.bank}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
