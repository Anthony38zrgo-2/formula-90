#!/usr/bin/env python3
"""Render a camera-view perspective preview of the 3D mountains from the track."""

import json
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import Axes3D
import trimesh

ROOT = Path(__file__).resolve().parents[2]
ASSET_DIR = ROOT / "game" / "assets" / "backgrounds" / "la_chutana_snes_day" / "mountains_3d"
OUT_DIR = ROOT / "reports" / "background"
OUT_DIR.mkdir(parents=True, exist_ok=True)


def render_camera_view():
    manifest_path = ASSET_DIR / "manifest.json"
    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    far_glb = ASSET_DIR / manifest["geometry_assets"]["far_mountains"]
    near_glb = ASSET_DIR / manifest["geometry_assets"]["near_mountains"]

    scene_far = trimesh.load(str(far_glb))
    scene_near = trimesh.load(str(near_glb))

    mesh_far = list(scene_far.geometry.values())[0]
    mesh_near = list(scene_near.geometry.values())[0]

    num_segments = manifest["generation"]["segments"]
    rows = manifest["generation"]["terrain_rows"]

    # Extract 360 profile
    far_peaks = np.array([mesh_far.vertices[i * rows + 3] for i in range(num_segments)])
    near_peaks = np.array([mesh_near.vertices[i * rows + 3] for i in range(num_segments)])

    angles_deg = np.linspace(0, 360, num_segments, endpoint=False)

    # Plot visual horizon from driver camera FOV
    fig = plt.figure(figsize=(16, 6))
    ax = fig.add_subplot(111)

    # Sky gradient background
    sky_y = np.linspace(0, 15, 100)
    for y_val in sky_y:
        t = y_val / 15.0
        # Gradient from horizon warm beige/yellow to zenith blue
        r_col = 0.95 * (1 - t) + 0.18 * t
        g_col = 0.92 * (1 - t) + 0.42 * t
        b_col = 0.82 * (1 - t) + 0.82 * t
        ax.axhspan(y_val, y_val + 0.16, color=(r_col, g_col, b_col), zorder=0)

    # Ground
    ax.axhspan(-2, 0, color=(0.72, 0.68, 0.55), zorder=1)

    # Far mountains in degrees above horizon
    r_far = manifest["generation"]["far"]["radius"]
    r_near = manifest["generation"]["near"]["radius"]

    far_angular_deg = np.degrees(np.arctan2(far_peaks[:, 1] - 1.5, r_far))
    near_angular_deg = np.degrees(np.arctan2(near_peaks[:, 1] - 1.5, r_near))

    # Far massif
    ax.fill_between(angles_deg, 0, far_angular_deg, color="#403928", alpha=0.9, zorder=2, label="Far Mountains (r=1600m)")
    ax.plot(angles_deg, far_angular_deg, color="#2a2418", lw=1.5, zorder=3)

    # Near massif
    ax.fill_between(angles_deg, 0, near_angular_deg, color="#92764a", alpha=0.95, zorder=4, label="Near Mountains (r=1150m)")
    ax.plot(angles_deg, near_angular_deg, color="#5a4526", lw=1.5, zorder=5)

    # Waterfalls
    for wf in manifest.get("waterfalls", []):
        ang_deg = np.degrees(wf["angle_rad"]) % 360
        y_pos_deg = np.degrees(np.arctan2(wf["position"][1], r_near))
        ax.scatter([ang_deg], [y_pos_deg], color="#3498db", s=80, zorder=6, edgecolors="white", label="Waterfall" if wf == manifest["waterfalls"][0] else "")

    ax.set_title("Formula-90 La Chutana: 360° Organic Mountain Silhouette & Horizon Framing", fontsize=14, fontweight="bold", pad=12)
    ax.set_xlabel("Driver Look Direction (Azimuth Angle °)", fontsize=11)
    ax.set_ylabel("Perceived Elevation (Degrees Above Horizon)", fontsize=11)
    ax.set_xlim(0, 360)
    ax.set_ylim(-0.5, 12.0)
    ax.set_xticks(np.arange(0, 361, 45))
    ax.set_xticklabels(["0° (E)", "45° (NE)", "90° (N)", "135° (NW)", "180° (W)", "225° (SW)", "270° (S)", "315° (SE)", "360° (E)"])
    ax.grid(True, linestyle=":", alpha=0.5, zorder=1)
    ax.legend(loc="upper right", framealpha=0.9)

    out_file = OUT_DIR / "mountains_organic_preview.png"
    plt.tight_layout()
    plt.savefig(out_file, dpi=150)
    plt.close()
    print(f"Render saved to: {out_file}")


if __name__ == "__main__":
    render_camera_view()
