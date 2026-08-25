from __future__ import annotations

import argparse
import hashlib
import json
import math
import random
from pathlib import Path

import numpy as np
import trimesh
from PIL import Image, ImageDraw

from voxelize_semantic_tree import _hex

from audit_vegetation_3d_library import _vertex_colors


def _unit(vector: np.ndarray) -> np.ndarray:
    return vector / max(float(np.linalg.norm(vector)), 1e-9)


def _mix(start: np.ndarray, end: np.ndarray, amount: float) -> np.ndarray:
    return start * (1.0 - amount) + end * amount


def _tone(color: tuple[int, int, int], factor: float) -> tuple[int, int, int]:
    return tuple(max(0, min(255, round(channel * factor))) for channel in color)


def tapered_tube(start, end, start_radius, end_radius, start_color, end_color):
    direction = end - start
    length = float(np.linalg.norm(direction))
    if length < 1e-6:
        return None
    axis = direction / length
    reference = np.asarray([0.0, 1.0, 0.0]) if abs(axis[1]) < 0.9 else np.asarray([1.0, 0.0, 0.0])
    normal = _unit(np.cross(axis, reference))
    binormal = _unit(np.cross(axis, normal))
    vertices, colors = [], []
    sides = 4
    for center, radius, color in ((start, start_radius, start_color), (end, end_radius, end_color)):
        for side in range(sides):
            angle = math.tau * side / sides
            vertices.append(center + radius * (normal * math.cos(angle) + binormal * math.sin(angle)))
            colors.append((*color, 255))
    faces = []
    for side in range(sides):
        nxt = (side + 1) % sides
        faces.extend(((side, nxt, sides + nxt), (side, sides + nxt, sides + side)))
    faces.extend(((0, 2, 1), (0, 3, 2), (4, 5, 6), (4, 6, 7)))
    mesh = trimesh.Trimesh(vertices=np.asarray(vertices), faces=np.asarray(faces), process=False)
    mesh.visual.vertex_colors = np.asarray(colors, dtype=np.uint8)
    return mesh


def _tree_skeleton(recipe: dict, rng: random.Random):
    height, width, depth = (float(recipe[key]) for key in ("height_m", "width_m", "depth_m"))
    lean = float(recipe.get("lean", 0.0))
    asymmetry = float(recipe.get("asymmetry", 0.0))
    branch_start = float(recipe.get("branch_start_ratio", 0.42))
    primary_count = int(recipe.get("primary_count", 6))
    trunk_radius = height * float(recipe.get("trunk_radius_ratio", 0.035))
    segments, endpoints = [], []
    trunk_points = []
    for index in range(7):
        t = index / 6.0
        trunk_points.append(np.asarray([
            lean * width * t * t + math.sin(t * math.pi * 1.7) * width * 0.012,
            height * (0.01 + 0.78 * t),
            math.sin(t * math.pi * 1.2 + 0.7) * depth * 0.012,
        ]))
    for index, (start, end) in enumerate(zip(trunk_points, trunk_points[1:])):
        t0, t1 = index / 6.0, (index + 1) / 6.0
        segments.append({"start": start, "end": end, "r0": trunk_radius * (1.0 - 0.50 * t0), "r1": trunk_radius * (1.0 - 0.50 * t1), "level": 0})
    for index in range(primary_count):
        phase = math.tau * index / primary_count + rng.uniform(-0.28, 0.28)
        attach_ratio = branch_start + 0.34 * ((index * 0.61803398875) % 1.0)
        attach = _mix(trunk_points[0], trunk_points[-1], attach_ratio / 0.79)
        reach = width * rng.uniform(0.27, 0.44) * (1.0 + asymmetry * math.cos(phase))
        target = attach + np.asarray([math.cos(phase) * reach + lean * width * 0.35, height * rng.uniform(0.15, 0.29), math.sin(phase) * depth * rng.uniform(0.24, 0.43)])
        elbow = _mix(attach, target, 0.48) + np.asarray([-math.sin(phase) * width * rng.uniform(-0.035, 0.035), height * rng.uniform(0.015, 0.045), math.cos(phase) * depth * rng.uniform(-0.035, 0.035)])
        radius = trunk_radius * rng.uniform(0.38, 0.52)
        segments.extend((
            {"start": attach, "end": elbow, "r0": radius, "r1": radius * 0.62, "level": 1},
            {"start": elbow, "end": target, "r0": radius * 0.62, "r1": radius * 0.26, "level": 1},
        ))
        endpoints.append(target)
        for child in range(2):
            child_phase = phase + (-0.72 if child == 0 else 0.72) + rng.uniform(-0.20, 0.20)
            child_start = _mix(elbow, target, rng.uniform(0.38, 0.66))
            child_end = target + np.asarray([math.cos(child_phase) * width * rng.uniform(0.10, 0.20), height * rng.uniform(0.04, 0.13), math.sin(child_phase) * depth * rng.uniform(0.10, 0.20)])
            child_radius = radius * 0.25
            segments.append({"start": child_start, "end": child_end, "r0": child_radius, "r1": child_radius * 0.22, "level": 2})
            endpoints.append(child_end)
    root_count = int(recipe.get("root_count", 6))
    base = trunk_points[0]
    for index in range(root_count):
        phase = math.tau * index / root_count + rng.uniform(-0.20, 0.20)
        root_end = np.asarray([math.cos(phase) * width * rng.uniform(0.07, 0.14), -height * rng.uniform(0.018, 0.035), math.sin(phase) * depth * rng.uniform(0.07, 0.14)])
        segments.append({"start": base, "end": root_end, "r0": trunk_radius * 0.72, "r1": trunk_radius * 0.10, "level": -1})
    return segments, endpoints


