#!/usr/bin/env python3
"""Loop-seam audit for the v10_v2 runtime assets (Fase 2).

For each WAV/OGG-backed voice of the engine families, quantifies the loop
discontinuity that Godot would reproduce:
- whole-file loop: seam = RMS(first N frames - last N frames) vs RMS(signal)
- region loop (WAVs with explicit loop_begin/loop_end): seam at the region
  boundary (loop_end wraps to loop_begin).

Emits diagnostics/loop_seam_audit.json. Thresholds (Fase 2 policy):
  seam_db <= -60 : seamless
  <= -40         : inaudible click
  >  -40         : candidate for region/retrigger/crossfade decision
"""
import json
import sys
import wave
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
V10 = ROOT / "game" / "sounds" / "runtime" / "v10_v2"
OUT = ROOT / "diagnostics" / "loop_seam_audit_final.json"

# Final regions as applied in commands.rs (best measured k*period alignment);
# whole-file mode for everything else.
FINAL_REGIONS = {
    "engine.on_mid": (0.033628, 1.639977),
    "engine.on_midhigh": (0.098141, 2.551587),
    "engine.on_high": (0.035034, 4.927007),
    "engine.off_mid": (0.031837, 2.588571),
}

VOICES = [
    ("engine.idle", "r25 int idle.ogg", "whole", None, None),
    ("engine.off_low", "r25 int off low.ogg", "whole", None, None),
    ("engine.off_mid", "r25 int off mid.wav", "region", None, None),
    ("engine.off_midhigh", "r25 int off midhigh.ogg", "whole", None, None),
    ("engine.off_downshift", "r25 int off downshift.wav", "whole", None, None),
    ("engine.on_mid", "r25 int on mid.wav", "region", None, None),
    ("engine.on_midhigh", "r25 int on midhigh.wav", "region", None, None),
    ("engine.on_high", "r25 int on high.wav", "region", None, None),
    ("engine.on_upshift", "r25 int on upshift.wav", "whole", None, None),
    ("ext.idle", "r25 ext idle.ogg", "whole", None, None),
    ("ext.on_mid", "r25 ext on mid rear close.wav", "whole", None, None),
    ("ext.on_midhigh", "r25 ext on midhihg rear close.wav", "whole", None, None),
    ("ext.on_high", "r25 ext on high rear close 1.wav", "whole", None, None),
    ("ext.off_mid", "r25 ext off mid rear close.wav", "whole", None, None),
    ("ext.off_midhigh", "r25 ext off midhigh rear close.wav", "whole", None, None),
    ("wheel.rolling", "tyre_rolling.wav", "whole", None, None),
    ("wheel.brakes", "lb brakes.ogg", "whole", None, None),
    ("wheel.skid", "Skid.ogg", "whole", None, None),
    ("wind", "single seater highspeed wind.ogg", "whole", None, None),
]


def read_wav(path: Path) -> tuple[np.ndarray, int]:
    with wave.open(str(path), "rb") as w:
        n = w.getnframes()
        rate = w.getframerate()
        raw = np.frombuffer(w.readframes(n), dtype=np.int16)
        if w.getnchannels() > 1:
            raw = raw.reshape(-1, w.getnchannels()).mean(axis=1)
        return raw.astype(np.float64) / 32768.0, rate


def seam_db(x: np.ndarray, edge_a: int, edge_b: int, width: int = 2048) -> float:
    """Discontinuity between the samples just before edge_a and just after edge_b."""
    a0 = max(0, edge_a - width)
    a1 = max(0, edge_a)
    b0 = min(len(x), edge_b)
    b1 = min(len(x), edge_b + width)
    if a1 - a0 < 256 or b1 - b0 < 256:
        return np.nan
    seg_a = x[a0:a1]
    seg_b = x[b0:b1]
    n = min(len(seg_a), len(seg_b))
    diff = np.sqrt(np.mean((seg_a[-n:] - seg_b[:n]) ** 2))
    rms = np.sqrt(np.mean(x**2)) + 1e-12
    return 20.0 * np.log10(diff / rms + 1e-12)


def main() -> int:
    results = []
    for vid, name, mode, beg, end in VOICES:
        path = V10 / name
        if not path.exists():
            results.append({"id": vid, "file": name, "error": "missing"})
            continue
        if path.suffix != ".wav":
            results.append(
                {
                    "id": vid,
                    "file": name,
                    "mode": mode,
                    "note": "ogg: decode pendiente (harness Godot, fase 2b)",
                    "seam_db": None,
                    "whole_seam_db": None,
                }
            )
            continue
        x, rate = read_wav(path)
        dur_s = len(x) / rate
        whole = seam_db(x, len(x), 0)
        region_seam = None
        if mode == "region":
            beg, end = FINAL_REGIONS.get(vid, (None, None))
            if beg is not None and end is not None:
                ib = int(beg * rate)
                ie = int(end * rate)
                if 0 < ib < ie < len(x):
                    region_seam = seam_db(x, ie, ib)
        results.append(
            {
                "id": vid,
                "file": name,
                "mode": mode,
                "duration_s": round(dur_s, 3),
                "rate": rate,
                "whole_seam_db": round(float(whole), 1) if np.isfinite(whole) else None,
                "region_seam_db": round(float(region_seam), 1) if region_seam is not None else None,
                "region_begin_s": beg,
                "region_end_s": end,
            }
        )
    OUT.write_text(json.dumps(results, indent=2), encoding="utf-8")
    for r in results:
        if "error" in r:
            print(f"  {r['id']:22s} {r['file']}: {r['error']}")
            continue
        tag = r.get("region_seam_db") if r.get("mode") == "region" else r.get("whole_seam_db")
        verdict = "OK" if tag is not None and tag <= -40 else ("CLICK" if tag is not None else "?")
        print(
            f"  {r['id']:22s} {r['file']:34s} dur={r.get('duration_s')} "
            f"whole={r.get('whole_seam_db')} region={r.get('region_seam_db')} -> {verdict}"
        )
    print(f"wrote {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())