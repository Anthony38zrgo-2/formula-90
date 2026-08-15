"""Generate the golden centerline fixture for the Rust parity test.

Uses the exact production Python code paths (svg_sanitizer, svg_normalizer
flatten_path / resample_closed) so the Rust parity test compares against the
real normalizer contract rather than a second Python implementation.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

_REPO = Path(__file__).resolve().parents[5]
_PIPELINE = _REPO / "blender" / "track_pipeline"
sys.path.insert(0, str(_PIPELINE))

from svg_sanitizer import sanitize_svg  # noqa: E402
from svg_normalizer import flatten_path, resample_closed  # noqa: E402

SAMPLE_SPACING_M = 2.0


def _polyline_length(points: list[tuple[float, float]]) -> float:
    return sum(
        math.hypot(points[i + 1][0] - points[i][0], points[i + 1][1] - points[i][1])
        for i in range(len(points) - 1)
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Generate the Rust parity golden fixture.")
    parser.add_argument("--source", required=True, help="Track SVG source (production fixture).")
    parser.add_argument("--output", required=True, help="Golden JSON output path.")
    args = parser.parse_args(argv)

    source = Path(args.source)
    if not source.is_file():
        print(f"FAIL source not found: {source}", file=sys.stderr)
        return 2

    source_bytes = source.read_bytes()
    canonical = sanitize_svg(source_bytes)
    centerline = None
    for element in canonical.iter():
        if element.get("data-role") == "centerline":
            centerline = element
            break
    if centerline is None:
        print("FAIL centerline not found", file=sys.stderr)
        return 2

    subpaths = flatten_path(centerline.get("d", ""))
    if len(subpaths) != 1:
        print("FAIL centerline must be a single subpath", file=sys.stderr)
        return 2

    raw = subpaths[0]
    resampled = resample_closed(raw, SAMPLE_SPACING_M)
    rounded = [[round(x, 6), round(z, 6)] for x, z in resampled]

    golden = {
        "schema_version": 1,
        "source": str(source),
        "source_sha256": hashlib.sha256(source_bytes).hexdigest(),
        "sample_spacing_m": SAMPLE_SPACING_M,
        "centerline_d": centerline.get("d", ""),
        "raw_polyline": raw,
        "resampled_polyline": rounded,
        "raw_length_m": _polyline_length(raw),
        "resampled_length_m": _polyline_length(resampled + [resampled[0]]),
        "resampled_count": len(resampled),
    }

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(golden, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    print(f"golden: {output.resolve()}")
    print(f"source_sha256: {golden['source_sha256']}")
    print(f"raw_length_m: {golden['raw_length_m']:.6f}")
    print(f"resampled_length_m: {golden['resampled_length_m']:.6f}")
    print(f"resampled_count: {golden['resampled_count']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
