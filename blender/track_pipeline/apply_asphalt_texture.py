from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image

from atomic_json import write_json_atomic
from texture_forge import SurfaceRecipe, forge_surface, make_seamless_edges, recipe_dict, save_image_atomic


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description="Build the curated seamless asphalt texture.")
    parser.add_argument("--config", required=True)
    args = parser.parse_args()
    config_path = Path(args.config).resolve()
    repo = config_path.parents[3]
    config = json.loads(config_path.read_text(encoding="utf-8"))
    materials = config["materials"]

    source = repo / materials["asphalt_source"]
    expected = materials["asphalt_source_sha256"]
    if not source.exists() or sha256(source) != expected:
        raise RuntimeError(f"Asphalt source hash mismatch: {source}")

    size = int(materials["asphalt_texture_size"])
    recipe = SurfaceRecipe(26, .009, 1.04)
    with Image.open(source) as image:
        resized = image.convert("RGB").resize((size, size), Image.Resampling.LANCZOS)
    result = make_seamless_edges(forge_surface(resized, recipe))

    texture_root = repo / config["generated_dir"] / "textures"
    output = texture_root / "shared" / "asphalt.png"
    output.parent.mkdir(parents=True, exist_ok=True)
    save_image_atomic(result, output)

    forge_manifest = texture_root / "texture_forge_manifest.json"
    forge = json.loads(forge_manifest.read_text(encoding="utf-8"))
    entry = forge["entries"]["shared/asphalt.png"]
    entry.update({
        "kind": "surface",
        "recipe": recipe_dict(recipe),
        "sha256": sha256(output),
        "role": "asphalt",
        "seamless": True,
        "source": {"path": materials["asphalt_source"], "sha256": sha256(source)},
    })
    write_json_atomic(forge_manifest, forge)
    print(json.dumps({"asphalt": str(output), "resolution": [size, size], "sha256": sha256(output)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
