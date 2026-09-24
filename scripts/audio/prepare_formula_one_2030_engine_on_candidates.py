from __future__ import annotations

import hashlib
import json
import math
import shutil
import wave
from pathlib import Path

import numpy
import soundfile

import build_grand_prix_sampler_bank
from analyze_v10_references import estimate_shaft_frequency, spectrum
from prepare_v10_sample_layer import find_engine_loop, make_crossfaded_loop, tonal_residual


ROOT_DIRECTORY = Path(__file__).resolve().parents[2]
SOURCE_AUDIO_PATH = ROOT_DIRECTORY / "game/sounds/banks/v10-gp3/tobe/r24-sound.mp3"
ACTIVE_BANK_DIRECTORY = ROOT_DIRECTORY / "game/audio/formula_one_2030_grand_prix_sampler"
CANDIDATE_BANK_DIRECTORY = ACTIVE_BANK_DIRECTORY / "engine_on_candidate"
SOURCE_INVENTORY_PATH = CANDIDATE_BANK_DIRECTORY / "source_inventory.json"
ACTIVE_MANIFEST_PATH = ACTIVE_BANK_DIRECTORY / "manifest.json"
CANDIDATE_MANIFEST_PATH = CANDIDATE_BANK_DIRECTORY / "manifest.json"
OUTPUT_SAMPLE_RATE = 44100
WINDOW_DURATION_SECONDS = 1.0
LOOP_CYCLES_720_DEGREES = 16
LOOP_CROSSFADE_FRAMES = 882
TARGET_TRUE_PEAK_DECIBELS_FULL_SCALE = -4.0
MAXIMUM_HALF_WINDOW_REVOLUTIONS_PER_MINUTE_SPREAD = 0.18
MINIMUM_ISOLATED_ENGINE_ENERGY_FRACTION = 0.10
SAMPLE_SPECIFICATIONS = [
    {"loop_identifier": "engine_low_loop", "window_start_seconds": 5.5},
    {"loop_identifier": "engine_medium_loop", "window_start_seconds": 87.0},
    {"loop_identifier": "engine_high_loop", "window_start_seconds": 120.0},
    {"loop_identifier": "engine_maximum_loop", "window_start_seconds": 104.5},
]


def calculate_sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def calculate_root_mean_square(audio: numpy.ndarray) -> float:
    return build_grand_prix_sampler_bank.rms_value(audio)


def calculate_band_root_mean_square(
    audio: numpy.ndarray,
    sample_rate: int,
    lower_frequency_hertz: float,
    upper_frequency_hertz: float,
) -> float:
    return build_grand_prix_sampler_bank.band_rms(
        audio,
        sample_rate,
        lower_frequency_hertz,
        upper_frequency_hertz,
    )


def calculate_decibels_full_scale(amplitude: float) -> float:
    return build_grand_prix_sampler_bank.dbfs(amplitude)


def calculate_true_peak_amplitude(audio: numpy.ndarray) -> float:
    return build_grand_prix_sampler_bank.estimate_true_peak_amplitude(audio)


def encode_mono_sixteen_bit_pulse_code_modulation_wave_file(
    audio: numpy.ndarray,
    sample_rate: int,
) -> bytes:
    return build_grand_prix_sampler_bank.wav_bytes(audio, sample_rate)


def decode_mono_sixteen_bit_pulse_code_modulation_wave_file(
    path: Path,
) -> tuple[numpy.ndarray, int, int]:
    with wave.open(str(path), "rb") as wave_file:
        channel_count = wave_file.getnchannels()
        sample_width_bytes = wave_file.getsampwidth()
        sample_rate = wave_file.getframerate()
        frame_count = wave_file.getnframes()
        if channel_count != 1 or sample_width_bytes != 2:
            raise ValueError(f"Expected mono PCM16 candidate WAV: {path}")
        audio = numpy.frombuffer(wave_file.readframes(frame_count), dtype="<i2")
    return audio.astype(numpy.float64) / 32768.0, sample_rate, frame_count


