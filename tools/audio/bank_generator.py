"""Bank generator — build the runtime bank from sources and deterministic synthesis.

Reads or synthesizes each entry in `bank_spec.BANK_SPEC`, writes 44.1 kHz mono
PCM16 WAVs plus `bank_manifest.json`. Same sources + same code => same bytes.
"""

from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np
from scipy import signal

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
    """Mid-high dominant undertray scrape: HPF at 280Hz, 1.4-4.8kHz friction, delicate spark sheen and silky 16ms attack."""
    seed = 9002
    duration_s = 0.36
    frame_count = int(round(duration_s * sample_rate))
    t = np.arange(frame_count, dtype=np.float64) / sample_rate
    rng = np.random.default_rng(seed)

    # 1. Subtle, silky S-curve attack (16ms) with smooth exponential body and gentle release
    t_att = np.clip(t / 0.016, 0.0, 1.0)
    attack = 0.5 * (1.0 - np.cos(np.pi * t_att))
    decay = 0.45 * np.exp(-t / 0.15) + 0.55 * np.exp(-t / 0.08)
    tail_fade = 0.5 * (1.0 + np.cos(np.pi * np.clip((t - (duration_s - 0.035)) / 0.035, 0.0, 1.0)))
    body_env = attack * decay * tail_fade

    # 2. Layer 1: Subtle physical anchor (7% mix, non-intrusive)
    pitch_decay = np.exp(-t / 0.035)
    f1 = 120.0 + 20.0 * pitch_decay
    f2 = 240.0
    thump = (
        0.60 * np.sin(2.0 * np.pi * f1 * t)
        + 0.40 * np.sin(2.0 * np.pi * f2 * t + 0.7)
    ) * np.exp(-t / 0.07) * attack

    # 3. Layer 2: Dominant Mid-High Asphalt Friction (1400 Hz - 4800 Hz)
    white1 = rng.standard_normal(frame_count)
    b_pink = [0.049922035, -0.095993537, 0.050612699, -0.004408786]
    a_pink = [1.0, -2.494956002, 2.017265875, -0.522189400]
    pink = signal.lfilter(b_pink, a_pink, white1)
    pink = pink / max(float(np.max(np.abs(pink))), 1e-9)

    # Bandpass focused in 1400 - 4800 Hz (rich mid-high texture)
    sos_midhigh = signal.butter(4, [1400.0, 4800.0], btype="bandpass", fs=sample_rate, output="sos")
    friction = signal.sosfilt(sos_midhigh, pink)
    friction /= max(float(np.max(np.abs(friction))), 1e-9)

    # Asphalt grain micro-modulation (36/72 Hz)
    grain_mod = 0.80 + 0.20 * (
        np.sin(2.0 * np.pi * 36.0 * t) * np.sin(2.0 * np.pi * 18.0 * t + 0.4)
        + 0.4 * np.sin(2.0 * np.pi * 72.0 * t + 0.9)
    )
    friction = friction * grain_mod * body_env

    # 4. Layer 3: Titanium Spark Sizzle (4.5 kHz - 9.0 kHz)
    prob_spark = 0.035
    sparks_raw = rng.uniform(0.0, 1.0, frame_count) < prob_spark
    spark_noise = rng.standard_normal(frame_count) * sparks_raw

    sos_sparks = signal.butter(4, [4500.0, 9000.0], btype="bandpass", fs=sample_rate, output="sos")
    sparks = signal.sosfilt(sos_sparks, spark_noise)
    sparks /= max(float(np.max(np.abs(sparks))), 1e-9)
    sparks = sparks * body_env * (0.75 + 0.25 * rng.uniform(0.0, 1.0, frame_count))

    # 5. Composite Mix with Mid-High Dominance (7% thump, 65% mid-high friction, 28% sparks)
    composite = 0.07 * thump + 0.65 * friction + 0.28 * sparks

    # Global High-Pass Filter at 280 Hz to cleanly decouple from engine low end
    sos_hpf = signal.butter(3, 280.0, btype="highpass", fs=sample_rate, output="sos")
    composite = signal.sosfilt(sos_hpf, composite)

    # Smooth parametric de-harshing (-3dB around 3.6 kHz)
    b_notch, a_notch = signal.iirnotch(3600.0, 2.5, fs=sample_rate)
    composite = 0.70 * composite + 0.30 * signal.lfilter(b_notch, a_notch, composite)

    # Warm analog saturation
    saturated = np.tanh(composite * 1.5)

    # Clean low-pass above 11 kHz
    sos_lp = signal.butter(2, 11000.0, btype="lowpass", fs=sample_rate, output="sos")
    final_sig = signal.sosfilt(sos_lp, saturated)
    final_sig = final_sig - np.mean(final_sig)
    peak = float(np.max(np.abs(final_sig)))
    if peak > 1e-9:
        final_sig = final_sig * (0.28 / peak)

    return final_sig.tolist(), {
        "recipe": "flat_floor_scrape_v2",
        "seed": seed,
        "duration_s": duration_s,
        "target_peak": 0.28,
        "attack_ms": 16.0,
        "components": [
            "16ms_s_curve_attack",
            "global_hpf_280hz",
            "midhigh_1400_4800hz_friction",
            "poisson_titanium_sparks_4500_9000hz",
            "parametric_3600hz_softening",
            "tanh_warmth",
        ],
    }


