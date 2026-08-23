from __future__ import annotations

import argparse
from pathlib import Path
import json
import math
import hashlib

import numpy as np
from PIL import Image, ImageDraw

from pipeline_common import read_json
from procedural_catalog import biome_from_config, palette_for, supported_biomes, specs_for_biome
from texture_forge import (
    CardRecipe,
    SurfaceRecipe,
    STYLE_ID,
    FORGE_VERSION,
    forge_card,
    forge_surface,
    low_frequency_field,
    make_seamless_edges,
    recipe_dict,
    rng_for,
    write_recipe_manifest,
)


def rgb8(color):
    return tuple(int(max(0, min(255, round(float(c) * 255)))) for c in color)


def vary(color, factor, alpha=255):
    c = np.clip(np.asarray(color, dtype=float) * float(factor), 0, 1)
    return (*rgb8(c), int(alpha))


def _save_surface(image: Image.Image, path: Path, recipe: SurfaceRecipe, recipes: dict, metadata: dict):
    path.parent.mkdir(parents=True, exist_ok=True)
    forge_surface(image, recipe).save(path)
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    recipes[str(path)] = {"kind": "surface", "recipe": recipe_dict(recipe), "sha256": digest, **metadata}


def _save_seamless_surface(image: Image.Image, path: Path, recipe: SurfaceRecipe, recipes: dict, metadata: dict):
    path.parent.mkdir(parents=True, exist_ok=True)
    make_seamless_edges(forge_surface(image, recipe)).save(path)
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    recipes[str(path)] = {
        "kind": "surface",
        "recipe": recipe_dict(recipe),
        "sha256": digest,
        "seamless": True,
        **metadata,
    }


def _save_card(image: Image.Image, path: Path, card_size: int, recipe: CardRecipe, recipes: dict, metadata: dict):
    path.parent.mkdir(parents=True, exist_ok=True)
    forge_card(image, card_size, recipe).save(path)
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    recipes[str(path)] = {"kind": "card", "recipe": recipe_dict(recipe), "sha256": digest, **metadata}


def make_asphalt(size: int, seed: int) -> Image.Image:
    rng = rng_for(seed, "asphalt")
    macro = low_frequency_field(rng, size, coarse=11, blur=max(0.8, size / 160.0)) - 0.5
    warm = low_frequency_field(rng, size, coarse=7, blur=max(1.0, size / 120.0)) - 0.5
    fine = rng.normal(0.0, 0.020, (size, size)).astype(np.float32)
    yy, xx = np.mgrid[0:size, 0:size].astype(np.float32)
    patch = np.exp(-(((xx - size * 0.68) / (size * 0.36)) ** 2 + ((yy - size * 0.32) / (size * 0.48)) ** 2))
    value = np.clip(0.19 + fine + macro * 0.050 - patch * 0.016, 0.105, 0.29)
    rgb = np.stack([
        value * (0.985 + warm * 0.022),
        value,
        value * (1.015 - warm * 0.018),
    ], axis=-1)
    return Image.fromarray(np.clip(rgb * 255, 0, 255).astype(np.uint8), "RGB")


