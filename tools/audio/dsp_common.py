"""Shared DSP primitives — single responsibility: stateless signal utilities.

No synthesis decisions here; only deterministic, testable transforms.
All functions are pure and operate on list[float] in [-1, 1] (or float ndarray).
"""

from __future__ import annotations

import math
import struct
import wave
from pathlib import Path

import numpy as np
from scipy.signal import resample_poly

SAMPLE_RATE = 44100
PCM_BITS = 16
CHANNELS = 1


def read_wav_mono(path: Path) -> tuple[int, np.ndarray]:
    """Read any WAV (8/16-bit, any channels) as a mono float64 array in [-1, 1].

    Returns (source_rate, samples). Non-16-bit or multi-channel sources are
    normalized/downmixed deterministically so any original sample can be used.
    """
    with wave.open(str(path), "rb") as r:
        ch = r.getnchannels()
        sw = r.getsampwidth()
        rate = r.getframerate()
        raw = r.readframes(r.getnframes())
    if sw == 1:
        arr = np.frombuffer(raw, dtype=np.uint8).astype(np.float64) - 128.0
        arr = arr / 128.0
    else:
        arr = np.frombuffer(raw, dtype=np.int16).astype(np.float64) / 32768.0
    if ch > 1:
        arr = arr.reshape(-1, ch).mean(axis=1)
    return rate, arr


def resample_mono(arr: np.ndarray, from_rate: int, to_rate: int = SAMPLE_RATE) -> np.ndarray:
    """Deterministic anti-aliased resample of a mono float array to `to_rate`."""
    if from_rate == to_rate or arr.size == 0:
        return arr
    return resample_poly(arr, to_rate, from_rate)


def remove_dc(samples: list[float]) -> list[float]:
    if not samples:
        return samples
    mean = sum(samples) / len(samples)
    return [v - mean for v in samples]


def one_pole_lowpass(samples: list[float], cutoff_hz: float, sample_rate: int = SAMPLE_RATE) -> list[float]:
    """One-pole IIR lowpass. Deterministic, no external deps."""
    if cutoff_hz <= 0 or cutoff_hz >= sample_rate * 0.5:
        return list(samples)
    alpha = 1.0 - math.exp(-2.0 * math.pi * cutoff_hz / sample_rate)
    out: list[float] = []
    state = 0.0
    for v in samples:
        state += alpha * (v - state)
        out.append(state)
    return out


def one_pole_highpass(samples: list[float], cutoff_hz: float, sample_rate: int = SAMPLE_RATE) -> list[float]:
    """One-pole highpass via lowpass subtraction."""
    if cutoff_hz <= 0:
        return list(samples)
    low = one_pole_lowpass(samples, cutoff_hz, sample_rate)
    return [s - lo for s, lo in zip(samples, low)]


def normalize_peak(samples: list[float], peak: float = 0.89) -> list[float]:
    if not samples:
        return samples
    mx = max(abs(v) for v in samples)
    if mx < 1e-12:
        return list(samples)
    scale = peak / mx
    return [max(-0.999, min(0.999, v * scale)) for v in samples]


def fade_edges(samples: list[float], fade_frames: int) -> None:
    """In-place linear fade in/out for click-free edges."""
    n = min(fade_frames, len(samples) // 2)
    if n <= 1:
        return
    for i in range(n):
        g = i / max(1, n - 1)
        samples[i] *= g
        samples[-1 - i] *= g


def make_loop_seamless(samples: list[float], xfade_frames: int = 2048) -> list[float]:
    """Make a loop seamless so that `sample[0] == sample[-1]` (periodic endpoint).

    An equal-power crossfade folds the tail region into the head, then the tail
    is tapered so the very last sample equals the first. This guarantees a
    discontinuity near zero at the wrap point, which a hard `i % len` loop will
    not click on. Length is preserved.
    """
    n = len(samples)
    if n < xfade_frames * 2 or xfade_frames <= 0:
        return list(samples)
    buf = list(samples)
    for i in range(xfade_frames):
        t = i / max(1, xfade_frames - 1)
        g_head = math.cos(t * math.pi * 0.5)
        g_tail = math.cos((1 - t) * math.pi * 0.5)
        buf[i] = samples[i] * g_head + samples[n - xfade_frames + i] * g_tail
    first = buf[0]
    # Taper the tail region so its end converges exactly to `first`, making the
    # wrap point continuous (sample[n-1] == sample[0]).
    for i in range(xfade_frames):
        t = i / max(1, xfade_frames - 1)
        idx = n - xfade_frames + i
        buf[idx] = buf[idx] * (1.0 - t) + first * t
    # Force exact endpoint equality to eliminate float drift at the wrap.
    buf[-1] = buf[0]
    return buf


def soft_clip(samples: list[float], drive: float = 1.0) -> list[float]:
    if drive == 1.0:
        return [math.tanh(v) for v in samples]
    return [math.tanh(v * drive) for v in samples]


def rms(samples: list[float]) -> float:
    if not samples:
        return 0.0
    return math.sqrt(sum(v * v for v in samples) / len(samples))


def peak_abs(samples: list[float]) -> float:
    if not samples:
        return 0.0
    return max(abs(v) for v in samples)


def dc_offset(samples: list[float]) -> float:
    if not samples:
        return 0.0
    return sum(samples) / len(samples)


def loop_discontinuity(samples: list[float]) -> float:
    """The loop-click metric: the exact step at the wrap point, |sample[-1] - sample[0]|.

    When a loop plays from its end back to its start, the only artificial
    transition is between `sample[-1]` and `sample[0]`. A seamless loop has this
    ~0. Both the periodic engine synthesis and `make_loop_seamless` force it to 0.
    """
    if not samples:
        return 0.0
    return abs(samples[-1] - samples[0])


def write_wav_mono16(path: Path, samples: list[float], sample_rate: int = SAMPLE_RATE) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = b"".join(struct.pack("<h", round(max(-1.0, min(1.0, v)) * 32767)) for v in samples)
    with wave.open(str(path), "wb") as w:
        w.setnchannels(CHANNELS)
        w.setsampwidth(PCM_BITS // 8)
        w.setframerate(sample_rate)
        w.writeframes(payload)


def read_wav_mono16(path: Path) -> tuple[wave._wave_params, list[float]]:
    with wave.open(str(path), "rb") as r:
        params = r.getparams()
        raw = r.readframes(r.getnframes())
    n = len(raw) // 2
    vals = [v[0] / 32767.0 for v in struct.iter_unpack("<h", raw[: n * 2])]
    return params, vals


def lufs_approx(samples: list[float]) -> float:
    r = rms(samples)
    if r < 1e-9:
        return -90.0
    return 20.0 * math.log10(r)
