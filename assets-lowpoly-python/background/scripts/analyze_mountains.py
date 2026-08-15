#!/usr/bin/env python3
"""Analyze mountain sprites for 3D procedural generation.

Extracts:
- Heightmap from alpha channel (per-column height)
- SNES color palette (unique colors)
- Per-column dominant color
- Slope analysis for waterfall placement candidates
- Silhouette metrics (height %, peaks, valleys)

Usage:
    python analyze_mountains.py <sprite_path> [--output <json_path>]
"""

import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image


def extract_heightmap(img_array: np.ndarray) -> np.ndarray:
    """Extract per-column heightmap from RGBA sprite.

    Returns array of shape (width,) with values in [0, 1].
    0 = base (bottom), 1 = top (highest opaque pixel).
    """
    alpha = img_array[:, :, 3]
    height, width = alpha.shape

    heightmap = np.zeros(width, dtype=np.float64)
    for x in range(width):
        col = alpha[:, x]
        opaque_rows = np.where(col > 128)[0]
        if len(opaque_rows) > 0:
            top_row = opaque_rows[0]  # smallest Y = highest pixel
            heightmap[x] = 1.0 - (top_row / height)
        else:
            heightmap[x] = 0.0

    return heightmap


def extract_palette(img_array: np.ndarray) -> np.ndarray:
    """Extract unique SNES palette colors from opaque pixels.

    Returns array of shape (N, 3) with RGB values.
    """
    pixels = img_array.reshape(-1, 4)
    opaque = pixels[pixels[:, 3] > 128][:, :3]
    unique = np.unique(opaque, axis=0)
    return unique


def extract_per_column_color(img_array: np.ndarray, palette: np.ndarray) -> np.ndarray:
    """Map each column to its dominant palette color.

    Returns array of shape (width, 3) with RGB values.
    """
    from scipy.spatial import cKDTree

    alpha = img_array[:, :, 3]
    height, width = alpha.shape
    tree = cKDTree(palette.astype(np.float64))

    colors = np.zeros((width, 3), dtype=np.uint8)
    for x in range(width):
        col = img_array[:, x]
        opaque_mask = col[:, 3] > 128
        if np.any(opaque_mask):
            col_pixels = col[opaque_mask, :3].astype(np.float64)
            _, indices = tree.query(col_pixels)
            # Most common palette color in this column
            unique_idx, counts = np.unique(indices, return_counts=True)
            dominant_idx = unique_idx[np.argmax(counts)]
            colors[x] = palette[dominant_idx]
        else:
            colors[x] = [0, 0, 0]

    return colors


def analyze_slope(heightmap: np.ndarray) -> list:
    """Analyze slope to find waterfall placement candidates.

    Returns list of candidate dicts with segment index, height, and steepness.
    Uses two strategies:
    1. Strict valleys: slope changes from negative to positive
    2. Relaxed local minima: height is lower than both neighbors
    """
    segments = len(heightmap)
    slope = np.diff(heightmap)

    valleys = []

    # Strategy 1: Strict valleys
    for i in range(1, segments - 1):
        if slope[i - 1] < 0 and slope[i] > 0:
            steepness_left = abs(slope[i - 1])
            steepness_right = abs(slope[i])
            combined = steepness_left + steepness_right
            valleys.append({
                "segment": i,
                "height_pct": float(heightmap[i]),
                "steepness_left": float(steepness_left),
                "steepness_right": float(steepness_right),
                "combined_steepness": float(combined),
            })

    # Strategy 2: Relaxed local minima (if no strict valleys found)
    if not valleys:
        for i in range(2, segments - 2):
            # Local minimum: lower than neighbors within window
            window = 10
            left_max = np.max(heightmap[max(0, i - window):i])
            right_max = np.max(heightmap[i + 1:min(segments, i + window + 1)])
            local_avg = (left_max + right_max) / 2.0

            # Must be a dip (lower than surrounding area)
            if heightmap[i] < local_avg * 0.95:
                steepness_left = abs(left_max - heightmap[i]) / max(window, 1)
                steepness_right = abs(right_max - heightmap[i]) / max(window, 1)
                combined = steepness_left + steepness_right

                valleys.append({
                    "segment": i,
                    "height_pct": float(heightmap[i]),
                    "steepness_left": float(steepness_left),
                    "steepness_right": float(steepness_right),
                    "combined_steepness": float(combined),
                })

    # Strategy 3: Steepest sections (if still no candidates)
    if not valleys:
        # Find the 3 steepest positive slopes (ascending sections)
        for i in range(len(slope)):
            if slope[i] > 0:
                valleys.append({
                    "segment": i,
                    "height_pct": float(heightmap[i]),
                    "steepness_left": 0.0,
                    "steepness_right": float(abs(slope[i])),
                    "combined_steepness": float(abs(slope[i])),
                })
        # Sort by steepness, take top 3 with spacing
        valleys.sort(key=lambda v: v["combined_steepness"], reverse=True)
        filtered = []
        for v in valleys:
            if not any(abs(v["segment"] - f["segment"]) < 80 for f in filtered):
                filtered.append(v)
            if len(filtered) >= 3:
                break
        valleys = filtered

    return valleys


