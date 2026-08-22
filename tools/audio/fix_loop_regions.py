#!/usr/bin/env python3
"""Fase 2 — fix loop regions for continuous engine/tyre/wind voices.

Each continuous bed is a steady engine/tyre loop whose waveform repeats at
the firing period P. A seamless loop requires loop_end - loop_begin == k*P
(exact phase), otherwise the loop jump clicks. This tool:
1. decodes each bed (WAV or OGG, via soundfile),
2. estimates P from the normalized autocorrelation peak,
3. searches the best aligned region [begin, begin + k*P],
4. verifies the resulting seam (>= -60 dB goal),
5. writes game/sounds/runtime/loop_regions.json (id -> region + mode) and
   re-exports OGG beds as PCM WAV (v10_v2/*.loop.wav) so Godot can apply the
   region (AudioStreamOggVorbis cannot hold regions; AudioStreamWAV can).
"""
import json
import wave
from pathlib import Path

import numpy as np
import soundfile as sf

ROOT = Path(__file__).resolve().parents[2]
V10 = ROOT / "game" / "sounds" / "runtime" / "v10_v2"
OUT_JSON = ROOT / "game" / "sounds" / "runtime" / "loop_regions.json"
SEAM_EVAL = ROOT / "diagnostics" / "loop_seam_after_fix.json"

# Looped beds (continuous voices). `ogg_as_wav`: re-export when the source is OGG.
BEDS = [
    ("idle", "r25 int idle.ogg"),
    ("off_low", "r25 int off low.ogg"),
    ("off_mid", "r25 int off mid.wav"),
    ("off_midhigh", "r25 int off midhigh.ogg"),
    ("off_downshift", "r25 int off downshift.wav"),
    ("on_mid", "r25 int on mid.wav"),
    ("on_midhigh", "r25 int on midhigh.wav"),
    ("on_high", "r25 int on high.wav"),
    ("on_upshift", "r25 int on upshift.wav"),
    ("ext_idle", "r25 ext idle.ogg"),
    ("ext_on_mid", "r25 ext on mid rear close.wav"),
    ("ext_on_midhigh", "r25 ext on midhihg rear close.wav"),
    ("ext_on_high", "r25 ext on high rear close 1.wav"),
    ("ext_off_mid", "r25 ext off mid rear close.wav"),
    ("ext_off_midhigh", "r25 ext off midhigh rear close.wav"),
    ("tyre_rolling", "tyre_rolling.wav"),
    ("skid", "Skid.ogg"),
    ("wind", "single seater highspeed wind.ogg"),
    ("brakes", "lb brakes.ogg"),
]

FMIN, FMAX = 25.0, 1500.0  # firing-period search range (Hz)


