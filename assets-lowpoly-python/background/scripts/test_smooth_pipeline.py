#!/usr/bin/env python3
"""Prototype and validate the organic continuous mountain generator."""

import json
import math
from pathlib import Path
import numpy as np
from scipy.ndimage import gaussian_filter1d
from shapely.geometry import Point, Polygon
import trimesh
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

BAND_COLORS = {
    "base": np.array([170, 142, 94], dtype=np.float64),    # #AA8E5E (sand/tan)
    "lower": np.array([142, 114, 70], dtype=np.float64),   # #8E7246 (lower slope)
    "mid": np.array([114, 88, 50], dtype=np.float64),      # #725832 (mid cliff)
    "upper": np.array([88, 66, 36], dtype=np.float64),     # #584224 (upper ridge)
    "summit": np.array([66, 48, 26], dtype=np.float64),    # #42301A (summit rock)
}

ELEV_THRESHOLDS = [0.0, 25.0, 50.0, 75.0, 100.0, 150.0]
COLOR_KEYS = ["base", "base", "lower", "mid", "upper", "summit"]


def interpolate_elevation_color(elevation: float) -> np.ndarray:
    """Interpolate vertex color smoothly based on continuous elevation."""
    if elevation <= 0.0:
        return BAND_COLORS["base"].copy()
    if elevation >= 150.0:
        return BAND_COLORS["summit"].copy()

    for idx in range(len(ELEV_THRESHOLDS) - 1):
        e0 = ELEV_THRESHOLDS[idx]
        e1 = ELEV_THRESHOLDS[idx + 1]
        if e0 <= elevation <= e1:
            t = (elevation - e0) / (e1 - e0)
            c0 = BAND_COLORS[COLOR_KEYS[idx]]
            c1 = BAND_COLORS[COLOR_KEYS[idx + 1]]
            return c0 * (1.0 - t) + c1 * t
    return BAND_COLORS["summit"].copy()


def smooth_circular_heightmap(heightmap: np.ndarray, sigma: float = 4.0) -> np.ndarray:
    """Apply Gaussian smoothing with circular boundary conditions (360-degree wrap-around)."""
    n = len(heightmap)
    extended = np.tile(heightmap, 3)
    smoothed = gaussian_filter1d(extended, sigma=sigma, mode="wrap")
    return smoothed[n:2*n]


def compute_continuous_mesh(peak_heightmap, radius, depth, height_scale, segments=640, rows=6):
    r_offsets = [-depth * 0.50, -depth * 0.30, -depth * 0.10, 0.0, depth * 0.15, depth * 0.35]
    h_multipliers = [0.0, 0.25, 0.65, 1.0, 0.70, 0.15]
    row_tints = [
        [0.60, 0.55, 0.50],  # inner apron
        [0.80, 0.75, 0.70],  # lower talus
        [0.95, 0.90, 0.85],  # mid cliff
        [1.10, 1.05, 1.00],  # summit ridge
        [0.85, 0.80, 0.75],  # rear crest
        [0.55, 0.50, 0.45],  # outer skirt
    ]

    vertices = []
    faces = []
    base_colors = []

    # Build vertices
    for i in range(segments):
        angle = (i / segments) * 2 * np.pi
        cos_a = np.cos(angle)
        sin_a = np.sin(angle)
        h_peak = peak_heightmap[i]
        # Base color from continuous elevation
        col = interpolate_elevation_color(h_peak / height_scale)

        for row in range(rows):
            r = radius + r_offsets[row]
            x = r * cos_a
            z = r * sin_a
            y = h_peak * h_multipliers[row]
            vertices.append([x, y, z])
            tint = row_tints[row]
            r_c = col[0] * tint[0]
            g_c = col[1] * tint[1]
            b_c = col[2] * tint[2]
            base_colors.append([r_c, g_c, b_c])

    # Regular quads (winding for inward+up normals)
    for i in range(segments - 1):
        for row in range(rows - 1):
            v0 = i * rows + row
            v1 = i * rows + row + 1
            v2 = (i + 1) * rows + row
            v3 = (i + 1) * rows + row + 1
            faces.append([v0, v2, v1])
            faces.append([v2, v3, v1])

    # Seam quads
    for row in range(rows - 1):
        v0 = (segments - 1) * rows + row
        v1 = (segments - 1) * rows + row + 1
        v2 = row
        v3 = row + 1
        faces.append([v0, v2, v1])
        faces.append([v2, v3, v1])

    verts = np.array(vertices, dtype=np.float64)
    faces_arr = np.array(faces, dtype=np.int64)

    # Compute vertex normals
    mesh_temp = trimesh.Trimesh(vertices=verts, faces=faces_arr, process=False)
    vertex_normals = mesh_temp.vertex_normals

    # Directional sun lighting: South-East sun at 45 deg elevation
    sun_dir = np.array([0.4, 0.8, -0.45], dtype=np.float64)
    sun_dir /= np.linalg.norm(sun_dir)

    vertex_colors = []
    for idx, norm in enumerate(vertex_normals):
        dot = np.dot(norm, sun_dir)
        # Low-poly arcade shading curve: ambient 0.55 + diffuse 0.45
        light = np.clip(0.60 + 0.40 * dot, 0.45, 1.15)
        bc = base_colors[idx]
        r_lit = int(np.clip(bc[0] * light, 0, 255))
        g_lit = int(np.clip(bc[1] * light, 0, 255))
        b_lit = int(np.clip(bc[2] * light, 0, 255))
        vertex_colors.append([r_lit, g_lit, b_lit, 255])

    mesh = trimesh.Trimesh(
        vertices=verts,
        faces=faces_arr,
        vertex_colors=np.array(vertex_colors, dtype=np.uint8),
        process=False,
    )
    return mesh


print("Continuous mesh builder test ready.")