def _bush_skeleton(recipe: dict, rng: random.Random):
    height, width, depth = (float(recipe[key]) for key in ("height_m", "width_m", "depth_m"))
    stem_count = int(recipe.get("stem_count", 5))
    crawl, asymmetry = float(recipe.get("crawl", 0.0)), float(recipe.get("asymmetry", 0.0))
    segments, endpoints = [], []
    base_radius = max(0.018, height * 0.028)
    for index in range(stem_count):
        phase = math.tau * index / stem_count + rng.uniform(-0.35, 0.35)
        base = np.asarray([math.cos(phase) * width * rng.uniform(0.01, 0.05), 0.0, math.sin(phase) * depth * rng.uniform(0.01, 0.05)])
        reach = width * rng.uniform(0.16, 0.34) * (1.0 + asymmetry * math.cos(phase))
        tip = np.asarray([math.cos(phase) * reach, height * rng.uniform(0.55 - crawl * 0.20, 0.92 - crawl * 0.18), math.sin(phase) * depth * rng.uniform(0.16, 0.34)])
        elbow = _mix(base, tip, 0.48) + np.asarray([math.cos(phase + math.pi / 2) * width * rng.uniform(-0.04, 0.04), height * rng.uniform(0.01, 0.06), math.sin(phase + math.pi / 2) * depth * rng.uniform(-0.04, 0.04)])
        radius = base_radius * rng.uniform(0.75, 1.15)
        segments.extend((
            {"start": base, "end": elbow, "r0": radius, "r1": radius * 0.55, "level": 0},
            {"start": elbow, "end": tip, "r0": radius * 0.55, "r1": radius * 0.18, "level": 1},
        ))
        endpoints.append(tip)
        fork_phase = phase + rng.choice((-1, 1)) * rng.uniform(0.55, 0.95)
        fork_start = _mix(elbow, tip, rng.uniform(0.35, 0.60))
        fork_end = fork_start + np.asarray([math.cos(fork_phase) * width * rng.uniform(0.10, 0.22), height * rng.uniform(0.10, 0.26), math.sin(fork_phase) * depth * rng.uniform(0.10, 0.22)])
        segments.append({"start": fork_start, "end": fork_end, "r0": radius * 0.28, "r1": radius * 0.07, "level": 2})
        endpoints.append(fork_end)
    for index in range(3):
        phase = math.tau * index / 3 + rng.uniform(-0.25, 0.25)
        segments.append({"start": np.asarray([0.0, 0.0, 0.0]), "end": np.asarray([math.cos(phase) * width * 0.10, -height * 0.015, math.sin(phase) * depth * 0.10]), "r0": base_radius * 0.70, "r1": base_radius * 0.08, "level": -1})
    return segments, endpoints


def generate_skeleton(recipe: dict):
    rng = random.Random(int(recipe["seed"]))
    return _tree_skeleton(recipe, rng) if recipe["kind"] == "tree" else _bush_skeleton(recipe, rng)


