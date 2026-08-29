"""Bank generator — build the runtime bank from sources and deterministic synthesis.

Reads or synthesizes each entry in `bank_spec.BANK_SPEC`, writes 44.1 kHz mono
PCM16 WAVs plus `bank_manifest.json`. Same sources + same code => same bytes.
"""

from __future__ import annotations

import argparse
from collections.abc import Sequence
from pathlib import Path

import numpy as np
from scipy import signal

from tools.audio.bank_manifest import BankManifest, FileEntry, sha256_file
from tools.audio.bank_spec import BANK_SPEC, DEFAULT_SOURCE_DIR, RETIRED_KEYS, SpecEntry
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
from tools.audio.playback_metadata import playback_for_key
from tools.common.output_policy import OutputMode, OutputPolicyError, validate_output_path

DEFAULT_CONFIG = Path("tools/audio/bank_config.yaml")
DEFAULT_OUTPUT = Path("scratch/audio/v10_vehicle")

# Loop seam crossfade frames (~46 ms) applied only to loopable entries.
LOOP_XFADE_FRAMES = 2048


def _synthesize_flat_floor_scrape(sample_rate: int) -> tuple[list[float], dict]:
    """Mid-high dominant undertray scrape: HPF at 280Hz, 1.4-4.8kHz friction, delicate spark sheen and silky 16ms attack."""
    seed = 9002
    duration_s = 0.36
    frame_count = round(duration_s * sample_rate)
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
        idx = round(ms * 0.001 * sample_rate)
        if idx < frame_count:
            pop_dur = round(0.015 * sample_rate)
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


def _enhance_engine_low(arr: np.ndarray, sample_rate: int, seed: int = 9011) -> np.ndarray:
    """Additive enhancement for engine_low: gear whine mechanical detail and sub-bass de-mud."""
    frame_count = len(arr)
    t = np.arange(frame_count, dtype=np.float64) / sample_rate

    f_whine = 1238.0  # 2x fundamental
    gear_whine = (
        0.50 * np.sin(2.0 * np.pi * f_whine * t)
        + 0.35 * np.sin(2.0 * np.pi * (f_whine * 1.5) * t + 0.4)
        + 0.25 * np.sin(2.0 * np.pi * (f_whine * 2.0) * t + 1.1)
    ) * (0.85 + 0.15 * np.sin(2.0 * np.pi * 25.0 * t))
    sos_whine = signal.butter(4, [1100.0, 3200.0], btype="bandpass", fs=sample_rate, output="sos")
    gear_whine = signal.sosfilt(sos_whine, gear_whine)
    gear_whine /= max(float(np.max(np.abs(gear_whine))), 1e-9)

    enhanced = arr + 0.10 * gear_whine
    saturated = np.tanh(enhanced * 1.2)
    return saturated


def _apply_midrange_sustain(
    arr: np.ndarray,
    sample_rate: int,
    low_f: float = 600.0,
    high_f: float = 1800.0,
    sustain_gain: float = 0.20,
    decay_ms: float = 65.0,
) -> np.ndarray:
    """Extend acoustic decay and sustain in the mid-range via modal reflection comb filters."""
    sos_band = signal.butter(4, [low_f, high_f], btype="bandpass", fs=sample_rate, output="sos")
    mid_isolated = signal.sosfilt(sos_band, arr)

    delay_times = [int(sample_rate * d) for d in (0.011, 0.017, 0.023)]
    decay_factor = np.exp(-1.0 / (sample_rate * (decay_ms / 1000.0)))

    sustain_tail = np.zeros_like(mid_isolated)
    for dt in delay_times:
        g = 0.55 * (decay_factor**dt)
        a_denom = np.zeros(dt + 1)
        a_denom[0] = 1.0
        a_denom[-1] = -g
        sustain_tail += signal.lfilter([1.0], a_denom, mid_isolated)

    sustain_tail /= len(delay_times)
    max_val = float(np.max(np.abs(sustain_tail)))
    if max_val > 1e-9:
        sustain_tail /= max_val

    return arr + sustain_gain * sustain_tail


