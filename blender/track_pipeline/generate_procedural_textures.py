from __future__ import annotations

import argparse
from pathlib import Path
import hashlib
import math

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

from pipeline_common import read_json


def seed_for(base_seed: int, name: str) -> int:
    digest = hashlib.sha256(f"{base_seed}:{name}".encode("utf-8")).digest()
    return int.from_bytes(digest[:8], "little", signed=False)


def rng_for(base_seed: int, name: str) -> np.random.Generator:
    return np.random.default_rng(seed_for(base_seed, name))


def save_rgb_noise(path: Path, size: int, base_rgb, strength: float, seed: int, low_frequency: bool = True) -> None:
    rng = rng_for(seed, path.stem)
    base = np.asarray(base_rgb, dtype=np.float32)
    noise = rng.normal(0.0, strength, (size, size, 1)).astype(np.float32)
    if low_frequency:
        small = rng.normal(0.0, strength * 1.8, (max(4, size // 16), max(4, size // 16), 1)).astype(np.float32)
        small_img = Image.fromarray(np.clip((small[..., 0] + 0.5) * 255.0, 0, 255).astype(np.uint8), "L")
        low = np.asarray(small_img.resize((size, size), Image.Resampling.BICUBIC), dtype=np.float32) / 255.0 - 0.5
        noise += low[..., None] * strength * 1.4
    rgb = np.clip(base[None, None, :] + noise, 0.0, 1.0)
    Image.fromarray((rgb * 255.0).astype(np.uint8), "RGB").save(path)


def save_asphalt(path: Path, size: int, seed: int) -> None:
    rng = rng_for(seed, "asphalt")
    base = np.full((size, size), 0.185, dtype=np.float32)
    fine = rng.normal(0.0, 0.018, (size, size)).astype(np.float32)
    coarse_small = rng.normal(0.0, 0.035, (max(4, size // 16), max(4, size // 16))).astype(np.float32)
    coarse_img = Image.fromarray(np.clip((coarse_small + 0.5) * 255.0, 0, 255).astype(np.uint8), "L")
    coarse = np.asarray(coarse_img.resize((size, size), Image.Resampling.BICUBIC), dtype=np.float32) / 255.0 - 0.5
    value = np.clip(base + fine + coarse * 0.055, 0.10, 0.29)
    rgb = np.stack([value * 0.98, value, value * 1.02], axis=-1)
    Image.fromarray((rgb * 255.0).astype(np.uint8), "RGB").save(path)


def save_grass_card(path: Path, size: int, seed: int, dry: bool) -> None:
    rng = np.random.default_rng(seed_for(seed, path.stem))
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    center = size // 2
    blade_count = 22 if not dry else 17
    colors = (
        [(78, 112, 51, 235), (100, 128, 55, 235), (128, 139, 68, 225)]
        if not dry
        else [(129, 120, 61, 235), (151, 136, 72, 225), (108, 105, 57, 230)]
    )
    for _ in range(blade_count):
        x0 = int(center + rng.uniform(-size * 0.20, size * 0.20))
        y0 = size - 2
        length = rng.uniform(size * 0.38, size * 0.88)
        x1 = int(x0 + rng.uniform(-size * 0.25, size * 0.25))
        y1 = int(max(1, y0 - length))
        width = int(max(1, round(rng.uniform(1.0, 2.4))))
        draw.line((x0, y0, x1, y1), fill=colors[int(rng.integers(0, len(colors)))], width=width)
    image = image.filter(ImageFilter.GaussianBlur(radius=0.35))
    image.save(path)


def save_bush_card(path: Path, size: int, seed: int, dry: bool) -> None:
    rng = np.random.default_rng(seed_for(seed, path.stem))
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    colors = (
        [(67, 108, 52, 245), (86, 125, 59, 240), (106, 139, 67, 235)]
        if not dry
        else [(101, 111, 56, 242), (126, 119, 63, 238), (145, 126, 69, 232)]
    )
    for _ in range(34):
        r = rng.uniform(size * 0.07, size * 0.17)
        cx = rng.uniform(size * 0.22, size * 0.78)
        cy = rng.uniform(size * 0.35, size * 0.82)
        color = colors[int(rng.integers(0, len(colors)))]
        draw.ellipse((cx-r, cy-r, cx+r, cy+r), fill=color)
    draw.rectangle((size * 0.46, size * 0.72, size * 0.54, size * 0.98), fill=(92, 69, 47, 235))
    image = image.filter(ImageFilter.GaussianBlur(radius=0.45))
    image.save(path)


def save_foliage(path: Path, size: int, seed: int, dry: bool = False) -> None:
    rng = np.random.default_rng(seed_for(seed, path.stem))
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    colors = (
        [(53, 101, 47, 255), (68, 116, 52, 250), (89, 129, 58, 245)]
        if not dry
        else [(83, 105, 49, 250), (111, 117, 55, 245), (137, 126, 61, 238)]
    )
    for _ in range(58):
        r = rng.uniform(size * 0.035, size * 0.095)
        angle = rng.uniform(0.0, math.tau)
        radial = math.sqrt(rng.random()) * size * 0.34
        cx = size * 0.5 + math.cos(angle) * radial
        cy = size * 0.5 + math.sin(angle) * radial * 0.78
        draw.ellipse((cx-r, cy-r, cx+r, cy+r), fill=colors[int(rng.integers(0, len(colors)))])
    image.save(path)


def save_checker(path: Path, size: int) -> None:
    image = Image.new("RGB", (size, size), (255, 255, 255))
    draw = ImageDraw.Draw(image)
    cells = 8
    cell = max(1, size // cells)
    for y in range(cells):
        for x in range(cells):
            color = (18, 18, 18) if (x + y) % 2 == 0 else (238, 238, 238)
            draw.rectangle((x * cell, y * cell, (x + 1) * cell, (y + 1) * cell), fill=color)
    image.save(path)


def save_building_facade(path: Path, size: int, seed: int, warehouse: bool) -> None:
    rng = np.random.default_rng(seed_for(seed, path.stem))
    base = (152, 145, 132, 255) if not warehouse else (128, 133, 130, 255)
    image = Image.new("RGBA", (size, size), base)
    draw = ImageDraw.Draw(image)
    rows = 2 if warehouse else 4
    cols = 5 if warehouse else 6
    margin = size * 0.10
    gap = size * 0.035
    cell_w = (size - 2 * margin - (cols - 1) * gap) / cols
    cell_h = (size * 0.62 - (rows - 1) * gap) / rows
    for row in range(rows):
        for col in range(cols):
            x0 = margin + col * (cell_w + gap)
            y0 = margin + row * (cell_h + gap)
            lit = rng.random() < 0.07
            c = (181, 171, 132, 255) if lit else (58, 70, 76, 255)
            draw.rectangle((x0, y0, x0 + cell_w, y0 + cell_h), fill=c)
    draw.rectangle((0, size * 0.82, size, size), fill=(113, 105, 94, 255))
    image.save(path)


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate deterministic low-resolution textures for the Blender racetrack pipeline.")
    parser.add_argument("--config", required=True)
    parser.add_argument("--seed", type=int, default=1995)
    ns = parser.parse_args()

    config_path = Path(ns.config).resolve()
    repo = config_path.parents[3]
    config = read_json(config_path)
    generated = repo / config["generated_dir"]
    out = generated / "textures"
    out.mkdir(parents=True, exist_ok=True)

    size = int(config.get("materials", {}).get("texture_size", 256))
    card_size = int(config.get("materials", {}).get("card_texture_size", 128))

    save_asphalt(out / "asphalt.png", size, ns.seed)
    save_rgb_noise(out / "dry_ground.png", size, (0.28, 0.30, 0.17), 0.035, ns.seed)
    save_rgb_noise(out / "bark.png", card_size, (0.29, 0.19, 0.105), 0.045, ns.seed)
    save_rgb_noise(out / "guardrail.png", card_size, (0.52, 0.54, 0.55), 0.045, ns.seed, low_frequency=False)
    save_foliage(out / "foliage_green.png", card_size, ns.seed, dry=False)
    save_foliage(out / "foliage_dry.png", card_size, ns.seed, dry=True)
    save_grass_card(out / "grass_green.png", card_size, ns.seed, dry=False)
    save_grass_card(out / "grass_dry.png", card_size, ns.seed, dry=True)
    save_bush_card(out / "bush_green.png", card_size, ns.seed, dry=False)
    save_bush_card(out / "bush_dry.png", card_size, ns.seed, dry=True)
    save_building_facade(out / "building_low.png", card_size, ns.seed, warehouse=False)
    save_building_facade(out / "building_warehouse.png", card_size, ns.seed, warehouse=True)
    save_checker(out / "start_finish.png", card_size)

    print(f"[textures] region={config.get('procedural_environment', {}).get('region', 'south_america')} seed={ns.seed}")
    print(f"[textures] wrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