def estimate_period(x: float, rate: int) -> int:
    y = x - x.mean()
    n = len(y)
    # Work on a 2..3 s median-energy chunk: plenty of firing cycles, fast FFT.
    chunk_len = min(n, rate * 3)
    start = max(0, n // 3)
    seg = y[start : start + chunk_len]
    if len(seg) % 2:
        seg = seg[:-1]
    if len(seg) < rate:
        seg = y[: min(n, rate * 3)]
        if len(seg) % 2:
            seg = seg[:-1]
    spec = np.fft.rfft(seg)
    corr = np.fft.irfft(spec * np.conj(spec)).real
    corr /= corr[0] + 1e-12
    lo = max(2, int(rate / FMAX))
    hi = max(lo + 2, int(rate / FMIN))
    k = int(np.argmax(corr[lo:hi])) + lo
    # parabolic refinement
    if 0 < k < len(corr) - 1:
        a, b, c = corr[k - 1], corr[k], corr[k + 1]
        d = 0.5 * (a - c) / (a - 2 * b + c) if (a - 2 * b + c) != 0 else 0.0
        k = k + d
    return int(round(k))


def seam_db(x: np.ndarray, edge_a: int, edge_b: int, width: int = 4096) -> float:
    a0, a1 = max(0, edge_a - width), max(0, edge_a)
    b0, b1 = min(len(x), edge_b), min(len(x), edge_b + width)
    if a1 - a0 < 512 or b1 - b0 < 512:
        return np.nan
    seg_a = x[a0:a1]
    seg_b = x[b0:b1]
    n = min(len(seg_a), len(seg_b))
    diff = np.sqrt(np.mean((seg_a[-n:] - seg_b[:n]) ** 2))
    rms = np.sqrt(np.mean(x**2)) + 1e-12
    return float(20.0 * np.log10(diff / rms + 1e-12))


def decode(path: Path) -> tuple[np.ndarray, int]:
    if path.suffix == ".wav":
        with wave.open(str(path), "rb") as w:
            n, rate = w.getnframes(), w.getframerate()
            raw = np.frombuffer(w.readframes(n), dtype=np.int16)
            if w.getnchannels() > 1:
                raw = raw.reshape(-1, w.getnchannels()).mean(axis=1)
            return raw.astype(np.float64) / 32768.0, rate
    data, rate = sf.read(str(path), dtype="float64", always_2d=True)
    mono = data.mean(axis=1)
    return mono, rate


def export_wav(source: Path, dest: Path, rate: int, channels: int) -> None:
    data, r = sf.read(str(source), dtype="int16", always_2d=True)
    assert r == rate
    if data.shape[1] < channels:
        data = np.repeat(data, channels, axis=1)
    with wave.open(str(dest), "wb") as w:
        w.setnchannels(channels)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(data[:, :channels].astype("<i2").tobytes())


def main() -> int:
    results = {}
    eval_rows = []
    for vid, name in BEDS:
        src = V10 / name
        if not src.exists():
            print(f"  {vid:14s} MISSING {name}")
            continue
        x, rate = decode(src)
        if len(x) < rate:
            print(f"  {vid:14s} too short")
            continue
        P = estimate_period(x, rate)
        # begin: first cycle aligned after a short warm-up (seek ~ 30 ms)
        head = max(1, int(0.030 * rate))
        begin = head
        k = max(1, (len(x) - begin - P) // P)
        end = begin + k * P
        best = None
        for cand_begin in range(begin, begin + P, max(1, P // 8)):
            cend = cand_begin + ((len(x) - cand_begin - P) // P) * P
            if cend - cand_begin < P:
                continue
            s = seam_db(x, cend, cand_begin)
            if best is None or s < best[0]:
                best = (s, cand_begin, cend, (cend - cand_begin) // P)
        s, b, e, cycles = best
        whole = seam_db(x, len(x), 0)
        # Short-region scan: drift grows with region length; short k*P regions
        # (4..40 cycles) minimize the wrap seam to cycle-to-cycle variation.
        short = None
        for kc in range(4, min(41, max(5, cycles // 2)) + 1):
            ce = b + kc * P
            if ce > len(x):
                break
            ss = seam_db(x, ce, b)
            if short is None or ss < short[0]:
                short = (ss, kc, ce)
        # Re-export OGG beds as PCM WAV so Godot can apply the region.
        wav_name = None
        if src.suffix == ".ogg":
            wav_name = name.replace(".ogg", ".loop.wav")
            export_wav(src, V10 / wav_name, rate, 2)
        short_policy = (
            {
                "cycles": short[1],
                "loop_begin_s": round(b / rate, 6),
                "loop_end_s": round(short[2] / rate, 6),
                "seam_db": round(short[0], 1),
            }
            if short is not None and short[0] < best[0]
            else None
        )
        results[vid] = {
            "file": name,
            "wav_export": wav_name,
            "rate": rate,
            "period_samples": P,
            "loop_begin_s": round(b / rate, 6),
            "loop_end_s": round(e / rate, 6),
            "cycles": cycles,
            "seam_db": round(s, 1),
            "whole_seam_db": round(whole, 1) if np.isfinite(whole) else None,
            "mode": "region",
            "short_loop": short_policy,
        }
        eval_rows.append(results[vid] | {"id": vid})
        short_txt = f" short={short[1]}cyc[{round(b/rate,3)}..{round(short[2]/rate,3)}] {round(short[0],1)}dB" if short_policy else ""
        print(
            f"  {vid:14s} P={P:5d} reg=[{b/rate:7.3f}..{e/rate:7.3f}] "
            f"({cycles:4d} cyc) seam={s:6.1f} dB (whole {whole:5.1f} dB){short_txt}"
        )
    OUT_JSON.write_text(json.dumps(results, indent=2), encoding="utf-8")
    SEAM_EVAL.write_text(json.dumps(eval_rows, indent=2), encoding="utf-8")
    ok = sum(1 for r in eval_rows if r["seam_db"] <= -30.0)
    short_ok = sum(1 for r in eval_rows if r.get("short_loop") and r["short_loop"]["seam_db"] <= -30.0)
    print(f"wrote {OUT_JSON}; long-region seams <= -30 dB: {ok}/{len(eval_rows)}; short-region candidates: {short_ok}/{len(eval_rows)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())