def _enhance_engine_mid(arr: np.ndarray, sample_rate: int, seed: int = 9012) -> np.ndarray:
    """Additive enhancement for engine_mid: airbox throat growl (750-1800 Hz), 2x harmonic, modal sustain & 3.6kHz LPF."""
    frame_count = len(arr)
    t = np.arange(frame_count, dtype=np.float64) / sample_rate
    rng = np.random.default_rng(seed)

    f_mid0 = 683.0
    mid_harm = 0.50 * np.sin(2.0 * np.pi * (f_mid0 * 2.0) * t) * (0.88 + 0.12 * np.sin(2.0 * np.pi * 30.0 * t))

    sos_growl = signal.butter(4, [750.0, 1800.0], btype="bandpass", fs=sample_rate, output="sos")
    intake_noise = signal.sosfilt(sos_growl, rng.standard_normal(frame_count))
    intake_noise /= max(float(np.max(np.abs(intake_noise))), 1e-9)

    enhanced = arr + 0.12 * mid_harm + 0.10 * intake_noise

    # Modal sustain and decay extension in the mid-range (650-1800 Hz) for strong punch in transitions
    enhanced = _apply_midrange_sustain(enhanced, sample_rate, 650.0, 1800.0, sustain_gain=0.20, decay_ms=65.0)

    # Smooth 3.6 kHz Low-Pass Filter: yields high register (>3.6 kHz) exclusively to engine_high
    sos_mid_lpf = signal.butter(2, 3600.0, btype="lowpass", fs=sample_rate, output="sos")
    enhanced = signal.sosfilt(sos_mid_lpf, enhanced)
    saturated = np.tanh(enhanced * 1.20)
    return saturated


def _enhance_engine_high(arr: np.ndarray, sample_rate: int, seed: int = 9013) -> np.ndarray:
    """Additive enhancement for engine_high with 3-branch decoupled saturation and mid-range modal sustain:

    1. Airbox Intake Growl: 100% linear bypass (0% saturation) for clean airflow.
    2. Rest of Sound (Base + Harmonics + Sheen): Mid-range modal sustain + Notch 3.4kHz + LPF 5.0kHz + minimal saturation (1.02).
    3. Pure Sinusoidal Body Core (465.2/232.4 Hz): Dedicated warm analog saturation (1.25, 28% weight).
    """
    frame_count = len(arr)
    t = np.arange(frame_count, dtype=np.float64) / sample_rate
    rng = np.random.default_rng(seed)

    # Base high-pass at 140 Hz (retains 150-450 Hz chest punch)
    sos_hpf = signal.butter(2, 140.0, btype="highpass", fs=sample_rate, output="sos")
    arr_clean = signal.sosfilt(sos_hpf, arr)

    # --- Rama 1: Airbox Intake Growl (100% Lineal, CERO saturación) ---
    white_turb = rng.standard_normal(frame_count)
    b_helm1, a_helm1 = signal.iirpeak(850.0, 3.0, fs=sample_rate)
    helm1 = signal.lfilter(b_helm1, a_helm1, white_turb)
    b_helm2, a_helm2 = signal.iirpeak(1350.0, 3.5, fs=sample_rate)
    helm2 = signal.lfilter(b_helm2, a_helm2, white_turb)
    intake_air = (0.55 * helm1 + 0.45 * helm2) * (0.85 + 0.15 * np.sin(2.0 * np.pi * 38.0 * t))
    sos_airbox = signal.butter(4, [650.0, 1600.0], btype="bandpass", fs=sample_rate, output="sos")
    intake_growl = signal.sosfilt(sos_airbox, intake_air)
    intake_growl /= max(float(np.max(np.abs(intake_growl))), 1e-9)
    intake_branch = 0.15 * intake_growl

    # --- Rama 2: Onda Sinusoidal Pura (Saturación Cálida Dedicada 1.25, peso 28%) ---
    f_sine1 = 465.2
    f_sine2 = 232.4
    pure_sine_core = 0.65 * np.sin(2.0 * np.pi * f_sine1 * t) + 0.35 * np.sin(2.0 * np.pi * f_sine2 * t)
    sine_branch = np.tanh(pure_sine_core * 1.25) * 0.28

    # --- Rama 3: Resto del Sonido (Saturación Mínima 1.02 con LPF 5.0kHz y Notch 3.4kHz) ---
    f3 = 1395.0 * 1.002
    f4 = 1860.0 * 0.998
    f5 = 2325.0 * 1.003
    f6 = 2790.0 * 0.997
    phase_drift = 0.25 * np.sin(2.0 * np.pi * 2.8 * t)

    hi_harm = (
        0.35 * np.sin(2.0 * np.pi * 930.0 * t + 0.4)
        + 0.28 * np.sin(2.0 * np.pi * f3 * t + phase_drift)
        + 0.20 * np.sin(2.0 * np.pi * f4 * t + phase_drift * 1.3 + 0.5)
        + 0.12 * np.sin(2.0 * np.pi * f5 * t + phase_drift * 0.8 + 1.1)
        + 0.05 * np.sin(2.0 * np.pi * f6 * t + phase_drift * 1.5 + 1.7)
    ) * (0.88 + 0.12 * np.sin(2.0 * np.pi * 38.0 * t))

    sos_sheen = signal.butter(4, [4200.0, 7000.0], btype="bandpass", fs=sample_rate, output="sos")
    sheen_noise = signal.sosfilt(sos_sheen, rng.standard_normal(frame_count))
    sheen_noise /= max(float(np.max(np.abs(sheen_noise))), 1e-9)

    rest_raw = arr_clean + 0.16 * hi_harm + 0.03 * sheen_noise
    # Modal decay and sustain in the mid-range (600-1600 Hz) for persistent transition body
    rest_sustained = _apply_midrange_sustain(rest_raw, sample_rate, 600.0, 1600.0, sustain_gain=0.18, decay_ms=65.0)

    b_notch, a_notch = signal.iirnotch(3400.0, 2.2, fs=sample_rate)
    rest_filtered = signal.lfilter(b_notch, a_notch, rest_sustained)
    sos_lpf_rest = signal.butter(2, 5000.0, btype="lowpass", fs=sample_rate, output="sos")
    rest_filtered = signal.sosfilt(sos_lpf_rest, rest_filtered)
    rest_branch = np.tanh(rest_filtered * 1.02)

    # --- Suma Composite Lineal ---
    composite = rest_branch + sine_branch + intake_branch
    return composite