def measure_revolutions_per_minute(audio_segment: numpy.ndarray, sample_rate: int) -> float:
    centered_audio_segment = audio_segment - float(numpy.mean(audio_segment))
    frequency, power = spectrum(centered_audio_segment, sample_rate)
    shaft_frequency_hertz, _ = estimate_shaft_frequency(frequency, power)
    return shaft_frequency_hertz * 60.0


def write_json(path: Path, payload: dict) -> bytes:
    encoded_payload = (json.dumps(payload, indent=2, ensure_ascii=False) + "\n").encode("utf-8")
    path.write_bytes(encoded_payload)
    return encoded_payload


def copy_active_bank_assets() -> None:
    CANDIDATE_BANK_DIRECTORY.mkdir(parents=True)
    shutil.copy2(ACTIVE_MANIFEST_PATH, CANDIDATE_MANIFEST_PATH)
    for active_wave_path in ACTIVE_BANK_DIRECTORY.glob("*.wav"):
        shutil.copy2(active_wave_path, CANDIDATE_BANK_DIRECTORY / active_wave_path.name)


def prepare_candidate_loop(
    source_mono_audio: numpy.ndarray,
    source_sample_rate: int,
    source_file_hash: str,
    base_loop_asset: dict,
    specification: dict,
) -> tuple[dict, dict]:
    source_start_frame = round(specification["window_start_seconds"] * source_sample_rate)
    source_end_frame = source_start_frame + round(WINDOW_DURATION_SECONDS * source_sample_rate)
    source_audio_segment = source_mono_audio[source_start_frame:source_end_frame]
    if len(source_audio_segment) != round(WINDOW_DURATION_SECONDS * source_sample_rate):
        raise ValueError(f"Source window is incomplete for {base_loop_asset['id']}.")

    centered_source_audio_segment = source_audio_segment - float(numpy.mean(source_audio_segment))
    frequency, power = spectrum(centered_source_audio_segment, source_sample_rate)
    shaft_frequency_hertz, estimator_score = estimate_shaft_frequency(frequency, power)
    measured_source_revolutions_per_minute = shaft_frequency_hertz * 60.0
    half_window_revolutions_per_minute = [
        measure_revolutions_per_minute(source_audio_segment[: source_sample_rate // 2], source_sample_rate),
        measure_revolutions_per_minute(source_audio_segment[source_sample_rate // 2 :], source_sample_rate),
    ]
    half_window_relative_revolutions_per_minute_spread = (
        max(half_window_revolutions_per_minute) - min(half_window_revolutions_per_minute)
    ) / max(float(numpy.mean(half_window_revolutions_per_minute)), 1.0)
    if half_window_relative_revolutions_per_minute_spread > MAXIMUM_HALF_WINDOW_REVOLUTIONS_PER_MINUTE_SPREAD:
        raise ValueError(f"Source window is not steady enough for {base_loop_asset['id']}.")

    isolated_engine_audio, _ = tonal_residual(
        centered_source_audio_segment,
        source_sample_rate,
        shaft_frequency_hertz,
    )
    isolated_engine_energy_fraction = float(
        numpy.sum(isolated_engine_audio**2)
        / (numpy.sum(centered_source_audio_segment**2) + 1e-30)
    )
    if isolated_engine_energy_fraction < MINIMUM_ISOLATED_ENGINE_ENERGY_FRACTION:
        raise ValueError(f"Harmonic engine isolation is too weak for {base_loop_asset['id']}.")

    seam_score, loop_start_frame, loop_length_frames, loop_cycles = find_engine_loop(
        isolated_engine_audio,
        source_sample_rate,
        shaft_frequency_hertz,
        LOOP_CYCLES_720_DEGREES,
    )
    crossfade_frames = min(LOOP_CROSSFADE_FRAMES, loop_length_frames // 4)
    isolated_engine_loop = make_crossfaded_loop(
        isolated_engine_audio,
        loop_start_frame,
        loop_length_frames,
        crossfade_frames,
    )
    measured_true_peak = calculate_true_peak_amplitude(isolated_engine_loop)
    if measured_true_peak <= 1e-12:
        raise ValueError(f"Isolated loop is silent for {base_loop_asset['id']}.")
    target_true_peak = 10.0 ** (TARGET_TRUE_PEAK_DECIBELS_FULL_SCALE / 20.0)
    normalized_engine_loop = isolated_engine_loop * target_true_peak / measured_true_peak
    normalized_engine_loop_bytes = encode_mono_sixteen_bit_pulse_code_modulation_wave_file(
        normalized_engine_loop,
        OUTPUT_SAMPLE_RATE,
    )
    candidate_wave_path = CANDIDATE_BANK_DIRECTORY / base_loop_asset["derived_filename"]
    candidate_wave_path.write_bytes(normalized_engine_loop_bytes)

    decoded_engine_loop, decoded_sample_rate, decoded_frame_count = (
        decode_mono_sixteen_bit_pulse_code_modulation_wave_file(candidate_wave_path)
    )
    if decoded_sample_rate != OUTPUT_SAMPLE_RATE or decoded_frame_count != len(decoded_engine_loop):
        raise ValueError(f"Candidate WAV format is incompatible for {base_loop_asset['id']}.")
    candidate_band_root_mean_square = calculate_band_root_mean_square(
        decoded_engine_loop,
        decoded_sample_rate,
        300.0,
        6000.0,
    )
    previous_band_root_mean_square = float(base_loop_asset["derived_band_rms_300_6000"])
    previous_calibrated_band_level = (
        previous_band_root_mean_square * float(base_loop_asset["calibrated_gain"])
    )
    if candidate_band_root_mean_square <= 1e-12:
        raise ValueError(f"Candidate loop has no useful 300-6000 Hz engine energy for {base_loop_asset['id']}.")
    calibrated_gain = previous_calibrated_band_level / candidate_band_root_mean_square
    effective_revolutions_per_minute = (
        decoded_sample_rate * 120.0 * loop_cycles / loop_length_frames
    )
    active_coverage = list(base_loop_asset["active_coverage_revolutions_per_minute"])
    candidate_loop_asset = dict(base_loop_asset)
    candidate_loop_asset.update(
        {
            "source_filename": (
                f"r24-sound.mp3@{specification['window_start_seconds']:.1f}-"
                f"{specification['window_start_seconds'] + WINDOW_DURATION_SECONDS:.1f}s"
            ),
            "source_sha256": source_file_hash,
            "derived_sha256": calculate_sha256(normalized_engine_loop_bytes),
            "derived_frames": int(len(decoded_engine_loop)),
            "derived_rms_dbfs": calculate_decibels_full_scale(
                calculate_root_mean_square(decoded_engine_loop)
            ),
            "derived_peak_dbfs": calculate_decibels_full_scale(
                float(numpy.max(numpy.abs(decoded_engine_loop)))
            ),
            "derived_band_rms_300_6000": candidate_band_root_mean_square,
            "comb_spacing_hz": effective_revolutions_per_minute / 120.0,
            "reference_revolutions_per_minute": effective_revolutions_per_minute,
            "calibrated_gain": calibrated_gain,
            "valid_playback_rate_min": min(
                active_coverage[0] / effective_revolutions_per_minute,
                active_coverage[1] / effective_revolutions_per_minute,
            ),
            "valid_playback_rate_max": max(
                active_coverage[0] / effective_revolutions_per_minute,
                active_coverage[1] / effective_revolutions_per_minute,
            ),
            "loop_start_frame": 0,
            "loop_end_frame_exclusive": int(len(decoded_engine_loop)),
            "loop_crossfade_frames": crossfade_frames,
            "preparation_recipe": "center_downmix_harmonic_isolation_phase_aligned_720_degree_loop_v1",
            "source_window_start_seconds": specification["window_start_seconds"],
            "source_window_end_seconds": specification["window_start_seconds"] + WINDOW_DURATION_SECONDS,
            "source_estimated_revolutions_per_minute": measured_source_revolutions_per_minute,
            "source_estimator_score": estimator_score,
            "source_half_window_revolutions_per_minute": half_window_revolutions_per_minute,
            "source_half_window_relative_revolutions_per_minute_spread": half_window_relative_revolutions_per_minute_spread,
            "isolated_engine_energy_fraction": isolated_engine_energy_fraction,
            "loop_cycles_720_degrees": loop_cycles,
            "loop_seam_score": seam_score,
        }
    )
    source_window_record = {
        "loop_identifier": base_loop_asset["id"],
        "derived_filename": base_loop_asset["derived_filename"],
        "source_window_start_seconds": specification["window_start_seconds"],
        "source_window_end_seconds": specification["window_start_seconds"] + WINDOW_DURATION_SECONDS,
        "source_estimated_revolutions_per_minute": measured_source_revolutions_per_minute,
        "source_estimator_score": estimator_score,
        "source_half_window_revolutions_per_minute": half_window_revolutions_per_minute,
        "source_half_window_relative_revolutions_per_minute_spread": half_window_relative_revolutions_per_minute_spread,
        "isolated_engine_energy_fraction": isolated_engine_energy_fraction,
        "loop_start_source_frame": loop_start_frame,
        "loop_end_source_frame_exclusive": loop_start_frame + loop_length_frames,
        "loop_cycles_720_degrees": loop_cycles,
        "loop_seam_score": seam_score,
        "loop_reference_revolutions_per_minute": effective_revolutions_per_minute,
        "derived_true_peak_target_decibels_full_scale": TARGET_TRUE_PEAK_DECIBELS_FULL_SCALE,
        "derived_band_rms_300_6000": candidate_band_root_mean_square,
        "calibrated_gain": calibrated_gain,
    }
    return candidate_loop_asset, source_window_record


def validate_candidate_manifest(manifest: dict) -> None:
    if len(manifest["loops"]) != 5 or len(manifest["transitions"]) != 4:
        raise ValueError("Candidate bank must keep the current five-loop RPM ladder.")
    if len(manifest["coast_loops"]) != 3 or len(manifest["events"]) != 8:
        raise ValueError("Candidate bank must retain the current coast loops and event assets.")
    for asset_group_name in ("loops", "coast_loops", "events"):
        for asset in manifest[asset_group_name]:
            asset_path = CANDIDATE_BANK_DIRECTORY / asset["derived_filename"]
            if not asset_path.is_file():
                raise ValueError(f"Missing candidate bank asset: {asset_path}")
            if calculate_sha256(asset_path.read_bytes()) != asset["derived_sha256"]:
                raise ValueError(f"Candidate bank SHA-256 mismatch: {asset_path}")
            if asset_group_name != "events":
                if asset["loop_end_frame_exclusive"] - asset["loop_start_frame"] < 4 * asset["loop_crossfade_frames"]:
                    raise ValueError(f"Candidate loop crossfade is invalid: {asset['id']}")
                if asset["valid_playback_rate_min"] > asset["valid_playback_rate_max"]:
                    raise ValueError(f"Candidate playback range is invalid: {asset['id']}")
                if not math.isfinite(asset["calibrated_gain"]) or asset["calibrated_gain"] <= 0.0:
                    raise ValueError(f"Candidate loop gain is invalid: {asset['id']}")


def main() -> None:
    if CANDIDATE_BANK_DIRECTORY.exists():
        raise FileExistsError(f"Candidate bank already exists: {CANDIDATE_BANK_DIRECTORY}")
    if not SOURCE_AUDIO_PATH.is_file():
        raise FileNotFoundError(SOURCE_AUDIO_PATH)

    active_manifest = json.loads(ACTIVE_MANIFEST_PATH.read_text(encoding="utf-8"))
    source_stereo_audio, source_sample_rate = soundfile.read(
        SOURCE_AUDIO_PATH,
        dtype="float64",
        always_2d=True,
    )
    if source_sample_rate != OUTPUT_SAMPLE_RATE or source_stereo_audio.shape[1] != 2:
        raise ValueError("The source recording must decode as 44.1 kHz stereo audio.")
    source_mono_audio = numpy.mean(source_stereo_audio, axis=1)
    source_file_hash = calculate_sha256(SOURCE_AUDIO_PATH.read_bytes())
    copy_active_bank_assets()

    source_window_records = []
    loop_assets_by_identifier = {asset["id"]: asset for asset in active_manifest["loops"]}
    candidate_loop_assets_by_identifier = {}
    for specification in SAMPLE_SPECIFICATIONS:
        loop_identifier = specification["loop_identifier"]
        candidate_loop_asset, source_window_record = prepare_candidate_loop(
            source_mono_audio,
            source_sample_rate,
            source_file_hash,
            loop_assets_by_identifier[loop_identifier],
            specification,
        )
        candidate_loop_assets_by_identifier[loop_identifier] = candidate_loop_asset
        source_window_records.append(source_window_record)

    candidate_manifest = dict(active_manifest)
    candidate_manifest["candidate_variant"] = "four isolated engine-on loop replacements"
    candidate_manifest["preparation_tool"] = "scripts/audio/prepare_formula_one_2030_engine_on_candidates.py"
    candidate_manifest["preparation_tool_revision"] = 1
    candidate_manifest["loops"] = [
        candidate_loop_assets_by_identifier.get(asset["id"], asset)
        for asset in active_manifest["loops"]
    ]
    inventory = {
        "source_audio_path": SOURCE_AUDIO_PATH.relative_to(ROOT_DIRECTORY).as_posix(),
        "source_audio_sha256": source_file_hash,
        "source_sample_rate": source_sample_rate,
        "source_channels": int(source_stereo_audio.shape[1]),
        "downmix": "arithmetic mean to mono",
        "engine_isolation": "project order-harmonic soft mask; non-harmonic residual excluded",
        "rpm_method": "scripts/audio/analyze_v10_references.py estimate_shaft_frequency",
        "rpm_and_throttle_limitation": "Source recording has no synchronized RPM or throttle telemetry; RPM values are spectral estimates and ON means powered-loop role only.",
        "window_duration_seconds": WINDOW_DURATION_SECONDS,
        "loop_cycles_720_degrees": LOOP_CYCLES_720_DEGREES,
        "target_true_peak_decibels_full_scale": TARGET_TRUE_PEAK_DECIBELS_FULL_SCALE,
        "source_windows": source_window_records,
    }
    inventory_bytes = write_json(SOURCE_INVENTORY_PATH, inventory)
    candidate_manifest["source_inventory_sha256"] = calculate_sha256(inventory_bytes)
    write_json(CANDIDATE_MANIFEST_PATH, candidate_manifest)
    validate_candidate_manifest(candidate_manifest)

    for asset in candidate_manifest["loops"]:
        if asset["id"] in candidate_loop_assets_by_identifier:
            print(
                f"{asset['id']}: source={asset['source_window_start_seconds']:.1f}s "
                f"estimated={asset['source_estimated_revolutions_per_minute']:.0f} revolutions per minute "
                f"reference={asset['reference_revolutions_per_minute']:.0f} revolutions per minute "
                f"gain={asset['calibrated_gain']:.3f} file={asset['derived_filename']}"
            )
    print(f"Candidate bank: {CANDIDATE_BANK_DIRECTORY}")


if __name__ == "__main__":
    main()
