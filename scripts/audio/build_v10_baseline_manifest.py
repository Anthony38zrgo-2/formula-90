# Builds reports/audio/physical-refactor/baseline_manifest.json from the 54
# PHY-001 baseline renders, merging each combo's out.metadata.json with the
# per-file metrics produced by analyze_v10_references.py (grouped by RPM).
#
# Usage:
#   python scripts/audio/build_v10_baseline_manifest.py
#
# Output:
#   reports/audio/physical-refactor/baseline_manifest.json

from __future__ import annotations

import json
from pathlib import Path

REPO_ROOT = Path("D:/Formula90s")
BASELINE = REPO_ROOT / "reports/audio/physical-refactor/baseline"
ANALYSIS = BASELINE / "analysis"
MANIFEST = REPO_ROOT / "reports/audio/physical-refactor/baseline_manifest.json"

RPM = [3000, 5000, 8000, 11000, 14000, 15000]
THROTTLE = ["0.25", "0.70", "1.00"]
LOAD = ["0.00", "0.50", "1.00"]


def combo_name(rpm: int, throttle: str, load: str) -> str:
    return f"r{rpm}_t{throttle}_l{load}"


def staged_key(rpm: int, throttle: str, load: str) -> str:
    return f"r{rpm}_t{throttle.replace('.', '')}_l{load.replace('.', '')}.wav"


METRIC_FIELDS = [
    "sample_rate",
    "channels",
    "frames",
    "duration_s",
    "dtype",
    "peak",
    "rms",
    "rms_dbfs",
    "crest_db",
    "dc",
    "clipping_fraction",
    "shaft_hz",
    "implied_rpm",
    "firing_hz",
    "firing_order",
    "firing_energy_fraction",
    "harmonic_energy_fraction",
    "spectral_centroid_hz",
    "spectral_rolloff_85_hz",
    "spectral_flatness",
    "band_20_80",
    "band_80_250",
    "band_250_600",
    "band_600_2500",
    "band_2500_7000",
    "band_7000_16000",
    "shaft_cycle_corr",
    "engine_cycle_720_corr",
    "adjacent_engine_cycle_corr_mean",
    "adjacent_engine_cycle_corr_std",
    "envelope_modulation_hz",
    "envelope_modulation_depth",
    "seam_value_jump",
    "seam_slope_jump",
    "best_seam_correlation",
    "best_seam_lag_samples",
]


def load_reference_metrics(rpm: int) -> dict[str, dict]:
    path = ANALYSIS / f"r_{rpm}" / "reference_metrics.json"
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    records: list[dict] = []
    for rpm in RPM:
        metrics_by_key = load_reference_metrics(rpm)
        for throttle in THROTTLE:
            for load in LOAD:
                combo = combo_name(rpm, throttle, load)
                meta_path = BASELINE / combo / "out.metadata.json"
                if not meta_path.exists():
                    print(f"MISSING metadata {meta_path}")
                    return 2
                meta = json.loads(meta_path.read_text(encoding="utf-8"))

                rec: dict[str, object] = {
                    "combo": combo,
                    "rpm": rpm,
                    "throttle": float(throttle),
                    "load": float(load),
                    "render": meta,
                    "metrics": {},
                }

                key = staged_key(rpm, throttle, load)
                metrics = metrics_by_key.get(key)
                if metrics is None:
                    print(f"MISSING metrics for {key} (rpm={rpm})")
                    return 2
                rec["metrics"] = {f: metrics.get(f) for f in METRIC_FIELDS}

                lim = meta.get("worst_limiter_reduction_db", 0.0)
                rec["limiter_engaged"] = bool(lim < -0.01)
                rec["limiter_reduction_db"] = float(lim)

                records.append(rec)

    payload = {
        "name": "v10-engine-synth PHY-001 frozen baseline",
        "architecture": "rust-greenfield-v10",
        "git_head": records[0]["render"]["git_head"] if records else None,
        "source_dirty": records[0]["render"]["source_dirty"] if records else None,
        "sample_rate": 48000,
        "seed": records[0]["render"]["seed"] if records else None,
        "firing_order": [0, 5, 1, 6, 2, 7, 3, 8, 4, 9],
        "grid": {
            "rpm": RPM,
            "throttle": [float(t) for t in THROTTLE],
            "load": [float(l) for l in LOAD],
            "count": len(records),
        },
        "render_flags": {
            "seconds": 6,
            "warmup": 1,
            "accel_seconds": 5.0,
            "acoustic_scene": True,
            "sample_layer_dir": "game/audio/v10_gf509",
            "hybrid_headroom_gain": 0.610000,
            "sample_mid_architecture": records[0]["render"]["sample_mid_architecture"]
            if records
            else None,
        },
        "limiter_engaged_count": sum(1 for r in records if r["limiter_engaged"]),
        "records": records,
    }

    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    print(f"Wrote {MANIFEST}")
    print(f"records={len(records)} limiter_engaged={payload['limiter_engaged_count']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
