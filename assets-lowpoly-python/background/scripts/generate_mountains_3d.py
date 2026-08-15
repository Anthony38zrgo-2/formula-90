#!/usr/bin/env python3
"""Generate procedural 3D mountain rings and sky dome for Formula-90 backgrounds.

Reads sprite analysis JSON (from analyze_mountains.py), generates:
- Far mountains ring (640 segments, 8 variants, vertex colors)
- Near mountains ring (640 segments, 8 variants, vertex colors)
- Sky dome (hemisphere with gradient vertex colors)
- Waterfall placement data

Exports GLBs with vertex color material injection and nearest filtering.

Usage:
    python generate_mountains_3d.py <far_analysis.json> <near_analysis.json> <output_dir>
"""

import argparse
import hashlib
import json
import struct
import sys
from pathlib import Path

import numpy as np
import trimesh


# --- Configuration ---

SEGMENTS = 640
VARIANT_COUNT = 8
VARIANT_NOISE_SCALE = 0.05

RADIUS_FAR = 700.0
RADIUS_NEAR = 600.0
HEIGHT_SCALE_FAR = 150.0
HEIGHT_SCALE_NEAR = 100.0

# Far mountains sprite is 38% height, reference is 65% -> scale up
HEIGHT_SCALE_FACTOR_FAR = 1.71

# Radial depth: how far the terrain extends backward from the peak
DEPTH_FAR = 300.0
DEPTH_NEAR = 200.0

# Rows per angular segment: inner_base, inner_slope, peak, outer_slope
TERRAIN_ROWS = 4

SKY_DOME_RADIUS = 1500.0
SKY_DOME_RINGS = 16
SKY_DOME_SEGMENTS = 64

ZENITH_COLOR = [0.18, 0.42, 0.82]
HORIZON_COLOR = [0.95, 0.92, 0.82]
GROUND_COLOR = [0.72, 0.68, 0.55]


# --- Geometry Generation ---

def generate_mountain_ring_segment(
    heightmap: np.ndarray,
    colors: np.ndarray,
    radius: float,
    total_segments: int,
    height_scale: float,
    depth: float,
    seg_start: int,
    seg_end: int,
    noise_scale: float = 0.0,
    seed: int = 0,
) -> trimesh.Trimesh:
    """Generate a terrain segment of the mountain ring.

    Creates a curved terrain strip with radial depth, not a thin wall.
    Each angular segment has 4 rows of vertices:
      V0: inner_base  (r = radius - depth*0.5,  y = 0)       — ground near track
      V1: inner_slope (r = radius - depth*0.2,  y = h*0.35)  — ascending slope
      V2: peak        (r = radius,              y = h)        — mountain crest
      V3: outer_slope (r = radius + depth*0.3,  y = h*0.25)  — descending back

    Args:
        heightmap: 1D array of normalized heights [0..1] per column.
        colors: (N, 3) array of RGB per column.
        radius: Ring radius in meters (position of peak).
        total_segments: Total angular segments (640).
        height_scale: Max height in meters.
        depth: Radial depth in meters.
        seg_start: Start segment index.
        seg_end: End segment index.
        noise_scale: Amplitude of height noise.
        seed: RNG seed for noise.

    Returns:
        Trimesh with vertices, faces, vertex_colors.
    """
    rng = np.random.default_rng(seed)
    vertices = []
    faces = []
    vertex_colors = []

    seg_count = seg_end - seg_start
    rows = TERRAIN_ROWS

    # Radial offsets from peak for each row
    r_offsets = [
        -depth * 0.5,   # V0: inner base (near track)
        -depth * 0.2,   # V1: inner slope
        0.0,            # V2: peak
        depth * 0.3,    # V3: outer slope
    ]

    # Height multipliers for each row
    h_multipliers = [0.0, 0.35, 1.0, 0.25]

    # Color tint per row (darker at base, lighter at peak, dark at back)
    row_tints = [
        [0.6, 0.55, 0.5],   # V0: dark ground
        [0.85, 0.8, 0.75],  # V1: mid slope
        [1.0, 1.0, 1.0],    # V2: peak (full color)
        [0.5, 0.48, 0.45],  # V3: dark back face
    ]

    for i in range(seg_count):
        global_i = seg_start + i
        angle = (global_i / total_segments) * 2 * np.pi

        cos_a = np.cos(angle)
        sin_a = np.sin(angle)

        h = heightmap[global_i % len(heightmap)]
        if noise_scale > 0:
            h = np.clip(h + rng.uniform(-noise_scale, noise_scale), 0.0, 1.0)

        col = colors[global_i % len(colors)]

        for row in range(rows):
            r = radius + r_offsets[row]
            x = r * cos_a
            z = r * sin_a
            y = h * height_scale * h_multipliers[row]

            # Apply depth noise to break uniformity
            if noise_scale > 0 and row != 2:  # don't noise the peak
                y = max(0.0, y + rng.uniform(-noise_scale * height_scale * 0.3,
                                              noise_scale * height_scale * 0.3))

            vertices.append([x, y, z])

            # Vertex color: sprite color * row tint
            tint = row_tints[row]
            r_col = int(np.clip(col[0] * tint[0], 0, 255))
            g_col = int(np.clip(col[1] * tint[1], 0, 255))
            b_col = int(np.clip(col[2] * tint[2], 0, 255))
            vertex_colors.append([r_col, g_col, b_col, 255])

    # Faces: grid between adjacent segments and rows
    for i in range(seg_count - 1):
        for row in range(rows - 1):
            # Current segment vertices
            v0 = i * rows + row
            v1 = i * rows + row + 1
            # Next segment vertices
            v2 = (i + 1) * rows + row
            v3 = (i + 1) * rows + row + 1

            faces.append([v0, v2, v1])
            faces.append([v1, v2, v3])

    # Close the ring: connect last segment to first
    if seg_count > 1:
        for row in range(rows - 1):
            v0 = (seg_count - 1) * rows + row
            v1 = (seg_count - 1) * rows + row + 1
            v2 = 0 * rows + row
            v3 = 0 * rows + row + 1

            faces.append([v0, v2, v1])
            faces.append([v1, v2, v3])

    mesh = trimesh.Trimesh(
        vertices=np.array(vertices, dtype=np.float64),
        faces=np.array(faces, dtype=np.int64),
        vertex_colors=np.array(vertex_colors, dtype=np.uint8),
        process=False,
    )
    return mesh