def _enhance_backfire(arr: np.ndarray, sample_rate: int, seed: int) -> np.ndarray:
    """Additive enhancement for backfire samples: shockwave snap, exhaust tube resonance, micro-pops, and combustion sheen."""
    frame_count = len(arr)
    t = np.arange(frame_count, dtype=np.float64) / sample_rate
    rng = np.random.default_rng(seed)

    # 1. Shockwave detonation snap (0-25ms) with sub-punch & high-velocity chirp
    t_snap = np.clip(t / 0.022, 0.0, 1.0)
    snap_env = (1.0 - t_snap) * np.exp(-t / 0.008)
    chirp_f = 4500.0 * np.exp(-t / 0.005) + 85.0
    snap_tone = np.sin(2.0 * np.pi * chirp_f * t)
    sos_snap = signal.butter(4, [2200.0, 7500.0], btype="bandpass", fs=sample_rate, output="sos")
    snap_noise = signal.sosfilt(sos_snap, rng.standard_normal(frame_count))
    snap_noise /= max(float(np.max(np.abs(snap_noise))), 1e-9)
    snap = (0.55 * snap_tone + 0.45 * snap_noise) * snap_env

    # 2. Inconel exhaust pipe modal ringing (15-90ms)
    ring_env = np.clip(t / 0.003, 0.0, 1.0) * np.exp(-t / 0.045)
    f1, f2, f3, f4 = 650.0, 1320.0, 1980.0, 2650.0
    ring = (
        0.35 * np.sin(2.0 * np.pi * f1 * t)
        + 0.30 * np.sin(2.0 * np.pi * f2 * t + 0.5)
        + 0.20 * np.sin(2.0 * np.pi * f3 * t + 1.1)
        + 0.15 * np.sin(2.0 * np.pi * f4 * t + 1.8)
    ) * ring_env

    # 3. Afterfire sputter / micro-pops (25-160ms)
    micro_pops = np.zeros(frame_count, dtype=np.float64)
    pop_offsets_ms = [28, 62, 105] if seed % 2 == 1 else [34, 75, 120]
    pop_gains = [0.45, 0.25, 0.12]
    for ms, gain in zip(pop_offsets_ms, pop_gains):
        idx = int(round(ms * 0.001 * sample_rate))
        if idx < frame_count:
            pop_dur = int(round(0.015 * sample_rate))
            end_idx = min(frame_count, idx + pop_dur)
            sub_t = t[:end_idx - idx]
            pop_env = (1.0 - sub_t / 0.015) * np.exp(-sub_t / 0.004)
            pop_sig = rng.standard_normal(len(sub_t)) * pop_env * gain
            micro_pops[idx:end_idx] += pop_sig
    sos_pops = signal.butter(4, [1800.0, 6500.0], btype="bandpass", fs=sample_rate, output="sos")
    micro_pops = signal.sosfilt(sos_pops, micro_pops)

    # 4. Combustion sheen & air (5.5 - 11.5 kHz)
    sheen_env = np.clip(t / 0.004, 0.0, 1.0) * np.exp(-t / 0.08)
    sos_sheen = signal.butter(4, [5500.0, 11500.0], btype="bandpass", fs=sample_rate, output="sos")
    sheen = signal.sosfilt(sos_sheen, rng.standard_normal(frame_count))
    sheen /= max(float(np.max(np.abs(sheen))), 1e-9)
    sheen = sheen * sheen_env

    # 5. Composite blend: original base + additive layers + warm saturation
    enhanced = arr + 0.45 * snap + 0.25 * ring + 0.35 * micro_pops + 0.30 * sheen
    saturated = np.tanh(enhanced * 1.3)
    return saturated


def _build_entry(spec: SpecEntry, source_dir: Path, sample_rate: int) -> tuple[list[float], dict]:
    if spec.synthesis in ("flat_floor_scrape_v1", "flat_floor_scrape_v2"):
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

    # Apply additive synthesis enhancement to backfire samples
    enhancement_info = None
    if spec.key in ("int_backfire", "int_backfire_2"):
        seed = 9005 if spec.key == "int_backfire" else 9006
        arr = _enhance_backfire(arr, sample_rate, seed)
        enhancement_info = [
            "detonation_shockwave_snap",
            "inconel_exhaust_modal_resonance",
            "afterfire_sputter_micro_pops",
            "combustion_sheen_air",
        ]

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
    if enhancement_info is not None:
        params["additive_enhancement"] = enhancement_info
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
