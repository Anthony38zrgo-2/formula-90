from pathlib import Path
import hashlib

import numpy as np
from scipy import signal
from scipy.io import wavfile

from tools.audio.bank_manifest import BankManifest, FileEntry, sha256_file
from tools.audio.dsp_common import lufs_approx
from tools.common.output_policy import validate_output_path


def prepare() -> None:
    repository = Path(__file__).resolve().parents[2]
    source = repository / "game/sounds/bank_sources/underfloor_metallic_scrape_source.wav"
    bank_directory = repository / "game/sounds/banks/commons"
    sample_rate, recording = wavfile.read(source)
    if sample_rate != 44100 or recording.dtype != np.int16:
        raise ValueError("Source must be PCM16 at 44100 Hz")
    samples = recording.astype(np.float64) / 32768.0
    if samples.ndim == 2:
        samples = np.mean(samples, axis=1)
    samples -= np.mean(samples)
    true_peak = float(np.max(np.abs(signal.resample_poly(samples, 4, 1))))
    attenuation = min(1.0, 10 ** (-3.0 / 20.0) / true_peak)
    samples *= attenuation
    output = bank_directory / "impact_scrape.wav"
    validate_output_path(repository, output, "promote")
    quantized = np.rint(samples * 32767.0).astype(np.int16)
    wavfile.write(output, sample_rate, quantized)
    verified = quantized.astype(np.float64) / 32768.0
    entry = FileEntry(
        file=output.name,
        role="impact_scrape",
        loop=False,
        duration_s=round(len(samples) / sample_rate, 6),
        peak=round(float(np.max(np.abs(verified))), 6),
        dc_offset=round(float(np.mean(verified)), 6),
        loudness_dbfs=round(lufs_approx(verified.tolist()), 2),
        provenance="derived from user-supplied undertray_scrape_metallic_p Edit 1 Export 1.wav",
        synthesis={
            "category": "impact",
            "source_file": source.relative_to(repository).as_posix(),
            "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
            "source_rate": sample_rate,
            "promotion_recipe": "underfloor_metallic_scrape_v1",
            "tool_revision": 1,
            "processing": {
                "arithmetic_stereo_downmix": True,
                "remove_mean_offset": True,
                "attenuation_decibels": float(20 * np.log10(attenuation)),
                "true_peak_ceiling_decibels_full_scale": -3.0,
            },
        },
        sha256=sha256_file(output),
    )
    manifest_path = bank_directory / "bank_manifest.json"
    manifest = BankManifest.load(manifest_path)
    manifest.files = [existing for existing in manifest.files if existing.file != output.name] + [entry]
    validate_output_path(repository, manifest_path, "promote")
    manifest.write(manifest_path)
    print(entry)


if __name__ == "__main__":
    prepare()
