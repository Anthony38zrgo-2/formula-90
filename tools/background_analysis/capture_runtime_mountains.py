#!/usr/bin/env python3
"""Generate panoramic elevation and visual profile analysis of the 3D mountains.

Produces:
- 360-degree cylindrical projection of Near and Far rings from the track origin.
- Visual comparison with arcade horizon metrics.
- Output report and visual plot in reports/background/mountains_3d_profile.png.
"""

import json
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import trimesh

ROOT = Path(__file__).resolve().parents[2]
ASSET_DIR = ROOT / "game" / "assets" / "backgrounds" / "la_chutana_snes_day" / "mountains_3d"
OUT_DIR = ROOT / "reports" / "background"
OUT_DIR.mkdir(parents=True, exist_ok=True)


def analyze_mountains_geometry():
    manifest_path = ASSET_DIR / "manifest.json"
    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    far_glb = ASSET_DIR / manifest["geometry_assets"]["far_mountains"]
    near_glb = ASSET_DIR / manifest["geometry_assets"]["near_mountains"]

    scene_far = trimesh.load(str(far_glb))
    scene_near = trimesh.load(str(near_glb))

    mesh_far = list(scene_far.geometry.values())[0]
    mesh_near = list(scene_near.geometry.values())[0]

    # Extract 640 angular segments from vertex peak row (row index 3 for 6-row profile)
    num_segments = manifest["generation"]["segments"]
    rows = manifest["generation"]["terrain_rows"]

    # Peak row is index 3
    far_peaks = np.array([mesh_far.vertices[i * rows + 3] for i in range(num_segments)])
    near_peaks = np.array([mesh_near.vertices[i * rows + 3] for i in range(num_segments)])

    angles_deg = np.linspace(0, 360, num_segments, endpoint=False)

    far_y = far_peaks[:, 1]
    near_y = near_peaks[:, 1]

    # Angular height in degrees from origin (0, 1.5, 0)
    r_far = manifest["generation"]["far"]["radius"]
    r_near = manifest["generation"]["near"]["radius"]

    far_angular_deg = np.degrees(np.arctan2(far_y - 1.5, r_far))
    near_angular_deg = np.degrees(np.arctan2(near_y - 1.5, r_near))

    # Plot
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(14, 8), sharex=True)

    # Plot 1: World Height in meters
    ax1.plot(angles_deg, far_y, label=f"Far Mountains (R={r_far:.0f}m, Max={far_y.max():.1f}m)", color="#403928", lw=2)
    ax1.plot(angles_deg, near_y, label=f"Near Mountains (R={r_near:.0f}m, Max={near_y.max():.1f}m)", color="#a08658", lw=2)
    ax1.fill_between(angles_deg, 0, far_y, color="#605338", alpha=0.3)
    ax1.fill_between(angles_deg, 0, near_y, color="#c0a068", alpha=0.5)

    # Mark waterfalls
    for wf in manifest.get("waterfalls", []):
        ang_deg = math_deg = np.degrees(wf["angle_rad"]) % 360
        y_pos = wf["position"][1]
        ax1.axvline(ang_deg, color="#3498db", linestyle="--", alpha=0.7)
        ax1.scatter([ang_deg], [y_pos], color="#2980b9", s=60, zorder=5, label="Waterfall" if wf == manifest["waterfalls"][0] else "")

    ax1.set_title("Formula-90 La Chutana: 360° Topographic Mountain Profiles (BG3-010)", fontsize=14, fontweight="bold")
    ax1.set_ylabel("World Elevation (meters)", fontsize=12)
    ax1.grid(True, linestyle=":", alpha=0.6)
    ax1.legend(loc="upper right", framealpha=0.9)
    ax1.set_ylim(0, max(far_y.max(), near_y.max()) * 1.15)

    # Plot 2: Perceived Angular Height from Driver Camera (FOV perspective)
    ax2.plot(angles_deg, far_angular_deg, label="Far Mountain Crest (Angular °)", color="#403928", lw=2)
    ax2.plot(angles_deg, near_angular_deg, label="Near Mountain Ridge (Angular °)", color="#a08658", lw=2)
    ax2.fill_between(angles_deg, 0, far_angular_deg, color="#605338", alpha=0.3)
    ax2.fill_between(angles_deg, 0, near_angular_deg, color="#c0a068", alpha=0.5)

    ax2.set_xlabel("Track View Azimuth Angle (degrees)", fontsize=12)
    ax2.set_ylabel("Angular Elevation (degrees above horizon)", fontsize=12)
    ax2.set_xlim(0, 360)
    ax2.set_xticks(np.arange(0, 361, 45))
    ax2.set_xticklabels(["0° (E)", "45° (NE)", "90° (N)", "135° (NW)", "180° (W)", "225° (SW)", "270° (S)", "315° (SE)", "360° (E)"])
    ax2.grid(True, linestyle=":", alpha=0.6)
    ax2.legend(loc="upper right", framealpha=0.9)

    out_png = OUT_DIR / "mountains_3d_profile.png"
    plt.tight_layout()
    plt.savefig(out_png, dpi=150)
    plt.close()

    print(f"Analysis saved: {out_png}")
    print(f"Far peaks range: {far_y.min():.1f}m - {far_y.max():.1f}m (angular: {far_angular_deg.min():.1f}° - {far_angular_deg.max():.1f}°)")
    print(f"Near peaks range: {near_y.min():.1f}m - {near_y.max():.1f}m (angular: {near_angular_deg.min():.1f}° - {near_angular_deg.max():.1f}°)")
    print(f"Far vertices: {len(mesh_far.vertices)}, faces: {len(mesh_far.faces)}")
    print(f"Near vertices: {len(mesh_near.vertices)}, faces: {len(mesh_near.faces)}")


if __name__ == "__main__":
    analyze_mountains_geometry()