def generate_foliage_clusters(recipe: dict, endpoints: list[np.ndarray]) -> list[dict]:
    rng = random.Random(int(recipe["seed"]) ^ 0xF011A6E)
    count = int(recipe["cluster_count"])
    height, width, depth = (float(recipe[key]) for key in ("height_m", "width_m", "depth_m"))
    center_y = height * float(recipe.get("canopy_center_ratio", 0.72 if recipe["kind"] == "tree" else 0.42))
    radius_y = height * float(recipe.get("canopy_height_ratio", 0.28 if recipe["kind"] == "tree" else 0.42))
    radius_x = width * float(recipe.get("canopy_width_ratio", 0.48))
    radius_z = depth * float(recipe.get("canopy_depth_ratio", 0.48))
    lean, bias_x = float(recipe.get("lean", 0.0)), float(recipe.get("foliage_bias_x", 0.0)) * width
    centers = []
    exterior_target = min(len(endpoints), round(count * 0.70))
    ordered = sorted(endpoints, key=lambda point: (float(point[1]), float(point[0]), float(point[2])))
    for index in range(exterior_target):
        endpoint = ordered[(index * 7) % len(ordered)]
        centers.append(endpoint + np.asarray([rng.uniform(-radius_x * 0.16, radius_x * 0.16), rng.uniform(-radius_y * 0.12, radius_y * 0.18), rng.uniform(-radius_z * 0.16, radius_z * 0.16)]))
    while len(centers) < count:
        x, y, z = rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-1, 1)
        if x * x + y * y + z * z > 1.0:
            continue
        world_y = center_y + y * radius_y
        centers.append(np.asarray([bias_x + x * radius_x + lean * width * (world_y / max(height, 1e-6)) ** 2, world_y, z * radius_z]))
    clusters = []
    for index, center in enumerate(centers):
        relative = np.asarray([center[0] / max(radius_x, 1e-6), (center[1] - center_y) / max(radius_y, 1e-6), center[2] / max(radius_z, 1e-6)])
        radial = min(1.0, float(np.linalg.norm(relative)))
        tone = 1 if radial < 0.42 else rng.choices([2, 3, 4, 5], [4, 7, 3, 1])[0]
        clusters.append({"center": center, "tone": tone, "angle": rng.uniform(0.0, math.tau), "index": index})
    return clusters


def generate_cluster_cards(recipe: dict, clusters: list[dict], palette: dict):
    rng = random.Random(int(recipe["seed"]) ^ 0xCA4D5)
    leaf_palette = [_hex(value) for value in palette["leaf"]]
    cards_per_cluster = int(recipe.get("cards_per_cluster", 3))
    width, height, depth = (float(recipe[key]) for key in ("width_m", "height_m", "depth_m"))
    base_width = float(recipe.get("card_width_m", max(width, depth) / max(math.sqrt(len(clusters)), 1.0) * 1.18))
    base_height = float(recipe.get("card_height_m", height / max(math.sqrt(len(clusters)), 1.0) * (0.78 if recipe["kind"] == "tree" else 0.92)))
    vertices, faces, colors = [], [], []
    for cluster in clusters:
        for card_index in range(cards_per_cluster):
            angle = cluster["angle"] + math.tau * card_index / max(cards_per_cluster, 1) + rng.uniform(-0.22, 0.22)
            tilt = math.radians(rng.uniform(-48.0, 58.0))
            horizontal = np.asarray([math.cos(angle), 0.0, math.sin(angle)])
            vertical = _unit(np.asarray([math.sin(angle) * math.sin(tilt), math.cos(tilt), -math.cos(angle) * math.sin(tilt)]))
            normal = _unit(np.cross(horizontal, vertical))
            card_width, card_height = base_width * rng.uniform(0.78, 1.24), base_height * rng.uniform(0.78, 1.22)
            spread = float(recipe.get("leaf_cluster_spread_m", base_width * 0.10))
            center = cluster["center"] + np.asarray([
                rng.uniform(-spread, spread),
                rng.uniform(-spread * 0.72, spread * 0.72),
                rng.uniform(-spread, spread),
            ])
            if recipe.get("leaf_style") == "lanceolate":
                # A small pointed leaf with a raised midrib.  Unlike the old crossed
                # canopy cards, every instance reads as one leaf and retains air
                # between neighbouring leaves.
                boundary = (
                    (0.0, -0.52), (0.34, -0.17), (0.28, 0.20),
                    (0.0, 0.54), (-0.28, 0.20), (-0.34, -0.17),
                )
                base = len(vertices)
                vertices.append(center + normal * card_width * rng.uniform(0.035, 0.075))
                vertices.extend(center + horizontal * card_width * u + vertical * card_height * v for u, v in boundary)
                for edge in range(6):
                    first = base + 1 + edge
                    second = base + 1 + (edge + 1) % 6
                    faces.extend(((base, first, second), (base, second, first)))
                base_color = leaf_palette[int(cluster["tone"])]
                ridge_color = _tone(base_color, 1.12)
                colors.append((*ridge_color, 255))
                colors.extend([(*base_color, 255)] * 6)
                continue
            # A card stays perfectly planar, but uses an irregular octagonal outline.
            # With vertex colors and no texture this avoids the visible wall of rectangles
            # produced by plain quads while retaining cheap crossed-card foliage.
            outline = (
                (-0.50, -0.10), (-0.34, -0.40), (-0.05, -0.52), (0.34, -0.40),
                (0.51, -0.05), (0.38, 0.34), (0.08, 0.52), (-0.36, 0.37),
            )
            jittered = [
                center + horizontal * card_width * u * rng.uniform(0.90, 1.08)
                + vertical * card_height * v * rng.uniform(0.90, 1.08)
                for u, v in outline
            ]
            base = len(vertices)
            vertices.append(center)
            vertices.extend(jittered)
            for edge in range(8):
                first = base + 1 + edge
                second = base + 1 + (edge + 1) % 8
                faces.extend(((base, first, second), (base, second, first)))
            colors.extend([(*leaf_palette[int(cluster["tone"])], 255)] * 9)
    mesh = trimesh.Trimesh(vertices=np.asarray(vertices), faces=np.asarray(faces), process=False)
    mesh.visual.vertex_colors = np.asarray(colors, dtype=np.uint8)
    return mesh, len(clusters) * cards_per_cluster


