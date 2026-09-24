from __future__ import annotations

import hashlib
import json
from pathlib import Path

import numpy
import soundfile
from scipy import signal


REPOSITORY_DIRECTORY = Path(__file__).resolve().parents[2]
SOURCE_DIRECTORY = REPOSITORY_DIRECTORY / "game/sounds/banks/v10-gp3/backup"
OUTPUT_DIRECTORY = REPOSITORY_DIRECTORY / "reports/audio-v10/metallic-engine/candidates/first-pass"
SAMPLE_RATE = 44100
OVERSAMPLING_FACTOR = 4
TRUE_PEAK_CEILING_DECIBELS = -0.5
SOURCE_HASHES = {
    "98_int_low.wav": "2eac0ef746c818ecac50fe77ca0099894f4762d55428a3cddfeb3007a6092cc5",
    "98_int_med.wav": "d1a527d5ddf3ec6d4f38127d1c6a3b00d7b7771d5473f68315e4352c98db4007",
    "98_int_high_1.wav": "fc68bc279e3f8098be009be24eb4249c4ab76ab805e595af7bc5c509b1413477",
    "98_int_max_5.wav": "f6b6cd4389fee32cb5f4b41a550e7aa146d4a5d0912fda86551dabfd598aa50b",
}
HARMONIC_LAYER_PROPORTIONS = {
    "98_int_low.wav": 0.30,
    "98_int_med.wav": 0.28,
    "98_int_high_1.wav": 0.035,
    "98_int_max_5.wav": 0.029,
}
FREQUENCY_ANCHORS = numpy.array((0, 1500, 2000, 3000, 5000, 8000, 11000, 16000, 22050))
GAIN_ANCHORS_DECIBELS = numpy.array((0, 0, 0.3, 1.8, 3.0, 3.2, 1.0, 0, 0))
BANDS = ((20, 250), (250, 2000), (2000, 5000), (5000, 10000), (10000, 20000))


def file_hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def root_mean_square(samples: numpy.ndarray) -> float:
    return float(numpy.sqrt(numpy.mean(numpy.square(samples))))


def amplitude_decibels(amplitude: float) -> float:
    return float(20 * numpy.log10(max(amplitude, 1e-12)))


