"""Generate spectral-morph intermediate engine loops for the canonical V10 bank.

Each intermediate interpolates the harmonic magnitudes of two neighbouring
captures while keeping the phase of the nearer capture, on an exact
samples-per-cycle grid so the loop always contains an integer number of engine
cycles. The historical GP3 coast maximum is imported byte for byte as the real
high-rpm off capture.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import wave
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from scipy.signal import resample

from tools.audio import build_canonical_v10_engine_bank as canonical

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BANK_DIRECTORY = ROOT / "game/sounds/banks/v10-v2-bank"
DEFAULT_HISTORICAL_BANK_DIRECTORY = ROOT / "game/sounds/banks/v10-gp3"
METADATA_FILENAME = "engine_intermediates.json"
GENERATOR_NAME = "tools/audio/generate_v10_engine_intermediates.py"
GENERATOR_REVISION = 1
SAMPLE_RATE = 44100
SAMPLES_PER_CYCLE_NUMERATOR = float(SAMPLE_RATE * canonical.REVOLUTIONS_PER_MINUTE_PER_HERTZ)
MORPH_FRAME_CYCLES = 16
MORPH_HOP_DIVISOR = 4
MAXIMUM_LOOP_SECONDS = 8.0
PCM24_SCALE = 8388608.0
CYCLE_BLOCK = 4

SOURCE_REVOLUTIONS_PER_MINUTE = {
    "engine_idle.wav": 4531.638723634397,
    "engine_on_low.wav": 9135.225375626043,
    "engine_on_med.wav": 12750.48991699179,
    "engine_on_high.wav": 17867.015566699003,
    "engine_off_low.wav": 7922.604938131586,
    "engine_off_med.wav": 12846.525800429074,
    "engine_off_high.wav": 12465.053474519184,
    "engine_off_maximum.wav": 16542.744529515698,
}


@dataclass(frozen=True)
class HistoricalImport:
    output_filename: str
    source_filename: str


@dataclass(frozen=True)
class SpectralMorph:
    output_filename: str
    phase_source_filename: str
    second_source_filename: str
    second_source_weight: float
    zone_revolutions_per_minute: float


HISTORICAL_IMPORTS = (
    HistoricalImport("engine_off_maximum.wav", "engine_coast_maximum.wav"),
)

SPECTRAL_MORPHS = (
    SpectralMorph(
        "engine_on_idle_low.wav",
        "engine_idle.wav",
        "engine_on_low.wav",
        0.400,
        6000.0,
    ),
    SpectralMorph(
        "engine_on_low_med.wav",
        "engine_on_low.wav",
        "engine_on_med.wav",
        0.306,
        10119.288,
    ),
    SpectralMorph(
        "engine_on_med_high.wav",
        "engine_on_med.wav",
        "engine_on_high.wav",
        0.300,
        14130.818,
    ),
    SpectralMorph(
        "engine_off_low_med.wav",
        "engine_off_high.wav",
        "engine_off_low.wav",
        0.456,
        10119.288,
    ),
    SpectralMorph(
        "engine_off_med_high.wav",
        "engine_off_med.wav",
        "engine_off_maximum.wav",
        0.372,
        14130.818,
    ),
)


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def samples_per_cycle_for_zone(zone_revolutions_per_minute: float) -> int:
    return max(2, round(SAMPLES_PER_CYCLE_NUMERATOR / zone_revolutions_per_minute))


def cycle_count_for(samples: np.ndarray, reference_revolutions_per_minute: float) -> int:
    _, cycles = canonical.derive_reference(
        samples.size,
        reference_revolutions_per_minute / canonical.REVOLUTIONS_PER_MINUTE_PER_HERTZ,
    )
    return cycles


def resample_to_cycle_grid(
    samples: np.ndarray, cycles: int, samples_per_cycle: int
) -> np.ndarray:
    target_length = cycles * samples_per_cycle
    resampled = resample(samples, target_length)
    return np.asarray(resampled, dtype=np.float64)


def morph_cycle_loops(
    phase_samples: np.ndarray,
    second_samples: np.ndarray,
    second_weight: float,
    samples_per_cycle: int,
    cycles: int,
) -> np.ndarray:
    total = cycles * samples_per_cycle
    frame = MORPH_FRAME_CYCLES * samples_per_cycle
    hop = frame // MORPH_HOP_DIVISOR
    window = np.hanning(frame)
    accumulated = np.zeros(total, dtype=np.float64)
    normalization = np.zeros(total, dtype=np.float64)
    index_grid = np.arange(frame)
    for offset in range(0, total, hop):
        indices = (index_grid + offset) % total
        phase_spectrum = np.fft.rfft(phase_samples[indices] * window)
        second_spectrum = np.fft.rfft(second_samples[indices] * window)
        magnitude = (np.abs(phase_spectrum) ** (1.0 - second_weight)) * (
            np.abs(second_spectrum) ** second_weight
        )
        blended = magnitude * np.exp(1j * np.angle(phase_spectrum))
        accumulated[indices] += np.fft.irfft(blended, n=frame) * window
        normalization[indices] += window * window
    return accumulated / np.maximum(normalization, 1e-9)


def encode_pcm24(samples: np.ndarray) -> bytes:
    quantized = np.clip(
        np.round(samples * PCM24_SCALE), -PCM24_SCALE, PCM24_SCALE - 1.0
    ).astype("<i4")
    return quantized.view(np.uint8).reshape(-1, 4)[:, :3].tobytes()


def wave_mono24_bytes(samples: np.ndarray) -> bytes:
    buffer = io.BytesIO()
    with wave.open(buffer, "wb") as writer:
        writer.setnchannels(1)
        writer.setsampwidth(3)
        writer.setframerate(SAMPLE_RATE)
        writer.writeframes(encode_pcm24(samples))
    return buffer.getvalue()


def build_intermediates(
    bank_directory: Path, historical_bank_directory: Path
) -> tuple[dict, dict[str, bytes]]:
    outputs: dict[str, bytes] = {}
    generated_samples: dict[str, np.ndarray] = {}
    engine_source_records: dict[str, dict] = {}
    historical_records: list[dict] = []
    intermediate_records: list[dict] = []

    def load_source(filename: str) -> tuple[np.ndarray, int, str]:
        reference = SOURCE_REVOLUTIONS_PER_MINUTE[filename]
        if filename in generated_samples:
            samples = generated_samples[filename]
            digest = sha256_bytes(outputs[filename])
        else:
            path = bank_directory / filename
            digest = sha256_bytes(path.read_bytes())
            _, samples = canonical.read_wave_mono(path)
        cycles = cycle_count_for(samples, reference)
        if filename not in engine_source_records:
            engine_source_records[filename] = {
                "filename": filename,
                "sha256": digest,
                "frames": int(samples.size),
                "integer_cycle_count": int(cycles),
                "reference_revolutions_per_minute": float(reference),
            }
        return samples, cycles, digest

    for importation in HISTORICAL_IMPORTS:
        source_path = historical_bank_directory / importation.source_filename
        payload = source_path.read_bytes()
        _, samples = canonical.read_wave_mono(source_path)
        reference = SOURCE_REVOLUTIONS_PER_MINUTE[importation.output_filename]
        cycles = cycle_count_for(samples, reference)
        outputs[importation.output_filename] = payload
        generated_samples[importation.output_filename] = samples
        historical_records.append(
            {
                "filename": importation.source_filename,
                "path": source_path.relative_to(ROOT).as_posix(),
                "sha256": sha256_bytes(payload),
                "frames": int(samples.size),
                "integer_cycle_count": int(cycles),
                "reference_revolutions_per_minute": float(reference),
            }
        )
        intermediate_records.append(
            {
                "output_filename": importation.output_filename,
                "recipe": "historical_gp3_coast_maximum_import",
                "source_filename": importation.source_filename,
                "source_sha256": sha256_bytes(payload),
                "frames": int(samples.size),
                "integer_cycle_count": int(cycles),
                "reference_revolutions_per_minute": float(reference),
                "output_sha256": sha256_bytes(payload),
            }
        )

    for morph in SPECTRAL_MORPHS:
        samples_per_cycle = samples_per_cycle_for_zone(morph.zone_revolutions_per_minute)
        cycle_frequency_hertz = SAMPLE_RATE / samples_per_cycle
        reference = cycle_frequency_hertz * canonical.REVOLUTIONS_PER_MINUTE_PER_HERTZ
        phase_samples, phase_cycles, phase_digest = load_source(morph.phase_source_filename)
        second_samples, second_cycles, second_digest = load_source(
            morph.second_source_filename
        )
        target_cycles = int(MAXIMUM_LOOP_SECONDS * cycle_frequency_hertz)
        cycles = min(phase_cycles, second_cycles, target_cycles)
        cycles = max(CYCLE_BLOCK, cycles // CYCLE_BLOCK * CYCLE_BLOCK)
        length = cycles * samples_per_cycle
        phase_grid = resample_to_cycle_grid(
            phase_samples, phase_cycles, samples_per_cycle
        )[:length]
        second_grid = resample_to_cycle_grid(
            second_samples, second_cycles, samples_per_cycle
        )[:length]
        phase_aligned, _ = canonical.rotate_to_reference_phase(
            phase_grid, cycle_frequency_hertz
        )
        second_aligned, _ = canonical.rotate_to_reference_phase(
            second_grid, cycle_frequency_hertz
        )
        morphed = morph_cycle_loops(
            phase_aligned,
            second_aligned,
            morph.second_source_weight,
            samples_per_cycle,
            cycles,
        )
        payload = wave_mono24_bytes(morphed)
        outputs[morph.output_filename] = payload
        generated_samples[morph.output_filename] = morphed
        intermediate_records.append(
            {
                "output_filename": morph.output_filename,
                "recipe": "spectral_magnitude_morph_integer_cycles",
                "phase_source_filename": morph.phase_source_filename,
                "phase_source_sha256": phase_digest,
                "second_source_filename": morph.second_source_filename,
                "second_source_sha256": second_digest,
                "second_source_weight": float(morph.second_source_weight),
                "samples_per_cycle": int(samples_per_cycle),
                "cycle_frequency_hertz": float(cycle_frequency_hertz),
                "reference_revolutions_per_minute": float(reference),
                "frames": int(morphed.size),
                "integer_cycle_count": int(cycles),
                "output_sha256": sha256_bytes(payload),
            }
        )

    metadata = {
        "schema_version": 1,
        "generator": GENERATOR_NAME,
        "generator_revision": GENERATOR_REVISION,
        "sample_rate": SAMPLE_RATE,
        "engine_sources": [
            engine_source_records[key] for key in sorted(engine_source_records)
        ],
        "historical_sources": historical_records,
        "intermediates": intermediate_records,
    }
    return metadata, outputs


def write_intermediates(
    bank_directory: Path, metadata: dict, outputs: dict[str, bytes]
) -> None:
    for filename, payload in sorted(outputs.items()):
        (bank_directory / filename).write_bytes(payload)
    (bank_directory / METADATA_FILENAME).write_bytes(canonical.serialize_json(metadata))
    print(f"wrote {len(outputs)} generated engine sources to {bank_directory}")


def main() -> int:
    argument_parser = argparse.ArgumentParser()
    argument_parser.add_argument("--bank-directory", type=Path, default=DEFAULT_BANK_DIRECTORY)
    argument_parser.add_argument(
        "--historical-bank-directory",
        type=Path,
        default=DEFAULT_HISTORICAL_BANK_DIRECTORY,
    )
    argument_parser.add_argument("--check", action="store_true")
    arguments = argument_parser.parse_args()

    metadata, outputs = build_intermediates(
        arguments.bank_directory, arguments.historical_bank_directory
    )
    if arguments.check:
        exit_code = 0
        for filename, expected in sorted(outputs.items()):
            path = arguments.bank_directory / filename
            actual = path.read_bytes() if path.is_file() else b""
            if actual != expected:
                print(f"generated engine source differs: {path}")
                exit_code = 1
        metadata_path = arguments.bank_directory / METADATA_FILENAME
        expected_metadata = canonical.serialize_json(metadata)
        actual_metadata = metadata_path.read_bytes() if metadata_path.is_file() else b""
        if actual_metadata != expected_metadata:
            print(f"generated metadata differs: {metadata_path}")
            exit_code = 1
        if exit_code == 0:
            print("generated engine sources match the shipped bank")
        return exit_code

    write_intermediates(arguments.bank_directory, metadata, outputs)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