def _synthesize_exhaust_mic(sample_rate: int) -> tuple[list[float], dict]:
    """V10 exhaust-mic loop (14,400 rpm -> 1200 Hz firing rate).

    Close exhaust microphone character: strong firing fundamental (1200 Hz = rpm/12)
    with the 2nd-6th harmonic 'scream' (2.4-7.2 kHz), per-firing combustion texture
    in the noise floor, exhaust pipe modal resonances (0.85/1.7/3.4 kHz) and
    broadband gas-flow hiss. All components are phase-aligned over a 1 s loop so
    the seam is continuous; `make_loop_seamless` then forces exact endpoint equality.
    """
    seed = 9020
    duration_s = 1.0
    frame_count = round(duration_s * sample_rate)
    t = np.arange(frame_count, dtype=np.float64) / sample_rate
    rng = np.random.default_rng(seed)
    f_firing = 14400.0 / 12.0  # firing frequency at the recorded rpm (V10 4-stroke)

    # 1. Harmonic "scream" ladder: firing fundamental + 2..6 harmonics with slow drift.
    harmonics = [1.0, 0.60, 0.45, 0.30, 0.18, 0.10]
    phase_drift = 0.25 * np.sin(2.0 * np.pi * 2.8 * t)
    scream = np.zeros(frame_count, dtype=np.float64)
    for k, amp in enumerate(harmonics, start=1):
        ph = phase_drift * (0.6 + 0.25 * k)
        scream += amp * np.sin(2.0 * np.pi * f_firing * k * t + ph)
    tremolo = 0.92 + 0.08 * np.sin(2.0 * np.pi * 24.0 * t)  # cylinder-to-cylinder roughness
    scream *= tremolo

    # 2. Combustion roar: broadband noise 250-9000 Hz with firing-rate ripple
    #    (per-firing texture rather than discrete pulses; steady-state loop).
    roar_bp = signal.butter(4, [250.0, 9000.0], btype="bandpass", fs=sample_rate, output="sos")
    roar = signal.sosfilt(roar_bp, rng.standard_normal(frame_count))
    roar /= max(float(np.max(np.abs(roar))), 1e-9)
    firing_ripple = 0.5 + 0.5 * np.sin(2.0 * np.pi * f_firing * t + 0.3)
    roar *= 0.35 + 0.65 * firing_ripple

    # 3. Exhaust pipe modal resonances (0.85 / 1.7 / 3.4 kHz) on broadband flow.
    pipe_raw = rng.standard_normal(frame_count)
    pipe = np.zeros(frame_count, dtype=np.float64)
    for (fc, q) in [(850.0, 4.0), (1700.0, 4.5), (3400.0, 5.0)]:
        bm, am = signal.iirpeak(fc, q, fs=sample_rate)
        pipe += signal.lfilter(bm, am, pipe_raw)
    pipe_hp = signal.butter(2, 300.0, btype="highpass", fs=sample_rate, output="sos")
    pipe = signal.sosfilt(pipe_hp, pipe)
    pipe /= max(float(np.max(np.abs(pipe))), 1e-9)

    # 4. Gas-flow hiss (air through the megaphone, high shelf).
    hiss_bp = signal.butter(2, [2000.0, 12000.0], btype="bandpass", fs=sample_rate, output="sos")
    hiss = signal.sosfilt(hiss_bp, rng.standard_normal(frame_count))
    hiss /= max(float(np.max(np.abs(hiss))), 1e-9)

    # 5. Engine-order body (240/480 Hz at 14,400 rpm) — light weight, exhaust has little sub.
    body = 0.05 * np.sin(2.0 * np.pi * 240.0 * t) + 0.03 * np.sin(2.0 * np.pi * 480.0 * t + 0.7)

    composite = 0.50 * scream + 0.38 * roar + 0.20 * pipe + 0.06 * hiss + body
    saturated = np.tanh(composite * 1.35)

    sos_lp = signal.butter(2, 12000.0, btype="lowpass", fs=sample_rate, output="sos")
    final_sig = signal.sosfilt(sos_lp, saturated)
    final_sig = final_sig - np.mean(final_sig)
    final_list = final_sig.tolist()
    final_list = make_loop_seamless(final_list, xfade_frames=LOOP_XFADE_FRAMES)
    final_list = remove_dc(final_list)
    final_list = normalize_peak(final_list, 0.89)

    return final_list, {
        "recipe": "exhaust_mic_v1",
        "seed": seed,
        "duration_s": duration_s,
        "target_peak": 0.89,
        "components": [
            "v10_firing_fundamental_1200hz_14400rpm",
            "harmonic_scream_ladder_2x6x_2400_7200hz",
            "combustion_roar_250_9000hz_firing_ripple",
            "exhaust_pipe_modal_resonances_850_1700_3400hz",
            "gas_flow_hiss_2k_12khz",
            "engine_order_body_240_480hz",
            "seamless_1s_loop",
            "tanh_warmth",
        ],
    }