def select_waterfall_locations(
    candidates: list, count: int = 3, min_segment_distance: int = 80
) -> list:
    """Select best waterfall locations with minimum separation.

    Prefers: high steepness + medium height (not at peak, not at base).
    """
    if not candidates:
        return []

    # Strategy 3 already returns filtered results, skip re-selection
    if len(candidates) > 0 and candidates[0].get("steepness_left", 0) == 0:
        # Strategy 3 results - already filtered
        selected = candidates[:count]
    else:
        # Strategy 1 or 2 - score and select with spacing
        for c in candidates:
            height_score = 1.0 - abs(c["height_pct"] - 0.5)
            c["score"] = c["combined_steepness"] * 0.6 + height_score * 0.4

        sorted_candidates = sorted(candidates, key=lambda c: c.get("score", 0), reverse=True)
        selected = []
        for cand in sorted_candidates:
            if len(selected) >= count:
                break
            too_close = any(
                abs(cand["segment"] - s["segment"]) < min_segment_distance
                for s in selected
            )
            if not too_close:
                selected.append(cand)

    return selected


def find_peaks(heightmap: np.ndarray) -> list:
    """Find peaks in the heightmap."""
    peaks = []
    for i in range(1, len(heightmap) - 1):
        if heightmap[i] > heightmap[i - 1] and heightmap[i] > heightmap[i + 1]:
            peaks.append({"segment": i, "height_pct": float(heightmap[i])})
    return peaks


def analyze_sprite(sprite_path: str) -> dict:
    """Full analysis of a mountain sprite."""
    img = Image.open(sprite_path).convert("RGBA")
    img_array = np.array(img)

    heightmap = extract_heightmap(img_array)
    palette = extract_palette(img_array)
    per_column_color = extract_per_column_color(img_array, palette)

    # Resample to 640 segments
    segments = 640
    x_orig = np.arange(len(heightmap))
    x_new = np.linspace(0, len(heightmap) - 1, segments)
    heightmap_resampled = np.interp(x_new, x_orig, heightmap)
    color_indices = np.clip(np.round(x_new).astype(int), 0, len(per_column_color) - 1)
    color_resampled = per_column_color[color_indices]

    # Analysis
    slope_candidates = analyze_slope(heightmap_resampled)
    waterfall_locations = select_waterfall_locations(slope_candidates, count=3)
    peaks = find_peaks(heightmap_resampled)

    # Add angle_rad to waterfall locations
    for wf in waterfall_locations:
        wf["angle_rad"] = float((wf["segment"] / segments) * 2 * np.pi)

    # Add angle_rad to peaks
    for p in peaks:
        p["angle_rad"] = float((p["segment"] / segments) * 2 * np.pi)

    # Metrics
    silhouette_pct = float(np.max(heightmap)) * 100
    content_columns = int(np.sum(heightmap > 0))
    content_pct = (content_columns / len(heightmap)) * 100

    return {
        "source": str(sprite_path),
        "dimensions": {"width": img.width, "height": img.height},
        "palette_colors": int(len(palette)),
        "palette_rgb": palette.tolist(),
        "heightmap_raw": heightmap.tolist(),
        "heightmap_640": heightmap_resampled.tolist(),
        "per_column_color_640": color_resampled.tolist(),
        "metrics": {
            "silhouette_max_pct": round(silhouette_pct, 1),
            "content_columns": content_columns,
            "content_pct": round(content_pct, 1),
        },
        "peaks": peaks,
        "waterfall_candidates": slope_candidates,
        "waterfall_locations": waterfall_locations,
        "segments": segments,
    }


def main():
    parser = argparse.ArgumentParser(description="Analyze mountain sprites for 3D generation")
    parser.add_argument("sprite", help="Path to mountain sprite PNG")
    parser.add_argument("--output", "-o", help="Output JSON path")
    args = parser.parse_args()

    result = analyze_sprite(args.sprite)

    output = json.dumps(result, indent=2)
    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
        print(f"Analysis written to {args.output}")
    else:
        print(output)


if __name__ == "__main__":
    main()