def make_terrain(size: int, seed: int, palette: dict, label: str) -> Image.Image:
    rng = rng_for(seed, f"terrain:{label}")
    green = np.asarray(palette["terrain"]["green"], np.float32)
    dry = np.asarray(palette["terrain"]["dry"], np.float32)
    dirt = np.asarray(palette["terrain"]["dirt"], np.float32)
    gm, dm, tm = map(float, palette["terrain"]["mix"])

    f_green = low_frequency_field(rng, size, coarse=7, blur=max(1.0, size / 110.0))
    f_dry = low_frequency_field(rng, size, coarse=6, blur=max(1.0, size / 100.0))
    f_dirt = low_frequency_field(rng, size, coarse=5, blur=max(1.2, size / 90.0))
    yy, xx = np.mgrid[0:size, 0:size].astype(np.float32)

    green_blob_a = np.exp(-(((xx - size * 0.27) / (size * 0.34)) ** 2 + ((yy - size * 0.68) / (size * 0.45)) ** 2) * 1.25)
    green_blob_b = np.exp(-(((xx - size * 0.78) / (size * 0.24)) ** 2 + ((yy - size * 0.73) / (size * 0.28)) ** 2) * 1.70)
    dirt_blob_a = np.exp(-(((xx - size * 0.76) / (size * 0.25)) ** 2 + ((yy - size * 0.23) / (size * 0.31)) ** 2) * 1.55)
    dirt_blob_b = np.exp(-(((xx - size * 0.43) / (size * 0.18)) ** 2 + ((yy - size * 0.36) / (size * 0.16)) ** 2) * 2.10)

    base = green * gm + dry * dm + dirt * tm
    green_mask = np.clip((f_green - 0.47) * 1.28 + green_blob_a * 0.55 + green_blob_b * 0.28, 0, 1)
    dry_mask = np.clip((f_dry - 0.50) * 1.05 + (1.0 - green_blob_a) * 0.10, 0, 0.78)
    dirt_mask = np.clip((f_dirt - 0.62) * 1.58 + dirt_blob_a * 0.62 + dirt_blob_b * 0.35, 0, 0.80)

    rgb = np.broadcast_to(base, (size, size, 3)).copy()
    rgb += (green - base)[None, None, :] * green_mask[..., None] * 0.50
    rgb += (dry - base)[None, None, :] * dry_mask[..., None] * 0.34
    rgb += (dirt - base)[None, None, :] * dirt_mask[..., None] * 0.46

    speckle = rng.random((size, size))
    dry_speck = speckle > 0.993
    dirt_speck = speckle < 0.006
    rgb[dry_speck] = rgb[dry_speck] * 0.75 + dry * 0.25
    rgb[dirt_speck] = rgb[dirt_speck] * 0.60 + dirt * 0.40
    rgb = np.clip(rgb + rng.normal(0.0, 0.010, (size, size, 1)).astype(np.float32), 0, 1)
    return Image.fromarray((rgb * 255).astype(np.uint8), "RGB")


