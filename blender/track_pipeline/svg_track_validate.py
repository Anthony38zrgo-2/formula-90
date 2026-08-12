"""Vertical-slice F90 Track SVG validation CLI.

Runs sanitize + normalize twice and asserts byte-identical normalized JSON,
then reports collision invariants derived from the existing terrain grid
without invoking Blender. Exits non-zero on any failure.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import hashlib
import sys

from svg_sanitizer import SVGSanitizeError, sanitize_svg
from svg_normalizer import NormalizeError, normalized_json_bytes, normalize


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate an F90 Track SVG profile and normalize it deterministically.")
    parser.add_argument("--input", required=True, help="Path to an SVG track source.")
    args = parser.parse_args()

    source = Path(args.input)
    if not source.exists():
        print(f"FAIL input not found: {source}", file=sys.stderr)
        return 2

    try:
        canonical = sanitize_svg(source.read_bytes())
    except SVGSanitizeError as exc:
        print("FAIL sanitization:\n- " + "\n- ".join(exc.diagnostics), file=sys.stderr)
        return 2

    try:
        first = normalized_json_bytes(canonical)
        second = normalized_json_bytes(canonical)
    except NormalizeError as exc:
        print(f"FAIL normalization: {exc}", file=sys.stderr)
        return 2

    hash_first = hashlib.sha256(first).hexdigest()
    hash_second = hashlib.sha256(second).hexdigest()
    deterministic = hash_first == hash_second

    result = normalize(canonical)
    checks = result["collision_validation"]
    invariants = {
        "finite_vertices": bool(checks["finite_vertices"]),
        "nondegenerate_triangles": bool(checks["nondegenerate_triangles"]),
        "blender_winding_upward": bool(checks["blender_winding_upward"]),
        "continuous_collision_grid": bool(checks["continuous_collision_grid"]),
        "safety_floor_valid": bool(checks["safety_floor_valid"]),
        "seam_continuous": float(checks["max_collision_seam_error_m"]) <= 1e-6,
    }

    print(f"track: {result['track_id']}")
    print(f"centerline length: {result['centerline']['length_m']:.3f} m")
    print(f"centerline points: {result['centerline']['point_count']}")
    print(f"banking: {len(result['banking'])} terrain: {len(result['terrain_zones'])} "
          f"barriers: {len(result['barriers'])} assets: {len(result['assets'])}")
    print(f"run1 hash: {hash_first}")
    print(f"run2 hash: {hash_second}")
    print(f"BYTE-IDENTICAL: {deterministic}")
    for name, ok in invariants.items():
        print(f" {'PASS' if ok else 'FAIL'} {name}")

    ok = deterministic and all(invariants.values())
    return 0 if ok else 2


if __name__ == "__main__":
    raise SystemExit(main())
