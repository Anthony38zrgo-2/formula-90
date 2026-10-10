from __future__ import annotations

import argparse
import hashlib
from pathlib import Path

from tools.audio.bank_manifest import BankManifest, FileEntry, sha256_file
from tools.audio.dsp_common import dc_offset, lufs_approx, peak_abs, read_wav_mono, remove_dc, resample_mono, write_wav_mono16
from tools.common.output_policy import validate_output_path

PREPARATION_REVISION = 1
RUNTIME_FILENAME = "neutral_first_gear_transition.wav"


def prepare(source: Path, bank_directory: Path, repository: Path) -> FileEntry:
    source_rate, recording = read_wav_mono(source)
    samples = remove_dc(resample_mono(recording, source_rate, 44100).tolist())
    for position in range(min(88, len(samples))):
        samples[position] *= position / 87
    for position in range(min(220, len(samples))):
        samples[-1 - position] *= position / 219
    samples = remove_dc(samples)
    maximum = peak_abs(samples)
    if maximum > 0.92:
        samples = [value * 0.92 / maximum for value in samples]
    output = bank_directory / RUNTIME_FILENAME
    validate_output_path(repository, output, "promote")
    write_wav_mono16(output, samples, 44100)
    entry = FileEntry(
        file=RUNTIME_FILENAME, role="neutral_first_gear_transition", loop=False,
        duration_s=round(len(samples) / 44100, 6), peak=round(peak_abs(samples), 6),
        dc_offset=round(dc_offset(samples), 6), loudness_dbfs=round(lufs_approx(samples), 2),
        provenance="derived from original user-supplied replacement samples",
        synthesis={"category": "shift", "source_file": source.resolve().relative_to(repository.resolve()).as_posix(),
                   "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(), "source_rate": source_rate,
                   "promotion_recipe": "neutral_first_gear_transition_v1", "tool_revision": PREPARATION_REVISION},
        sha256=sha256_file(output),
    )
    manifest_path = bank_directory / "bank_manifest.json"
    manifest = BankManifest.load(manifest_path)
    manifest.files = [existing for existing in manifest.files if existing.file != RUNTIME_FILENAME] + [entry]
    validate_output_path(repository, manifest_path, "promote")
    manifest.write(manifest_path)
    return entry


def main() -> None:
    repository = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, default=repository / "game/sounds/bank_sources/neutral_first_gear_transition_source.wav")
    parser.add_argument("--bank", type=Path, default=repository / "game/sounds/banks/commons")
    arguments = parser.parse_args()
    print(prepare(arguments.source, arguments.bank, repository))


if __name__ == "__main__":
    main()
