#!/usr/bin/env python3
"""Measure the dominant fundamental (f0) of a WAV segment via autocorrelation.

Used to pin the FMOD autopitch model for the R25 engine layers:
at the RPM where total pitch == 0 semitones, the sample's f0 should equal
the V10 firing order (5th order: f = rpm/60*5 Hz) at that RPM.
"""
import sys
import wave
import numpy as np


def read_wav(path: str) -> tuple[np.ndarray, int]:
    with wave.open(path, "rb") as w:
        n = w.getnframes()
        rate = w.getframerate()
        raw = np.frombuffer(w.readframes(n), dtype=np.int16)
        ch = w.getnchannels()
        if ch > 1:
            raw = raw.reshape(-1, ch).mean(axis=1)
        return raw.astype(np.float64) / 32768.0, rate


def f0_autocorr(x: np.ndarray, rate: int, fmin: float = 40.0, fmax: float = 2000.0) -> float:
    lo = max(2, int(rate / fmax))
    hi = max(lo + 1, int(rate / fmin))
    corr = np.correlate(x, x, "full")[len(x) - 1:]
    corr /= corr[0] + 1e-12
    seg = corr[lo:hi]
    # parabolic refinement around the peak
    k = int(np.argmax(seg)) + lo
    if k <= 0 or k >= len(corr) - 1:
        return 0.0
    a, b, c = corr[k - 1], corr[k], corr[k + 1]
    denom = a - 2 * b + c
    d = 0.0 if denom == 0 else 0.5 * (a - c) / denom
    return rate / (k + d)


def main() -> None:
    for path in sys.argv[1:]:
        x, rate = read_wav(path)
        n = len(x)
        step = max(1, int(rate * 0.5))
        wins = []
        for start in range(n // 8, n, step):
            seg = x[start:start + step]
            if seg.size < rate // 4:
                break
            if np.max(np.abs(seg)) < 0.02:
                continue
            f = f0_autocorr(seg, rate)
            if 60 < f < 1800:
                wins.append(f)
        if not wins:
            print(f"{path}: no stable f0")
            continue
        wins = np.array(wins)
        # median of the most common octave cluster
        hist, edges = np.histogram(np.log2(wins), bins=24)
        bin_max = int(np.argmax(hist))
        cluster = wins[(np.log2(wins) >= edges[bin_max]) & (np.log2(wins) < edges[bin_max + 1])]
        print(f"{path}: f0_median={np.median(wins):.1f} Hz  cluster={np.median(cluster):.1f} Hz  n={len(wins)}")


if __name__ == "__main__":
    main()