def _build_entry(spec: SpecEntry, source_dir: Path, sample_rate: int) -> tuple[list[float], dict]:
    if spec.key in RETIRED_KEYS:
        raise ValueError(f"refusing to build retired key: {spec.key}")
    if spec.synthesis == "exhaust_mic_v1":
        samples, params = _synthesize_exhaust_mic(sample_rate)
        params["category"] = spec.category
        return samples, params
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

    # Apply additive synthesis enhancement to backfire and engine bands
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
    elif spec.key == "engine_low":
        arr = _enhance_engine_low(arr, sample_rate, 9011)
        enhancement_info = ["straight_cut_gear_whine_1.2_3.2khz", "tanh_warmth"]
    elif spec.key == "engine_mid":
        arr = _enhance_engine_mid(arr, sample_rate, 9012)
        enhancement_info = [
            "airbox_throat_growl_750_1800hz",
            "harmonic_overtone_2x_1366hz",
            "midrange_modal_decay_sustain_650_1800hz",
            "smooth_lpf_3600hz",
            "tanh_warmth",
        ]
    elif spec.key == "engine_high":
        arr = _enhance_engine_high(arr, sample_rate, 9013)
        enhancement_info = [
            "hpf_140hz_chest_punch_retained",
            "pure_sine_warm_saturation_465hz_232hz_weight_28pct",
            "airbox_intake_linear_bypass_0pct_saturation",
            "midrange_modal_decay_sustain_600_1600hz",
            "rest_sound_minimal_saturation_1.02_lpf_5000hz",
            "complete_v10_harmonics_930_1395_1860_2325_2790hz",
            "anti_fatigue_notch_3400hz",
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
    return samples, params


def generate_bank(source_dir: Path, output_dir: Path, sample_rate: int = SAMPLE_RATE) -> BankManifest:
    entries: list[FileEntry] = []
    for spec in BANK_SPEC:
        if spec.key in RETIRED_KEYS:
            continue
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
                playback=playback_for_key(spec.key),
                synthesis=params,
                provenance=(
                    "deterministic procedural synthesis"
                    if spec.synthesis is not None
                    else "derived from original samples (source-assets/audio/legacy-f1-1998)"
                ),
                sha256="",
            )
        )

    # Purge any stale retired wavs left over from a previous build.
    for key in RETIRED_KEYS:
        stale = output_dir / f"{key}.wav"
        if stale.is_file():
            stale.unlink()

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


def main(argv: Sequence[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Build a v10_vehicle preview bank")
    ap.add_argument("--repo-root", type=Path, default=Path("."))
    ap.add_argument("--source", type=Path, default=DEFAULT_SOURCE_DIR)
    ap.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    ap.add_argument("--sample-rate", type=int, default=SAMPLE_RATE)
    args = ap.parse_args(argv)
    try:
        root = args.repo_root.resolve(strict=True)
        output = validate_output_path(root, args.output, OutputMode.PREVIEW).path
    except (OSError, OutputPolicyError) as error:
        ap.exit(2, f"audio-bank-preview: {error}\n")
    source = args.source if args.source.is_absolute() else root / args.source
    if not source.is_dir():
        print(f"Source samples dir not found: {source}")
        return 2
    manifest = generate_bank(source, output, args.sample_rate)
    print(f"Generated preview {manifest.bank_name} -> {output} ({len(manifest.files)} files)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