def _fit_to_recipe(wood, foliage, recipe):
    combined = np.vstack((wood.vertices, foliage.vertices))
    mins, maxs = combined.min(axis=0), combined.max(axis=0)
    target = np.asarray([recipe["width_m"], recipe["height_m"], recipe["depth_m"]], dtype=np.float64)
    scale = target / np.maximum(maxs - mins, 1e-9)
    for mesh in (wood, foliage):
        mesh.vertices = mesh.vertices * scale
    combined = np.vstack((wood.vertices, foliage.vertices))
    mins, maxs = combined.min(axis=0), combined.max(axis=0)
    root_depth = float(recipe.get("root_depth_ratio", 0.025)) * float(recipe["height_m"])
    offset = np.asarray([-(mins[0] + maxs[0]) * 0.5, -root_depth - mins[1], -(mins[2] + maxs[2]) * 0.5])
    for mesh in (wood, foliage):
        mesh.apply_translation(offset)


def _mesh_dimensions(wood, foliage):
    vertices = np.vstack((wood.vertices, foliage.vertices))
    extents = vertices.max(axis=0) - vertices.min(axis=0)
    return {"width": float(extents[0]), "height": float(extents[1]), "depth": float(extents[2])}


def build_asset_meshes(recipe: dict, palette: dict):
    segments, endpoints = generate_skeleton(recipe)
    wood_palette = [_hex(value) for value in palette["branch"]]
    parts = []
    for index, segment in enumerate(segments):
        level = int(segment["level"])
        color_index = 1 if level <= 0 else 2 if level == 1 else 3
        if int(hashlib.sha256(f"{recipe['id']}:{index}".encode()).hexdigest()[:2], 16) > 224:
            color_index = min(color_index + 1, len(wood_palette) - 1)
        part = tapered_tube(segment["start"], segment["end"], segment["r0"], segment["r1"], _tone(wood_palette[color_index], 0.96), _tone(wood_palette[color_index], 1.04))
        if part is not None:
            parts.append(part)
    wood = trimesh.util.concatenate(parts)
    clusters = generate_foliage_clusters(recipe, endpoints)
    foliage, card_count = generate_cluster_cards(recipe, clusters, palette)
    _fit_to_recipe(wood, foliage, recipe)
    return wood, foliage, {"cluster_count": len(clusters), "leaf_cards": card_count}


def _project(vertices, yaw, pitch):
    cy, sy, cp, sp = math.cos(yaw), math.sin(yaw), math.cos(pitch), math.sin(pitch)
    rotation_y = np.asarray([[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]])
    rotation_x = np.asarray([[1.0, 0.0, 0.0], [0.0, cp, -sp], [0.0, sp, cp]])
    rotated = vertices @ rotation_y.T @ rotation_x.T
    return rotated[:, :2], rotated[:, 2]


