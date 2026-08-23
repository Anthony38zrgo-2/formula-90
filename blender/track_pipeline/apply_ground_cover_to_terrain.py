from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image
import numpy as np

from atomic_json import write_json_atomic
from generate_procedural_textures import apply_ground_cover_detail
from texture_forge import make_seamless_edges, save_image_atomic


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_terrain_base(path: Path, texture_size: int) -> Image.Image:
    """Load the curated terrain source at the configured runtime resolution."""
    size = max(1, int(texture_size))
    with Image.open(path) as source:
        base = source.convert("RGB")
    if base.size != (size, size):
        base = base.resize((size, size), Image.Resampling.LANCZOS)
    return base


def apply_semantic_grass_points(base: Image.Image, sources: list[Path], points: list, bounds: list, seed: int, opacity: float) -> Image.Image:
    """Stamp grass bitmaps only at exact SVG-authored world positions."""
    width, height = base.size
    min_x, min_z, max_x, max_z = (float(value) for value in bounds)
    result = base.convert("RGB")
    source_images = [Image.open(path).convert("RGB") for path in sources]
    rng = np.random.default_rng(int(seed))
    for index, (world_x, world_z) in enumerate(points):
        px = round((float(world_x) - min_x) / (max_x - min_x) * (width - 1))
        py = round((max_z - float(world_z)) / (max_z - min_z) * (height - 1))
        diameter = int(rng.integers(8, 15))
        source = source_images[index % len(source_images)]
        patch = source.resize((diameter, diameter), Image.Resampling.BILINEAR)
        if bool(rng.integers(0, 2)):
            patch = patch.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
        yy, xx = np.ogrid[:diameter, :diameter]
        radius = max(1.0, diameter * 0.5)
        radial = np.clip(1.0 - np.sqrt((xx - radius + .5) ** 2 + (yy - radius + .5) ** 2) / radius, 0.0, 1.0)
        mask = Image.fromarray(np.uint8(radial * 255.0 * float(opacity)))
        result.paste(patch, (px - diameter // 2, py - diameter // 2), mask)
    for image in source_images:
        image.close()
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description="Apply deterministic hybrid grass detail to the active terrain texture.")
    parser.add_argument("--config", required=True)
    args = parser.parse_args()
    config_path = Path(args.config).resolve()
    repo = config_path.parents[3]
    config = json.loads(config_path.read_text(encoding="utf-8"))
    spec = config["terrain_ground_cover"]

    base = repo / spec["base_source"]
    if sha256(base) != spec["base_source_sha256"]:
        raise RuntimeError(f"Ground-cover base source hash mismatch: {base}")
    sources = [repo / path for path in spec["texture_sources"]]
    for source in sources:
        expected = spec["source_sha256"].get(source.name)
        if not source.exists() or sha256(source) != expected:
            raise RuntimeError(f"Ground-cover detail source hash mismatch: {source}")

    output = repo / spec["output"]
    texture_size = int(config["materials"]["terrain_texture_size"])
    terrain_base = load_terrain_base(base, texture_size)
    if spec["mode"] == "semantic_texture_points":
        placements_path = repo / config["generated_dir"] / "placements.json"
        placements = json.loads(placements_path.read_text(encoding="utf-8"))
        if placements.get("authority") != "canonical_svg":
            raise RuntimeError("Semantic grass requires canonical_svg placements authority")
        layout_path = repo / config["semantic_layout_config"]
        layout = json.loads(layout_path.read_text(encoding="utf-8"))
        points = placements.get("grass_texture_points_xz", [])
        if not points:
            raise RuntimeError("Canonical SVG emitted no grass texture points")
        result = apply_semantic_grass_points(
            terrain_base, sources, points, layout["world_bounds_xz"],
            int(config["materials"]["seed"]), float(spec["texture_opacity"]),
        )
    else:
        result = apply_ground_cover_detail(
            terrain_base, sources, int(config["materials"]["seed"]),
            config["track_id"], float(spec["texture_coverage"]), float(spec["texture_opacity"]),
        )
    result = make_seamless_edges(result)
    output.parent.mkdir(parents=True, exist_ok=True)
    save_image_atomic(result, output)

    forge_manifest = output.parents[4] / "texture_forge_manifest.json"
    forge = json.loads(forge_manifest.read_text(encoding="utf-8"))
    key = output.relative_to(forge_manifest.parent).as_posix()
    entry = forge["entries"][key]
    entry["sha256"] = sha256(output)
    entry["ground_cover"] = {
        "mode": spec["mode"],
        "card_share": float(spec["card_share"]),
        "texture_share": float(spec["texture_share"]),
        "texture_coverage": float(spec["texture_coverage"]),
        "texture_opacity": float(spec["texture_opacity"]),
        "base_source": {"path": spec["base_source"], "sha256": sha256(base)},
        "sources": [{"path": path.relative_to(repo).as_posix(), "sha256": sha256(path)} for path in sources],
        "semantic_point_count": len(points) if spec["mode"] == "semantic_texture_points" else 0,
    }
    write_json_atomic(forge_manifest, forge)
    print(json.dumps({"terrain": str(output), "sha256": sha256(output), "coverage": spec["texture_coverage"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