def apply_ground_cover_detail(
    terrain: Image.Image,
    source_paths: list[Path],
    seed: int,
    label: str,
    coverage: float,
    opacity: float,
) -> Image.Image:
    """Blend unmodified ground-cover source pixels into deterministic terrain regions."""
    if not source_paths:
        return terrain.convert("RGB")
    coverage = float(np.clip(coverage, 0.0, 1.0))
    opacity = float(np.clip(opacity, 0.0, 1.0))
    size = terrain.size[0]
    if terrain.size != (size, size):
        raise ValueError("Ground-cover terrain must be square")
    rng = rng_for(seed, f"ground-cover:{label}")
    sources = [np.asarray(Image.open(path).convert("RGB"), dtype=np.float32) / 255.0 for path in source_paths]
    detail = np.zeros((size, size, 3), dtype=np.float32)
    tile = max(32, size // 4)
    for y in range(0, size, tile):
        for x in range(0, size, tile):
            source = sources[int(rng.integers(0, len(sources)))]
            patch = Image.fromarray((source * 255).astype(np.uint8)).resize((tile, tile), Image.Resampling.BILINEAR)
            if bool(rng.integers(0, 2)):
                patch = patch.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
            if bool(rng.integers(0, 2)):
                patch = patch.transpose(Image.Transpose.FLIP_TOP_BOTTOM)
            h = min(tile, size - y)
            w = min(tile, size - x)
            detail[y:y + h, x:x + w] = np.asarray(patch, dtype=np.float32)[:h, :w] / 255.0
    field = low_frequency_field(rng, size, coarse=9, blur=max(1.0, size / 96.0))
    threshold = float(np.quantile(field, 1.0 - coverage)) if 0.0 < coverage < 1.0 else (1.0 if coverage <= 0.0 else 0.0)
    mask = (field >= threshold).astype(np.float32) * opacity
    base = np.asarray(terrain.convert("RGB"), dtype=np.float32) / 255.0
    mixed = base * (1.0 - mask[..., None]) + detail * mask[..., None]
    return Image.fromarray(np.clip(mixed * 255.0, 0, 255).astype(np.uint8))


def make_shoulder(size: int, seed: int, palette: dict, label: str) -> Image.Image:
    rng = rng_for(seed, f"shoulder:{label}")
    dry = np.asarray(palette["terrain"]["dry"], np.float32)
    dirt = np.asarray(palette["terrain"]["dirt"], np.float32)
    field = low_frequency_field(rng, size, coarse=8, blur=max(0.8, size / 150.0))
    mix = np.clip(0.58 + (field - 0.5) * 0.48, 0.22, 0.88)
    rgb = dry[None, None, :] * mix[..., None] + dirt[None, None, :] * (1.0 - mix[..., None])
    grit = rng.normal(0.0, 0.020, (size, size, 1)).astype(np.float32)
    rgb = np.clip(rgb + grit, 0, 1)
    return Image.fromarray((rgb * 255).astype(np.uint8), "RGB")


def make_tree_source(size: int, seed: int, base_color, variant: int) -> Image.Image:
    rng = rng_for(seed, f"tree-source:{variant}:{base_color}")
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    trunk_w = size * (0.050 + 0.008 * (variant % 2))
    trunk_top = size * (0.39 + 0.045 * (variant == 2))
    draw.polygon([
        (size / 2 - trunk_w, size * 0.96),
        (size / 2 + trunk_w, size * 0.96),
        (size / 2 + trunk_w * 0.58, trunk_top),
        (size / 2 - trunk_w * 0.52, trunk_top),
    ], fill=(92, 67, 42, 255))

    forms = {
        0: [(.50,.39,.30,.22),(.32,.49,.22,.18),(.68,.47,.23,.19),(.50,.26,.20,.16)],
        1: [(.50,.39,.34,.20),(.27,.47,.19,.15),(.73,.46,.19,.15),(.50,.25,.17,.13)],
        2: [(.50,.31,.19,.30),(.41,.50,.16,.22),(.60,.47,.17,.22),(.50,.16,.13,.15)],
        3: [(.47,.41,.34,.23),(.24,.48,.18,.15),(.73,.49,.24,.18),(.60,.27,.19,.15)],
    }[variant]
    for idx, (cx, cy, rx, ry) in enumerate(forms):
        j = rng.uniform(0.94, 1.07)
        box = ((cx-rx*j)*size, (cy-ry*j)*size, (cx+rx*j)*size, (cy+ry*j)*size)
        factor = [0.88, 1.00, 1.10, 0.95][idx % 4]
        draw.ellipse(box, fill=vary(base_color, factor))
        x0, y0, x1, y1 = box
        draw.polygon([
            (x0 + (x1-x0)*.16, y0 + (y1-y0)*.20),
            (x0 + (x1-x0)*.55, y0 + (y1-y0)*.08),
            (x0 + (x1-x0)*.44, y0 + (y1-y0)*.43),
        ], fill=vary(base_color, min(1.24, factor + .14)))
    draw.polygon([
        (size*.50,size*.47),(size*.80,size*.44),(size*.71,size*.62),(size*.48,size*.63)
    ], fill=vary(base_color, .74))
    return image


def make_bush_source(size: int, seed: int, base_color, variant: int) -> Image.Image:
    rng = rng_for(seed, f"bush-source:{variant}:{base_color}")
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    draw.rectangle((size*.48,size*.72,size*.52,size*.96), fill=(85,65,42,220))
    count = 16 + variant * 2
    for _ in range(count):
        rx = rng.uniform(size*.085, size*.18)
        ry = rng.uniform(size*.075, size*.15)
        cx = rng.uniform(size*.14, size*.86)
        cy = rng.uniform(size*.42, size*.79)
        lighting = .86 + .30 * (1 - (cx/size)*.62 - (cy/size)*.18)
        draw.ellipse((cx-rx,cy-ry,cx+rx,cy+ry), fill=vary(base_color, lighting))
    draw.polygon([
        (size*.08,size*.76),(size*.20,size*.66),(size*.42,size*.70),(size*.58,size*.64),
        (size*.80,size*.69),(size*.93,size*.78),(size*.82,size*.87),(size*.15,size*.87)
    ], fill=vary(base_color,.80))
    return image


def make_grass_source(size: int, seed: int, base_color, variant: int) -> Image.Image:
    rng = rng_for(seed, f"grass-source:{variant}:{base_color}")
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    blades = 22 + variant * 4
    for _ in range(blades):
        x0 = size*.5 + rng.uniform(-size*.34, size*.34)
        y0 = size*.96
        length = rng.uniform(size*.34, size*(.72+.035*variant))
        lean = rng.uniform(-size*.26, size*.26)
        width = max(1, int(round(rng.uniform(1.0, 2.2) * size / 128.0)))
        draw.line((x0,y0,x0+lean,y0-length), fill=vary(base_color,rng.uniform(.76,1.17),235), width=width)
    draw.ellipse((size*.24,size*.88,size*.76,size*.99), fill=vary(base_color,.60,110))
    return image


def make_building_facade(size: int, seed: int, base_color, variant: int, biome) -> Image.Image:
    rng = rng_for(seed, f"building:{biome.id}:{variant}")
    image = Image.new("RGBA", (size, size), (*rgb8(base_color), 255))
    draw = ImageDraw.Draw(image)
    industrial = variant in (1, 3)
    rows = 2 if industrial else 4
    cols = (5 + variant) if industrial else (4 + variant % 2)
    margin = size*.08
    bottom = size*.84
    cell_w = (size - 2*margin) / (cols*1.32)
    xgap = (size - 2*margin - cols*cell_w) / max(1, cols-1)
    cell_h = (bottom-margin) / (rows*1.55)
    ygap = (bottom-margin-rows*cell_h) / max(1, rows)
    cornice = .075 if biome.altitude == "high" else (.040 if biome.longitude == "east" else .055)
    cornice_factor = .63 if biome.altitude == "high" else (.82 if biome.longitude == "east" else .68)
    draw.rectangle((0,0,size,size*cornice), fill=vary(base_color,cornice_factor))
    draw.rectangle((size*.72,0,size,size), fill=vary(base_color,.88,78))
    for r in range(rows):
        for c in range(cols):
            x0 = margin + c*(cell_w+xgap)
            y0 = margin + r*(cell_h+ygap)
            window = (48,58,61,255) if not industrial else (58,65,65,255)
            if rng.random() < .06:
                window = (169,153,105,255)
            draw.rectangle((x0-2,y0-2,x0+cell_w+2,y0+cell_h+2), fill=(35,35,33,150))
            draw.rectangle((x0,y0,x0+cell_w,y0+cell_h), fill=window)
    if industrial:
        door_w = size*(.22 if variant == 1 else .28)
        draw.rectangle((size*.5-door_w/2,size*.60,size*.5+door_w/2,size*.96), fill=(82,79,72,255))
        for y in np.linspace(size*.64,size*.91,4):
            draw.line((size*.5-door_w/2,y,size*.5+door_w/2,y), fill=(52,52,49,255), width=1)
    else:
        draw.rectangle((size*.44,size*.67,size*.56,size*.96), fill=(75,66,56,255))
    draw.rectangle((0,size*.90,size,size), fill=vary(base_color,.70))
    return image


def make_simple_noise(size: int, seed: int, label: str, base_rgb, strength: float) -> Image.Image:
    rng = rng_for(seed, label)
    base = np.asarray(base_rgb, np.float32)
    macro = low_frequency_field(rng, size, coarse=8, blur=max(.6, size/180.0)) - .5
    fine = rng.normal(0,strength,(size,size,1)).astype(np.float32)
    rgb = np.clip(base[None,None,:]+macro[...,None]*strength*1.3+fine,0,1)
    return Image.fromarray((rgb*255).astype(np.uint8),"RGB")


def make_checker(size: int) -> Image.Image:
    image = Image.new("RGB",(size,size),(238,238,238))
    draw = ImageDraw.Draw(image)
    cell=max(1,size//8)
    for y in range(8):
        for x in range(8):
            draw.rectangle((x*cell,y*cell,(x+1)*cell,(y+1)*cell),fill=(18,18,18) if (x+y)%2==0 else (238,238,238))
    return image


def generate_biome_bank(root: Path, biome, card_size: int, terrain_size: int, seed: int, recipes: dict, ground_cover: dict | None = None):
    palette = palette_for(biome)
    d = root / "biomes" / biome.continent / biome.longitude / biome.altitude
    d.mkdir(parents=True, exist_ok=True)

    surface_recipe = SurfaceRecipe(posterize_levels=28, dither_strength=.010, contrast=1.035)
    terrain_path = d / "terrain.png"
    terrain_image = make_terrain(terrain_size, seed, palette, biome.id)
    terrain_metadata = {"biome": biome.id, "role": "terrain"}
    if ground_cover and ground_cover.get("biome") == biome.id:
        source_paths = [Path(path) for path in ground_cover["source_paths"]]
        terrain_image = apply_ground_cover_detail(
            terrain_image,
            source_paths,
            seed,
            biome.id,
            float(ground_cover.get("texture_coverage", 0.5)),
            float(ground_cover.get("texture_opacity", 0.35)),
        )
        terrain_metadata["ground_cover"] = {
            "card_share": float(ground_cover.get("card_share", 0.5)),
            "texture_share": float(ground_cover.get("texture_share", 0.5)),
            "texture_coverage": float(ground_cover.get("texture_coverage", 0.5)),
            "texture_opacity": float(ground_cover.get("texture_opacity", 0.35)),
            "sources": [{"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()} for path in source_paths],
        }
    _save_surface(terrain_image, terrain_path, surface_recipe, recipes, terrain_metadata)
    shoulder_path = d / "shoulder.png"
    _save_surface(make_shoulder(max(256,terrain_size//2), seed, palette, biome.id), shoulder_path, SurfaceRecipe(24,.012,1.045), recipes, {"biome":biome.id,"role":"roadside_shoulder"})
    bark_path = d / "bark.png"
    _save_surface(make_simple_noise(card_size,seed,f"bark:{biome.id}",(.29,.19,.105),.028), bark_path, SurfaceRecipe(18,.010,1.05), recipes, {"biome":biome.id,"role":"bark"})

    assets = {}
    source_size = max(256, card_size*2)
    tree_recipe = CardRecipe(18,.018,40,64,.17,.22,.075,.10)
    bush_recipe = CardRecipe(16,.020,38,64,.15,.18,.065,.08)
    grass_recipe = CardRecipe(14,.022,34,64,.10,.10,.045,.06)
    building_recipe = SurfaceRecipe(22,.012,1.06)

    for category in ("trees","bushes","grass","fake_buildings"):
        key = {"trees":"tree","bushes":"bush","grass":"grass","fake_buildings":"structures"}[category]
        colors = palette[key]
        for spec in specs_for_biome(biome, category):
            path = d / f"{spec.id}.png"
            color = colors[spec.variant_index % 4]
            metadata = {"biome": biome.id, "category": category, "variant": spec.variant_index}
            if category == "trees":
                _save_card(make_tree_source(source_size,seed,color,spec.variant_index),path,card_size,tree_recipe,recipes,metadata)
            elif category == "bushes":
                _save_card(make_bush_source(source_size,seed,color,spec.variant_index),path,card_size,bush_recipe,recipes,metadata)
            elif category == "grass":
                _save_card(make_grass_source(source_size,seed,color,spec.variant_index),path,card_size,grass_recipe,recipes,metadata)
            else:
                _save_surface(make_building_facade(card_size,seed,color,spec.variant_index,biome),path,building_recipe,recipes,metadata)
            assets[spec.id] = str(path.relative_to(root)).replace("\\","/")

    return {
        "biome": biome.id,
        "terrain": str(terrain_path.relative_to(root)).replace("\\","/"),
        "shoulder": str(shoulder_path.relative_to(root)).replace("\\","/"),
        "bark": str(bark_path.relative_to(root)).replace("\\","/"),
        "assets": assets,
    }


def main() -> int:
    p = argparse.ArgumentParser(description="Generate deterministic Formula90s PS1-rally texture bank.")
    p.add_argument("--config", required=True)
    p.add_argument("--seed", type=int, default=1995)
    ns = p.parse_args()
    cp = Path(ns.config).resolve()
    repo = cp.parents[3]
    config = read_json(cp)
    out = repo / config["generated_dir"] / "textures"
    shared = out / "shared"
    shared.mkdir(parents=True, exist_ok=True)

    materials_cfg = config.get("materials", {})
    surface_size = int(materials_cfg.get("texture_size",256))
    asphalt_size = int(materials_cfg.get("asphalt_texture_size", surface_size))
    terrain_size = int(materials_cfg.get("terrain_texture_size",512))
    card_size = int(materials_cfg.get("card_texture_size",128))
    recipes: dict[str,dict] = {}

    asphalt_source_path = materials_cfg.get("asphalt_source")
    if asphalt_source_path:
        asphalt_source = repo / asphalt_source_path
        expected = materials_cfg.get("asphalt_source_sha256")
        actual = hashlib.sha256(asphalt_source.read_bytes()).hexdigest()
        if expected and actual != expected:
            raise RuntimeError(f"Asphalt source hash mismatch: {asphalt_source} {actual}")
        with Image.open(asphalt_source) as source:
            asphalt = source.convert("RGB").resize((asphalt_size, asphalt_size), Image.Resampling.LANCZOS)
    else:
        asphalt = make_asphalt(asphalt_size, ns.seed)
    _save_seamless_surface(
        asphalt, shared/"asphalt.png", SurfaceRecipe(26,.009,1.04), recipes,
        {"role":"asphalt", "source": asphalt_source_path or "procedural"},
    )
    _save_surface(make_simple_noise(card_size,ns.seed,"guardrail",(.50,.52,.52),.026),shared/"guardrail.png",SurfaceRecipe(18,.012,1.05),recipes,{"role":"guardrail"})
    _save_surface(make_checker(card_size),shared/"start_finish.png",SurfaceRecipe(8,0.0,1.0),recipes,{"role":"start_finish"})

    active = biome_from_config(config)
    ground_cover_cfg = config.get("terrain_ground_cover")
    ground_cover = None
    if ground_cover_cfg:
        source_paths = [repo / path for path in ground_cover_cfg.get("texture_sources", [])]
        expected_hashes = ground_cover_cfg.get("source_sha256", {})
        for path in source_paths:
            if not path.exists():
                raise RuntimeError(f"Ground-cover source missing: {path}")
            expected = expected_hashes.get(path.name)
            actual = hashlib.sha256(path.read_bytes()).hexdigest()
            if expected and actual != expected:
                raise RuntimeError(f"Ground-cover source hash mismatch: {path.name} {actual}")
        ground_cover = {**ground_cover_cfg, "biome": active.id, "source_paths": [str(path) for path in source_paths]}
    manifests = {
        b.id: generate_biome_bank(
            out, b, card_size, terrain_size, ns.seed, recipes,
            ground_cover if b.id == active.id else None,
        )
        for b in supported_biomes()
    }
    active_manifest = {
        "forge": {"version":FORGE_VERSION,"style":STYLE_ID,"seed":ns.seed},
        "active_biome": active.id,
        "shared": {
            "asphalt":"shared/asphalt.png",
            "guardrail":"shared/guardrail.png",
            "start_finish":"shared/start_finish.png",
        },
        **manifests[active.id],
    }
    (out/"active_manifest.json").write_text(json.dumps(active_manifest,indent=2),encoding="utf-8")
    (out/"bank_manifest.json").write_text(json.dumps({"forge":active_manifest["forge"],"biomes":manifests},indent=2),encoding="utf-8")

    normalized_recipes = {}
    for path, data in recipes.items():
        pth = Path(path)
        try:
            rel = str(pth.resolve().relative_to(out.resolve())).replace("\\","/")
        except Exception:
            rel = str(pth).replace("\\","/")
        normalized_recipes[rel] = data
    write_recipe_manifest(out/"texture_forge_manifest.json",seed=ns.seed,entries=normalized_recipes)

    print(f"[textures] forge={STYLE_ID} v{FORGE_VERSION}")
    print(f"[textures] generated {len(manifests)} South America biome combinations")
    print("[textures] bank: 4 trees + 4 bushes + 4 grass cards + 4 facades per biome")
    print(f"[textures] active={active.id} seed={ns.seed}")
    print(f"[textures] wrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
