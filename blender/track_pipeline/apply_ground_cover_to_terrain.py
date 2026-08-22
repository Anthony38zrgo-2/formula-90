from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image

from generate_procedural_textures import apply_ground_cover_detail


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


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
    result = apply_ground_cover_detail(
        Image.open(base).convert("RGB"), sources, int(config["materials"]["seed"]),
        config["track_id"], float(spec["texture_coverage"]), float(spec["texture_opacity"]),
    )
    output.parent.mkdir(parents=True, exist_ok=True)
    result.save(output)

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
    }
    forge_manifest.write_text(json.dumps(forge, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"terrain": str(output), "sha256": sha256(output), "coverage": spec["texture_coverage"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