def generate_full_ring(
    heightmap: np.ndarray,
    colors: np.ndarray,
    radius: float,
    height_scale: float,
    depth: float,
    height_scale_factor: float = 1.0,
) -> trimesh.Trimesh:
    """Generate a complete mountain ring with 8 variants.

    The sprite is the seed. Variants are created by rotation, mirror, and noise.
    """
    segments_per_variant = SEGMENTS // VARIANT_COUNT

    # Apply height scale factor (for far mountains)
    if height_scale_factor != 1.0:
        heightmap = np.clip(heightmap * height_scale_factor, 0.0, 1.0)

    # Variant definitions: (rotation, mirror, noise_scale, seed)
    variants = [
        (0, False, 0.0, 0),
        (SEGMENTS // 4, False, 0.0, 1),
        (SEGMENTS // 2, True, 0.0, 2),
        (3 * SEGMENTS // 4, True, 0.0, 3),
        (0, False, VARIANT_NOISE_SCALE, 4),
        (SEGMENTS // 4, False, VARIANT_NOISE_SCALE, 5),
        (SEGMENTS // 2, True, VARIANT_NOISE_SCALE, 6),
        (3 * SEGMENTS // 4, True, VARIANT_NOISE_SCALE, 7),
    ]

    ring_segments = []
    for v_idx, (rotation, mirror, noise, seed) in enumerate(variants):
        h_var = np.roll(heightmap, rotation)
        c_var = np.roll(colors, rotation, axis=0)
        if mirror:
            h_var = h_var[::-1]
            c_var = c_var[::-1]

        seg_start = v_idx * segments_per_variant
        seg_end = (v_idx + 1) * segments_per_variant

        seg_mesh = generate_mountain_ring_segment(
            h_var, c_var, radius, SEGMENTS, height_scale, depth,
            seg_start, seg_end, noise, seed
        )
        ring_segments.append(seg_mesh)

    full_ring = trimesh.util.concatenate(ring_segments)
    full_ring.merge_vertices()
    full_ring.fix_normals()
    return full_ring


def generate_sky_dome() -> trimesh.Trimesh:
    """Generate a hemisphere with gradient vertex colors.

    zenith (blue) -> horizon (warm) -> ground (brown)
    """
    vertices = []
    faces = []
    vertex_colors = []

    zenith = np.array(ZENITH_COLOR)
    horizon = np.array(HORIZON_COLOR)
    ground = np.array(GROUND_COLOR)

    # Top vertex (zenith)
    vertices.append([0.0, SKY_DOME_RADIUS, 0.0])
    vertex_colors.append([int(zenith[0]*255), int(zenith[1]*255), int(zenith[2]*255), 255])

    # Ring vertices
    for ring in range(1, SKY_DOME_RINGS + 1):
        t = ring / SKY_DOME_RINGS  # 0 at top, 1 at horizon
        phi = t * (np.pi / 2)  # 0 = zenith, pi/2 = horizon
        y = SKY_DOME_RADIUS * np.cos(phi)
        r = SKY_DOME_RADIUS * np.sin(phi)

        # Color interpolation: zenith -> horizon -> ground
        if t < 0.5:
            col = zenith + (horizon - zenith) * (t * 2.0)
        else:
            col = horizon + (ground - horizon) * ((t - 0.5) * 2.0)

        col_u8 = [int(np.clip(c * 255, 0, 255)) for c in col]

        for seg in range(SKY_DOME_SEGMENTS):
            theta = (seg / SKY_DOME_SEGMENTS) * 2 * np.pi
            x = r * np.cos(theta)
            z = r * np.sin(theta)
            vertices.append([x, y, z])
            vertex_colors.append([col_u8[0], col_u8[1], col_u8[2], 255])

    # Faces: connect zenith to first ring
    for seg in range(SKY_DOME_SEGMENTS):
        next_seg = (seg + 1) % SKY_DOME_SEGMENTS
        v0 = 0  # zenith
        v1 = 1 + seg
        v2 = 1 + next_seg
        faces.append([v0, v1, v2])

    # Faces: connect ring to ring
    for ring in range(SKY_DOME_RINGS - 1):
        base_curr = 1 + ring * SKY_DOME_SEGMENTS
        base_next = 1 + (ring + 1) * SKY_DOME_SEGMENTS
        for seg in range(SKY_DOME_SEGMENTS):
            next_seg = (seg + 1) % SKY_DOME_SEGMENTS
            v0 = base_curr + seg
            v1 = base_curr + next_seg
            v2 = base_next + seg
            v3 = base_next + next_seg
            faces.append([v0, v2, v1])
            faces.append([v1, v2, v3])

    mesh = trimesh.Trimesh(
        vertices=np.array(vertices, dtype=np.float64),
        faces=np.array(faces, dtype=np.int64),
        vertex_colors=np.array(vertex_colors, dtype=np.uint8),
        process=False,
    )
    mesh.merge_vertices()
    mesh.fix_normals()
    return mesh


# --- GLB Patching (from grandstand generator) ---

def inject_vertex_color_material(glb_path: str) -> None:
    """Patch GLB JSON to add pbrMetallicRoughness material with baseColorFactor=[1,1,1,1].

    This ensures Godot enables vertex_color_use_as_albedo automatically.
    """
    with open(glb_path, "rb") as f:
        data = f.read()

    # GLB header: magic (4) + version (4) + length (4)
    magic = data[:4]
    if magic != b"glTF":
        return

    version = struct.unpack("<I", data[4:8])[0]
    total_length = struct.unpack("<I", data[8:12])[0]

    # Find JSON chunk
    offset = 12
    while offset < total_length:
        chunk_length = struct.unpack("<I", data[offset:offset+4])[0]
        chunk_type = data[offset+4:offset+8]
        chunk_data = data[offset+8:offset+8+chunk_length]

        if chunk_type == b"JSON":
            gltf_json = json.loads(chunk_data.decode("utf-8"))

            # Add material if not present
            if "materials" not in gltf_json:
                gltf_json["materials"] = []

            # Check if vertex color material already exists
            has_vc_mat = any(
                m.get("name") == "VertexColorMaterial"
                for m in gltf_json["materials"]
            )

            if not has_vc_mat:
                vc_material = {
                    "name": "VertexColorMaterial",
                    "pbrMetallicRoughness": {
                        "baseColorFactor": [1.0, 1.0, 1.0, 1.0],
                        "metallicFactor": 0.0,
                        "roughnessFactor": 1.0,
                    },
                }
                mat_index = len(gltf_json["materials"])
                gltf_json["materials"].append(vc_material)

                # Assign material to all meshes
                for mesh in gltf_json.get("meshes", []):
                    for prim in mesh.get("primitives", []):
                        prim["material"] = mat_index

            # Re-encode JSON chunk
            new_json = json.dumps(gltf_json, separators=(",", ":")).encode("utf-8")
            # Pad to 4-byte alignment
            while len(new_json) % 4 != 0:
                new_json += b" "
            new_json_length = len(new_json)

            # Rebuild GLB
            new_data = data[:offset]  # everything before JSON chunk
            new_data += struct.pack("<I", new_json_length)
            new_data += b"JSON"
            new_data += new_json
            # Copy remaining chunks
            remaining_offset = offset + 8 + chunk_length
            new_data += data[remaining_offset:]
            # Update total length
            new_data = (
                new_data[:8]
                + struct.pack("<I", len(new_data))
                + new_data[12:]
            )

            with open(glb_path, "wb") as f:
                f.write(new_data)
            return

        offset += 8 + chunk_length


def enforce_nearest_sampling(glb_path: str) -> None:
    """Patch GLB JSON to force NEAREST texture sampling."""
    with open(glb_path, "rb") as f:
        data = f.read()

    magic = data[:4]
    if magic != b"glTF":
        return

    total_length = struct.unpack("<I", data[8:12])[0]
    offset = 12

    while offset < total_length:
        chunk_length = struct.unpack("<I", data[offset:offset+4])[0]
        chunk_type = data[offset+4:offset+8]
        chunk_data = data[offset+8:offset+8+chunk_length]

        if chunk_type == b"JSON":
            gltf_json = json.loads(chunk_data.decode("utf-8"))

            # Force nearest sampling on all samplers
            for sampler in gltf_json.get("samplers", []):
                sampler["magFilter"] = 9728  # NEAREST
                sampler["minFilter"] = 9728  # NEAREST

            new_json = json.dumps(gltf_json, separators=(",", ":")).encode("utf-8")
            while len(new_json) % 4 != 0:
                new_json += b" "
            new_json_length = len(new_json)

            new_data = data[:offset]
            new_data += struct.pack("<I", new_json_length)
            new_data += b"JSON"
            new_data += new_json
            remaining_offset = offset + 8 + chunk_length
            new_data += data[remaining_offset:]
            new_data = (
                new_data[:8]
                + struct.pack("<I", len(new_data))
                + new_data[12:]
            )

            with open(glb_path, "wb") as f:
                f.write(new_data)
            return

        offset += 8 + chunk_length


# --- Main Pipeline ---

def export_mesh(mesh: trimesh.Trimesh, path: str) -> None:
    """Export mesh to GLB with material injection."""
    scene = trimesh.Scene()
    scene.add_geometry(mesh)
    scene.export(path)
    inject_vertex_color_material(path)


def compute_sha256(path: str) -> str:
    """Compute SHA-256 hash of a file."""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    parser = argparse.ArgumentParser(
        description="Generate procedural 3D mountain rings and sky dome"
    )
    parser.add_argument("far_analysis", help="Path to far_mountains analysis JSON")
    parser.add_argument("near_analysis", help="Path to near_mountains analysis JSON")
    parser.add_argument("output_dir", help="Output directory for GLBs and manifest")
    args = parser.parse_args()

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    # Load analysis data
    with open(args.far_analysis, "r", encoding="utf-8") as f:
        far_data = json.load(f)
    with open(args.near_analysis, "r", encoding="utf-8") as f:
        near_data = json.load(f)

    far_heightmap = np.array(far_data["heightmap_640"])
    far_colors = np.array(far_data["per_column_color_640"], dtype=np.uint8)
    near_heightmap = np.array(near_data["heightmap_640"])
    near_colors = np.array(near_data["per_column_color_640"], dtype=np.uint8)

    print(f"Far mountains: {len(far_heightmap)} segments, {far_data['palette_colors']} colors")
    print(f"Near mountains: {len(near_heightmap)} segments, {near_data['palette_colors']} colors")

    # Generate far mountains ring
    print("Generating far mountains ring...")
    far_ring = generate_full_ring(
        far_heightmap, far_colors, RADIUS_FAR, HEIGHT_SCALE_FAR, DEPTH_FAR, HEIGHT_SCALE_FACTOR_FAR
    )
    far_path = str(output_dir / "far_mountains_ring.glb")
    export_mesh(far_ring, far_path)
    print(f"  -> {far_path} ({far_ring.vertices.shape[0]} verts, {far_ring.faces.shape[0]} faces)")

    # Generate near mountains ring
    print("Generating near mountains ring...")
    near_ring = generate_full_ring(
        near_heightmap, near_colors, RADIUS_NEAR, HEIGHT_SCALE_NEAR, DEPTH_NEAR
    )
    near_path = str(output_dir / "near_mountains_ring.glb")
    export_mesh(near_ring, near_path)
    print(f"  -> {near_path} ({near_ring.vertices.shape[0]} verts, {near_ring.faces.shape[0]} faces)")

    # Generate sky dome
    print("Generating sky dome...")
    sky_dome = generate_sky_dome()
    sky_path = str(output_dir / "sky_dome.glb")
    export_mesh(sky_dome, sky_path)
    print(f"  -> {sky_path} ({sky_dome.vertices.shape[0]} verts, {sky_dome.faces.shape[0]} faces)")

    # Waterfall placement data
    waterfalls = []
    near_wf = near_data.get("waterfall_locations", [])
    for wf in near_wf[:3]:
        angle = wf["angle_rad"]
        x = RADIUS_NEAR * np.cos(angle)
        z = RADIUS_NEAR * np.sin(angle)
        y = wf["height_pct"] * HEIGHT_SCALE_NEAR
        waterfalls.append({
            "segment": wf["segment"],
            "angle_rad": round(angle, 4),
            "position": [round(x, 2), round(y, 2), round(z, 2)],
            "scale_y": 1.4 if wf["combined_steepness"] > 0.15 else 1.15,
        })

    print(f"Waterfall placements: {len(waterfalls)}")

    # Compute SHA-256 hashes
    far_hash = compute_sha256(far_path)
    near_hash = compute_sha256(near_path)
    sky_hash = compute_sha256(sky_path)

    # Write manifest
    manifest = {
        "schema_version": 1,
        "asset": "la_chutana_mountains_3d",
        "coordinate_convention": {
            "forward": "-Z",
            "up": "+Y",
            "right": "+X",
            "unit": "meter",
        },
        "geometry_assets": {
            "far_mountains": "far_mountains_ring.glb",
            "near_mountains": "near_mountains_ring.glb",
            "sky_dome": "sky_dome.glb",
        },
        "generation": {
            "source_sprites": [
                far_data["source"],
                near_data["source"],
            ],
            "palette": "SNES 16-bit quantized",
            "palette_colors_far": far_data["palette_colors"],
            "palette_colors_near": near_data["palette_colors"],
            "segments": SEGMENTS,
            "variants_per_ring": VARIANT_COUNT,
            "variant_noise_scale": VARIANT_NOISE_SCALE,
            "height_scale_far": HEIGHT_SCALE_FAR,
            "height_scale_near": HEIGHT_SCALE_NEAR,
            "height_scale_factor_far": HEIGHT_SCALE_FACTOR_FAR,
            "depth_far": DEPTH_FAR,
            "depth_near": DEPTH_NEAR,
            "terrain_rows": TERRAIN_ROWS,
            "radius_far": RADIUS_FAR,
            "radius_near": RADIUS_NEAR,
            "sky_dome_radius": SKY_DOME_RADIUS,
            "sky_dome_rings": SKY_DOME_RINGS,
            "sky_dome_segments": SKY_DOME_SEGMENTS,
        },
        "waterfalls": waterfalls,
        "validation": {
            "source_sha256": {
                "far_mountains_ring.glb": far_hash,
                "near_mountains_ring.glb": near_hash,
                "sky_dome.glb": sky_hash,
            },
        },
    }

    manifest_path = str(output_dir / "manifest.json")
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)
    print(f"Manifest: {manifest_path}")

    # Write build report
    build_report = {
        "far_mountains": {
            "vertices": int(far_ring.vertices.shape[0]),
            "faces": int(far_ring.faces.shape[0]),
            "bbox_min": far_ring.bounds[0].tolist(),
            "bbox_max": far_ring.bounds[1].tolist(),
            "dimensions": (far_ring.bounds[1] - far_ring.bounds[0]).tolist(),
        },
        "near_mountains": {
            "vertices": int(near_ring.vertices.shape[0]),
            "faces": int(near_ring.faces.shape[0]),
            "bbox_min": near_ring.bounds[0].tolist(),
            "bbox_max": near_ring.bounds[1].tolist(),
            "dimensions": (near_ring.bounds[1] - near_ring.bounds[0]).tolist(),
        },
        "sky_dome": {
            "vertices": int(sky_dome.vertices.shape[0]),
            "faces": int(sky_dome.faces.shape[0]),
            "bbox_min": sky_dome.bounds[0].tolist(),
            "bbox_max": sky_dome.bounds[1].tolist(),
            "dimensions": (sky_dome.bounds[1] - sky_dome.bounds[0]).tolist(),
        },
        "waterfalls": len(waterfalls),
    }

    report_path = str(output_dir / "build_report.json")
    with open(report_path, "w", encoding="utf-8") as f:
        json.dump(build_report, f, indent=2)
    print(f"Build report: {report_path}")

    print("\n=== GENERATION COMPLETE ===")


if __name__ == "__main__":
    main()
