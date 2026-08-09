from __future__ import annotations

import argparse
from pathlib import Path
import hashlib
import json

from pipeline_common import read_json
from texture_forge import FORGE_VERSION, STYLE_ID


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate deterministic Texture Forge outputs and provenance hashes.")
    parser.add_argument("--config", required=True)
    ns = parser.parse_args()
    cp = Path(ns.config).resolve()
    repo = cp.parents[3]
    config = read_json(cp)
    root = repo / config["generated_dir"] / "textures"
    active_path = root / "active_manifest.json"
    forge_path = root / "texture_forge_manifest.json"
    if not active_path.exists() or not forge_path.exists():
        print("FAIL missing Texture Forge manifests")
        return 2

    active = json.loads(active_path.read_text(encoding="utf-8"))
    forge = json.loads(forge_path.read_text(encoding="utf-8"))
    failures = 0
    if active.get("forge", {}).get("style") != STYLE_ID or int(active.get("forge", {}).get("version", -1)) != FORGE_VERSION:
        print(f"FAIL active forge metadata {active.get('forge')}")
        failures += 1
    if forge.get("style") != STYLE_ID or int(forge.get("forge_version", -1)) != FORGE_VERSION:
        print(f"FAIL provenance forge metadata style={forge.get('style')} version={forge.get('forge_version')}")
        failures += 1

    checked = 0
    for rel, entry in forge.get("entries", {}).items():
        path = root / rel
        if not path.exists():
            print(f"FAIL missing texture {rel}")
            failures += 1
            continue
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != entry.get("sha256"):
            print(f"FAIL hash {rel}")
            failures += 1
        checked += 1

    expected_assets = 9 * (4 + 4 + 4 + 4)
    asset_entries = sum(1 for data in forge.get("entries", {}).values() if data.get("category") in {"trees", "bushes", "grass", "fake_buildings"})
    if asset_entries != expected_assets:
        print(f"FAIL texture bank assets={asset_entries} expected={expected_assets}")
        failures += 1
    else:
        print(f"PASS texture bank assets={asset_entries}")

    print(f"Texture Forge checked={checked} style={STYLE_ID} version={FORGE_VERSION}")
    return 0 if failures == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
