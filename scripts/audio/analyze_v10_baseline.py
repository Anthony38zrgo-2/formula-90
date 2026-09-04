# Groups the PHY-001 baseline by RPM and runs the reference analyzer on each
# group with --forced-rpm, since every render is a known steady RPM.
#
# The reference analyzer keys reference_metrics.json by filename, and each
# render emits out.wav. So we stage uniquely-named copies (<combo>.wav) into
# each RPM analysis directory before analysis.
#
# Usage:
#   python scripts/audio/analyze_v10_baseline.py
#
# Output:
#   reports/audio/physical-refactor/baseline/analysis/r_<rpm>/
#     {<combo>.wav staged copies, reference_metrics.json, order_spectra.csv,
#      reference_overview.png, reference_order_spectra.png}

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path("D:/Formula90s")
BASELINE = REPO_ROOT / "reports/audio/physical-refactor/baseline"
ANALYSIS = BASELINE / "analysis"
REF_ANALYZER = REPO_ROOT / "scripts/audio/analyze_v10_references.py"

RPM = [3000, 5000, 8000, 11000, 14000, 15000]
THROTTLE = ["0.25", "0.70", "1.00"]
LOAD = ["0.00", "0.50", "1.00"]


def combo_name(rpm: int, throttle: str, load: str) -> str:
    return f"r{rpm}_t{throttle}_l{load}"


def stage_name(rpm: int, throttle: str, load: str) -> str:
    return f"r{rpm}_t{throttle.replace('.', '')}_l{load.replace('.', '')}.wav"


def main() -> int:
    ANALYSIS.mkdir(parents=True, exist_ok=True)
    total = len(RPM) * len(THROTTLE) * len(LOAD)
    done = 0

    for rpm in RPM:
        rpm_dir = ANALYSIS / f"r_{rpm}"
        rpm_dir.mkdir(parents=True, exist_ok=True)
        staged: list[Path] = []
        for throttle in THROTTLE:
            for load in LOAD:
                done += 1
                combo = combo_name(rpm, throttle, load)
                src = BASELINE / combo / "out.wav"
                if not src.exists():
                    print(f"[{done}/{total}] MISSING {src}", file=sys.stderr)
                    return 2
                dst = rpm_dir / stage_name(rpm, throttle, load)
                shutil.copyfile(src, dst)
                staged.append(dst)
                print(f"[{done}/{total}] staged {combo}")

        cmd = [
            sys.executable,
            str(REF_ANALYZER),
            "--forced-rpm",
            str(rpm),
            "--output-dir",
            str(rpm_dir),
            *(str(p) for p in staged),
        ]
        print(f"analyzing rpm={rpm} ({len(staged)} files)")
        result = subprocess.run(cmd, cwd=str(REPO_ROOT))
        if result.returncode != 0:
            print(f"analysis failed for rpm={rpm}", file=sys.stderr)
            return result.returncode

    print(f"Analysis complete. See {ANALYSIS}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