def circular_filter(samples: numpy.ndarray, filter_sections: numpy.ndarray) -> numpy.ndarray:
    padding_length = min(8192, len(samples) // 4)
    padded_samples = numpy.concatenate((samples[-padding_length:], samples, samples[:padding_length]))
    filtered_samples = signal.sosfiltfilt(filter_sections, padded_samples)
    return filtered_samples[padding_length:-padding_length]


def equalize_upper_harmonics(samples: numpy.ndarray) -> numpy.ndarray:
    frequencies = numpy.fft.rfftfreq(len(samples), 1 / SAMPLE_RATE)
    gains_decibels = numpy.interp(frequencies, FREQUENCY_ANCHORS, GAIN_ANCHORS_DECIBELS)
    gains = numpy.power(10, gains_decibels / 20)
    return numpy.fft.irfft(numpy.fft.rfft(samples) * gains, n=len(samples))


def generate_harmonic_layer(samples: numpy.ndarray, proportion: float) -> numpy.ndarray:
    source_filter = signal.butter(4, (1800, 4300), btype="bandpass", fs=SAMPLE_RATE, output="sos")
    harmonic_source = circular_filter(samples, source_filter)
    padding_length = min(8192, len(samples) // 4)
    padded_source = numpy.concatenate(
        (harmonic_source[-padding_length:], harmonic_source, harmonic_source[:padding_length])
    )
    oversampled_source = signal.resample_poly(padded_source, OVERSAMPLING_FACTOR, 1)
    generated_harmonics = numpy.power(oversampled_source, 3)
    harmonic_filter = signal.butter(
        4,
        (4300, 10500),
        btype="bandpass",
        fs=SAMPLE_RATE * OVERSAMPLING_FACTOR,
        output="sos",
    )
    filtered_harmonics = signal.sosfiltfilt(harmonic_filter, generated_harmonics)
    downsampled_harmonics = signal.resample_poly(filtered_harmonics, 1, OVERSAMPLING_FACTOR)
    harmonic_layer = downsampled_harmonics[padding_length:-padding_length]
    target_level = root_mean_square(harmonic_source) * proportion
    layer_level = root_mean_square(harmonic_layer)
    if layer_level < 1e-12:
        return numpy.zeros_like(samples)
    return harmonic_layer * (target_level / layer_level)


def true_peak_amplitude(samples: numpy.ndarray) -> float:
    return float(numpy.max(numpy.abs(signal.resample_poly(samples, 8, 1))))


def band_levels(samples: numpy.ndarray) -> dict[str, float]:
    frequencies, power = signal.welch(samples, SAMPLE_RATE, nperseg=8192)
    levels = {}
    for lower_frequency, upper_frequency in BANDS:
        selection = (frequencies >= lower_frequency) & (frequencies < upper_frequency)
        band_power = float(numpy.trapezoid(power[selection], frequencies[selection]))
        levels[f"{lower_frequency}-{upper_frequency}"] = float(
            10 * numpy.log10(max(band_power, 1e-20))
        )
    return levels


def measurements(samples: numpy.ndarray) -> dict:
    return {
        "sample_count": len(samples),
        "duration_seconds": len(samples) / SAMPLE_RATE,
        "direct_current_offset": float(numpy.mean(samples)),
        "root_mean_square_decibels": amplitude_decibels(root_mean_square(samples)),
        "sample_peak_decibels": amplitude_decibels(float(numpy.max(numpy.abs(samples)))),
        "true_peak_decibels": amplitude_decibels(true_peak_amplitude(samples)),
        "band_levels_decibels": band_levels(samples),
        "loop_boundary_step": float(abs(samples[0] - samples[-1])),
    }


def process_source(source_path: Path) -> dict:
    expected_hash = SOURCE_HASHES[source_path.name]
    actual_hash = file_hash(source_path)
    if actual_hash != expected_hash:
        raise ValueError(f"Source changed: {source_path}")
    original_samples, sample_rate = soundfile.read(source_path, dtype="float64")
    if sample_rate != SAMPLE_RATE or original_samples.ndim != 1:
        raise ValueError(f"Unexpected source format: {source_path}")

    centered_samples = original_samples - numpy.mean(original_samples)
    equalized_samples = equalize_upper_harmonics(centered_samples)
    harmonic_layer = generate_harmonic_layer(
        centered_samples, HARMONIC_LAYER_PROPORTIONS[source_path.name]
    )
    combined_samples = equalized_samples + harmonic_layer
    ceiling_amplitude = 10 ** (TRUE_PEAK_CEILING_DECIBELS / 20)
    applied_gain = min(1.0, ceiling_amplitude / true_peak_amplitude(combined_samples))
    processed_samples = combined_samples * applied_gain

    OUTPUT_DIRECTORY.mkdir(parents=True, exist_ok=True)
    output_path = OUTPUT_DIRECTORY / source_path.name
    soundfile.write(output_path, processed_samples, SAMPLE_RATE, subtype="PCM_16")
    exported_samples, exported_rate = soundfile.read(output_path, dtype="float64")
    if exported_rate != SAMPLE_RATE or len(exported_samples) != len(original_samples):
        raise ValueError(f"Export format mismatch: {output_path}")

    source_measurements = measurements(original_samples)
    output_measurements = measurements(exported_samples)
    return {
        "filename": source_path.name,
        "source_sha256": actual_hash,
        "output_sha256": file_hash(output_path),
        "harmonic_layer_proportion": HARMONIC_LAYER_PROPORTIONS[source_path.name],
        "applied_output_gain_decibels": amplitude_decibels(applied_gain),
        "source": source_measurements,
        "output": output_measurements,
        "band_changes_decibels": {
            band: output_measurements["band_levels_decibels"][band]
            - source_measurements["band_levels_decibels"][band]
            for band in source_measurements["band_levels_decibels"]
        },
    }


def main() -> None:
    results = [process_source(SOURCE_DIRECTORY / filename) for filename in SOURCE_HASHES]
    report = {
        "source_directory": str(SOURCE_DIRECTORY),
        "output_directory": str(OUTPUT_DIRECTORY),
        "sample_rate": SAMPLE_RATE,
        "format": "mono 16-bit PCM",
        "true_peak_ceiling_decibels": TRUE_PEAK_CEILING_DECIBELS,
        "frequency_anchors_hertz": FREQUENCY_ANCHORS.tolist(),
        "equalizer_gain_anchors_decibels": GAIN_ANCHORS_DECIBELS.tolist(),
        "results": results,
    }
    report_path = OUTPUT_DIRECTORY / "processing_report.json"
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    for result in results:
        print(
            result["filename"],
            "gain", round(result["applied_output_gain_decibels"], 2),
            "bands", {band: round(change, 2) for band, change in result["band_changes_decibels"].items()},
            "true peak", round(result["output"]["true_peak_decibels"], 2),
        )
    print(report_path)


if __name__ == "__main__":
    main()
