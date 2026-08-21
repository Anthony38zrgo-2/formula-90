"""Bank generator — build the runtime bank from sources and deterministic synthesis.

Reads or synthesizes each entry in `bank_spec.BANK_SPEC`, writes 44.1 kHz mono
PCM16 WAVs plus `bank_manifest.json`. Same sources + same code => same bytes.
"""

from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np

from tools.audio.bank_manifest import BankManifest, FileEntry, sha256_file
from tools.audio.bank_spec import BANK_SPEC, DEFAULT_SOURCE_DIR, ENGINE_BAND_NATIVE_RPM, SpecEntry
from tools.audio.dsp_common import (
    SAMPLE_RATE,
    dc_offset,
    lufs_approx,
    make_loop_seamless,
    normalize_peak,
    peak_abs,
    read_wav_mono,
    remove_dc,
    resample_mono,
    write_wav_mono16,
)

DEFAULT_CONFIG = Path("tools/audio/bank_config.yaml")
DEFAULT_OUTPUT = Path("game/sounds/banks/v10_vehicle")

# Loop seam crossfade frames (~46 ms) applied only to loopable entries.
LOOP_XFADE_FRAMES = 2048



def _synthesize_flat_floor_scrape(sample_rate: int) -> tuple[list[float], dict]:
    """Subtle, short floor-plank brush: filtered grit plus a muted body resonance."""
    seed = 9001
    duration_s = 0.22
    frame_count = int(round(duration_s * sample_rate))
    t = np.arange(frame_count, dtype=np.float64) / sample_rate
    rng = np.random.default_rng(seed)

    # Restrained metallic/asphalt texture without the harsh full-band crack of
    # the legacy GP_scrape sample.
    white = rng.standard_normal(frame_count)
    grit = np.concatenate(([0.0], np.diff(white)))
    kernel = np.ones(9, dtype=np.float64) / 9.0
    grit = np.convolve(grit, kernel, mode="same")
    grit /= max(float(np.max(np.abs(grit))), 1e-9)

    attack = np.minimum(t / 0.006, 1.0)
    decay = np.exp(-t / 0.075)
    envelope = attack * decay
    resonance = (
        0.26 * np.sin(2.0 * np.pi * 185.0 * t)
        + 0.10 * np.sin(2.0 * np.pi * 430.0 * t + 0.7)
    ) * np.exp(-t / 0.055)
    samples = (0.52 * grit * envelope + resonance * attack).tolist()
    samples = remove_dc(samples)
    samples = normalize_peak(samples, 0.32)
    return samples, {
        "recipe": "flat_floor_scrape_v1",
        "seed": seed,
        "duration_s": duration_s,
        "target_peak": 0.32,
        "components": ["differentiated_filtered_noise", "185hz_resonance", "430hz_resonance"],
    }

def _build_entry(spec: SpecEntry, source_dir: Path, sample_rate: int) -> tuple[list[float], dict]:
    if spec.synthesis == "flat_floor_scrape_v1":
        samples, params = _synthesize_flat_floor_scrape(sample_rate)
        params["category"] = spec.category
        return samples, params
    if spec.synthesis is not None:
        raise ValueError(f"unknown synthesis recipe: {spec.synthesis}")
    if spec.source is None:
        raise ValueError(f"{spec.key} has neither source nor synthesis recipe")
    src = source_dir / spec.source
    if not src.is_file():
        raise FileNotFoundError(f"source sample missing: {src}")
    rate, arr = read_wav_mono(src)
    arr = resample_mono(arr, rate, sample_rate)
    arr = np.nan_to_num(arr, nan=0.0, posinf=1.0, neginf=-1.0)
    samples = arr.astype(float).tolist()
    samples = remove_dc(samples)
    if spec.loop:
        # Make the raw loop seamless so a hard i % len wrap does not click.
        samples = make_loop_seamless(samples, xfade_frames=LOOP_XFADE_FRAMES)
        samples = remove_dc(samples)
    samples = normalize_peak(samples, 0.89)
    params: dict = {
        "source_file": spec.source,
        "source_rate": rate,
        "category": spec.category,
        "source_sha256": sha256_file(src),
    }
    if spec.key in ENGINE_BAND_NATIVE_RPM:
        params["native_rpm"] = ENGINE_BAND_NATIVE_RPM[spec.key]
    return samples, params


def generate_bank(source_dir: Path, output_dir: Path, sample_rate: int = SAMPLE_RATE) -> BankManifest:
    entries: list[FileEntry] = []
    for spec in BANK_SPEC:
        samples, params = _build_entry(spec, source_dir, sample_rate)
        name = f"{spec.key}.wav"
        write_wav_mono16(output_dir / name, samples, sample_rate)
        dur = len(samples) / sample_rate
        entries.append(
            FileEntry(
                file=name,
                role=spec.role,
                loop=spec.loop,
                duration_s=round(dur, 6),
                loop_start_s=None,
                loop_end_s=None,
                loudness_dbfs=round(lufs_approx(samples), 2),
                peak=round(peak_abs(samples), 6),
                dc_offset=round(dc_offset(samples), 6),
                synthesis=params,
                provenance=(
                    "deterministic procedural synthesis"
                    if spec.synthesis is not None
                    else "derived from original samples (assets-lowpoly-python/sounds)"
                ),
                sha256="",
            )
        )

    # Fill sha256 after write.
    for e in entries:
        e.sha256 = sha256_file(output_dir / e.file)

    manifest = BankManifest(
        bank_name="v10_vehicle",
        sample_rate=sample_rate,
        channels=1,
        pcm_bits=16,
        seed=0,
        generator="formula90s hybrid sample/synthesis bank builder 1.1",
        files=entries,
    )
    manifest.write(output_dir / "bank_manifest.json")
    return manifest


def main() -> int:
    ap = argparse.ArgumentParser(description="Build v10_vehicle bank from sources and deterministic synthesis")
    ap.add_argument("--source", type=Path, default=DEFAULT_SOURCE_DIR)
    ap.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    ap.add_argument("--sample-rate", type=int, default=SAMPLE_RATE)
    args = ap.parse_args()
    if not args.source.is_dir():
        print(f"Source samples dir not found: {args.source}")
        return 2
    m = generate_bank(args.source, args.output, args.sample_rate)
    print(f"Generated {m.bank_name} -> {args.output} ({len(m.files)} files)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