def render_audit(wood, foliage, path: Path):
    image = Image.new("RGBA", (900, 900), (30, 32, 33, 255))
    views = ((0.0, 0.0, "0 deg"), (math.pi / 4, 0.0, "45 deg"), (math.pi / 2, 0.0, "90 deg"), (math.pi / 4, -0.28, "isometric"))
    for view_index, (yaw, pitch, label) in enumerate(views):
        tile_x, tile_y = (view_index % 2) * 450, (view_index // 2) * 450
        projected_meshes, all_points = [], []
        for mesh in (wood, foliage):
            points, depths = _project(mesh.vertices, yaw, pitch)
            projected_meshes.append((mesh, points, depths)); all_points.append(points)
        joined = np.vstack(all_points); mins, maxs = joined.min(axis=0), joined.max(axis=0)
        scale = 350.0 / max(float(maxs[0] - mins[0]), float(maxs[1] - mins[1]), 1e-6)
        center = (mins + maxs) * 0.5
        draw = ImageDraw.Draw(image); primitives = []
        for mesh, points, depths in projected_meshes:
            colors = _vertex_colors(mesh)
            for face in mesh.faces:
                polygon = [(tile_x + 225 + (points[i][0] - center[0]) * scale, tile_y + 215 - (points[i][1] - center[1]) * scale) for i in face]
                primitives.append((float(np.mean(depths[face])), polygon, tuple(int(v) for v in colors[face[0]][:3])))
        for _, polygon, color in sorted(primitives, key=lambda item: item[0]):
            draw.polygon(polygon, fill=(*color, 255))
        draw.text((tile_x + 12, tile_y + 12), label, fill=(238, 238, 238, 255))
    path.parent.mkdir(parents=True, exist_ok=True); image.save(path)


def build_asset(recipe: dict, palette: dict, output_root: Path, audit_root: Path):
    wood, foliage, stats = build_asset_meshes(recipe, palette)
    category = "trees" if recipe["kind"] == "tree" else "bushes"
    glb = output_root / category / f"{recipe['id']}.glb"; glb.parent.mkdir(parents=True, exist_ok=True)
    scene = trimesh.Scene(); scene.add_geometry(wood, node_name=f"{recipe['id']}_wood", geom_name="wood"); scene.add_geometry(foliage, node_name=f"{recipe['id']}_foliage", geom_name="foliage")
    glb.write_bytes(scene.export(file_type="glb"))
    preview = audit_root / f"{recipe['id']}.png"; render_audit(wood, foliage, preview)
    repo_root = output_root.parents[3]
    dimensions = _mesh_dimensions(wood, foliage)
    return {"id": recipe["id"], "kind": recipe["kind"], "family": recipe["family"], "category": category, "glb": glb.relative_to(repo_root).as_posix(), "preview": preview.relative_to(repo_root).as_posix(), "dimensions_m": {key: round(value, 6) for key, value in dimensions.items()}, **stats, "wood_triangles": int(len(wood.faces)), "foliage_triangles": int(len(foliage.faces)), "sha256": hashlib.sha256(glb.read_bytes()).hexdigest(), "lod": False, "collision": False, "foliage_double_sided_geometry": True}


def build_library(spec: dict, repo: Path, *, asset_ids: set[str] | None = None):
    palette_doc = json.loads((repo / spec["palette"]).read_text(encoding="utf-8"))
    output_root = repo / "game/resources/environment/assets"; audit_root = repo / "game/resources/environment/generated/library_audit"
    assets = []
    for recipe in spec["assets"]:
        if asset_ids is not None and recipe["id"] not in asset_ids:
            continue
        palette = palette_doc["categories"]["tree" if recipe["kind"] == "tree" else "bush"]
        assets.append(build_asset(recipe, palette, output_root, audit_root)); print(f"built {recipe['id']}")
    return assets


def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--recipes", type=Path, required=True); parser.add_argument("--repo", type=Path, default=Path.cwd()); parser.add_argument("--asset-id", action="append", dest="asset_ids"); parser.add_argument("--no-manifest", action="store_true")
    args = parser.parse_args(); spec = json.loads(args.recipes.read_text(encoding="utf-8")); selected = set(args.asset_ids) if args.asset_ids else None
    assets = build_library(spec, args.repo, asset_ids=selected)
    if not args.no_manifest:
        if selected is not None:
            raise SystemExit("partial builds require --no-manifest")
        manifest = {"schema_version": 3, "generator": "procedural_lanceolate_vegetation_v3", "assets": assets}
        path = args.repo / "game/resources/environment/assets/manifest.json"; path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        print(json.dumps({"assets": len(assets), "manifest": str(path)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
