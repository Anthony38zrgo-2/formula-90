from __future__ import annotations

import json
import math
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

REPO_ROOT = Path(__file__).resolve().parents[2]
BARRIERS_DIR = REPO_ROOT / "assets-lowpoly-python" / "track_props" / "barriers"


def ensure_dirs(base: Path) -> tuple[Path, Path]:
    tex_dir = base / "textures"
    mesh_dir = base / "meshes"
    tex_dir.mkdir(parents=True, exist_ok=True)
    mesh_dir.mkdir(parents=True, exist_ok=True)
    return tex_dir, mesh_dir


def create_tire_red_white_textures(tex_dir: Path, source_black_dir: Path):
    """Derive red/white striped tire wall albedo from the base tire wall."""
    black_front = Image.open(source_black_dir / "tirewall_front_albedo_128x256.png").convert("RGBA")
    black_top = Image.open(source_black_dir / "tirewall_top_albedo_128x64.png").convert("RGBA")
    black_end = Image.open(source_black_dir / "tirewall_end_albedo_64x256.png").convert("RGBA")

    # Front: 5 rows (each row is ~51.2px tall). Rows 1 and 3 (0-indexed) receive red/white alternating blocks.
    front_arr = np.array(black_front, dtype=np.float32)
    h, w, _ = front_arr.shape
    row_h = h / 5.0

    for r in [1, 3]:  # rows 1 and 3 are colored
        y_start = int(r * row_h)
        y_end = int((r + 1) * row_h)
        # Block pattern across width (4 blocks: Red, White, Red, White)
        block_w = w / 4.0
        for b in range(4):
            x_start = int(b * block_w)
            x_end = int((b + 1) * block_w)
            is_red = (b % 2 == 0) if (r == 1) else (b % 2 == 1)
            # Tint the existing tire shading
            tire_lum = (front_arr[y_start:y_end, x_start:x_end, :3].mean(axis=2, keepdims=True)) / 255.0
            # Boost brightness for visibility
            lum_boost = np.clip(tire_lum * 2.2, 0.0, 1.0)
            if is_red:
                # 90s racing vibrant red
                color = np.array([210.0, 30.0, 25.0], dtype=np.float32)
            else:
                # 90s racing clean off-white
                color = np.array([230.0, 230.0, 225.0], dtype=np.float32)
            front_arr[y_start:y_end, x_start:x_end, :3] = color * lum_boost

    # Add subtle PS1 noise/quantization
    front_arr = np.clip(front_arr, 0, 255).astype(np.uint8)
    front_img = Image.fromarray(front_arr, "RGBA")
    front_img.save(tex_dir / "front_128x256.png")

    # Top: Tread with matching red/white pattern across width
    top_arr = np.array(black_top, dtype=np.float32)
    th, tw, _ = top_arr.shape
    block_w = tw / 4.0
    for b in range(4):
        x_start = int(b * block_w)
        x_end = int((b + 1) * block_w)
        is_red = (b % 2 == 0)
        lum = (top_arr[:, x_start:x_end, :3].mean(axis=2, keepdims=True)) / 255.0
        lum_boost = np.clip(lum * 2.0, 0.0, 1.0)
        color = np.array([210.0, 30.0, 25.0], dtype=np.float32) if is_red else np.array([230.0, 230.0, 225.0], dtype=np.float32)
        top_arr[:, x_start:x_end, :3] = color * lum_boost
    top_arr = np.clip(top_arr, 0, 255).astype(np.uint8)
    top_img = Image.fromarray(top_arr, "RGBA")
    top_img.save(tex_dir / "top_128x64.png")

    # End
    black_end.save(tex_dir / "end_64x256.png")
    print(f"[asset] Generated tire_red_white textures in {tex_dir}")


def create_concrete_jersey_textures(tex_dir: Path):
    """Generate PS1 low-poly weathered concrete Jersey barrier albedo."""
    # Front: 128x256
    front = Image.new("RGBA", (128, 256), (180, 178, 172, 255))
    draw = ImageDraw.Draw(front)
    
    # Concrete noise and subtle panel seams
    rng = np.random.default_rng(1995)
    noise = rng.normal(0, 12, (256, 128, 3)).astype(np.float32)
    base = np.full((256, 128, 3), [175, 173, 168], dtype=np.float32)
    arr = np.clip(base + noise, 0, 255).astype(np.uint8)
    front = Image.fromarray(arr, "RGB").convert("RGBA")
    draw = ImageDraw.Draw(front)

    # Vertical expansion joints / seams
    draw.line([(0, 0), (0, 255)], fill=(120, 118, 114, 255), width=2)
    draw.line([(127, 0), (127, 255)], fill=(120, 118, 114, 255), width=2)
    draw.line([(64, 0), (64, 255)], fill=(135, 133, 128, 255), width=1)

    # Base bevel and grime/rubber tire scuff marks (bottom 30% of barrier)
    scuff = np.zeros((256, 128, 4), dtype=np.uint8)
    for y in range(170, 256):
        intensity = (y - 170) / 86.0
        darkness = int(80 * intensity)
        # horizontal rubber scrape bands
        for x in range(128):
            if (x * 7 + y * 13) % 11 < 5:
                scuff[y, x] = [40, 40, 42, int(180 * intensity)]
    
    scuff_img = Image.fromarray(scuff, "RGBA").filter(ImageFilter.GaussianBlur(1.5))
    front.alpha_composite(scuff_img)

    # Top chamfer highlight
    draw = ImageDraw.Draw(front)
    draw.rectangle([(0, 0), (128, 8)], fill=(215, 213, 208, 160))
    draw.line([(0, 8), (128, 8)], fill=(130, 128, 124, 180), width=1)
    
    front.save(tex_dir / "front_128x256.png")

    # Top: 128x64 concrete top surface
    top_base = np.full((64, 128, 3), [195, 193, 188], dtype=np.float32)
    top_noise = rng.normal(0, 10, (64, 128, 3)).astype(np.float32)
    top_arr = np.clip(top_base + top_noise, 0, 255).astype(np.uint8)
    top = Image.fromarray(top_arr, "RGB").convert("RGBA")
    top_draw = ImageDraw.Draw(top)
    top_draw.line([(0, 0), (0, 64)], fill=(140, 138, 134, 255), width=2)
    top_draw.line([(127, 0), (127, 64)], fill=(140, 138, 134, 255), width=2)
    top.save(tex_dir / "top_128x64.png")

    # End: 64x256 cross-section of Jersey barrier
    end = Image.new("RGBA", (64, 256), (160, 158, 153, 255))
    end_draw = ImageDraw.Draw(end)
    end_draw.rectangle([(0, 0), (63, 255)], fill=(155, 153, 148, 255))
    end_draw.line([(0, 0), (63, 0)], fill=(200, 198, 193, 255), width=3)
    end_draw.line([(0, 0), (0, 255)], fill=(110, 108, 104, 255), width=2)
    end_draw.line([(63, 0), (63, 255)], fill=(110, 108, 104, 255), width=2)
    end.save(tex_dir / "end_64x256.png")
    print(f"[asset] Generated concrete_jersey textures in {tex_dir}")


def create_guardrail_armco_textures(tex_dir: Path):
    """Generate PS1 low-poly Armco corrugated steel guardrail textures."""
    # Front: 128x128 corrugated steel beam
    front = Image.new("RGBA", (128, 128), (170, 175, 180, 255))
    draw = ImageDraw.Draw(front)
    
    # 2 W-shaped corrugated valleys and ridges
    # Top ridge, valley 1, middle ridge, valley 2, bottom ridge
    for x in range(128):
        for y in range(128):
            # Sinusoidal profile of W-beam
            profile = math.sin(y / 128.0 * math.pi * 4.0 - math.pi * 0.5)
            # Specular metallic shading
            shade = 175 + int(profile * 45)
            front.putpixel((x, y), (shade, shade + 2, shade + 5, 255))

    # Add bolt details at post locations (x=16 and x=112)
    draw = ImageDraw.Draw(front)
    for bx in [16, 112]:
        for by in [32, 96]:
            draw.ellipse([(bx - 3, by - 3), (bx + 3, by + 3)], fill=(70, 72, 75, 255), outline=(220, 225, 230, 255))
    
    # Post shadow behind beam
    front.save(tex_dir / "front_128x128.png")

    # Runtime billboard: transparent negative space with painted rails/posts.
    # This intentionally reads as Armco from the driving camera without using
    # opaque rectangular geometry.
    card = Image.new("RGBA", (256, 128), (0, 0, 0, 0))
    card_draw = ImageDraw.Draw(card)
    for px in (18, 128, 238):
        card_draw.rectangle([(px - 4, 18), (px + 4, 127)],
                            fill=(105, 110, 116, 255))
        card_draw.line([(px - 3, 18), (px - 3, 127)],
                       fill=(205, 210, 214, 255), width=2)
    for center_y in (28, 62, 96):
        card_draw.rounded_rectangle([(0, center_y - 10), (255, center_y + 10)],
                                    radius=5, fill=(154, 160, 166, 255),
                                    outline=(74, 79, 84, 255), width=2)
        card_draw.line([(2, center_y - 5), (253, center_y - 5)],
                       fill=(225, 229, 232, 255), width=3)
        card_draw.line([(2, center_y + 6), (253, center_y + 6)],
                       fill=(91, 96, 101, 255), width=3)
        for bolt_x in (18, 128, 238):
            card_draw.ellipse([(bolt_x - 2, center_y - 2),
                               (bolt_x + 2, center_y + 2)],
                              fill=(45, 48, 51, 255))
    card.save(tex_dir / "front_card_rgba_256x128.png")

    # Top: 128x64 top lip of beam
    top = Image.new("RGBA", (128, 64), (195, 198, 202, 255))
    top_draw = ImageDraw.Draw(top)
    top_draw.line([(0, 0), (128, 0)], fill=(230, 235, 240, 255), width=2)
    top_draw.line([(0, 63), (128, 63)], fill=(120, 123, 128, 255), width=2)
    top.save(tex_dir / "top_128x64.png")

    # Post: 64x128 I-beam / C-post support
    post = Image.new("RGBA", (64, 128), (130, 133, 138, 255))
    post_draw = ImageDraw.Draw(post)
    post_draw.rectangle([(16, 0), (48, 127)], fill=(155, 158, 163, 255), outline=(100, 102, 105, 255))
    post.save(tex_dir / "post_64x128.png")
    print(f"[asset] Generated guardrail_armco textures in {tex_dir}")


def create_plastic_blocks_textures(tex_dir: Path):
    """Generate PS1 modular plastic barrier blocks (alternating red and white)."""
    # Front: 128x256 (Left half Red, Right half White)
    front = Image.new("RGBA", (128, 256), (220, 35, 30, 255))
    draw = ImageDraw.Draw(front)
    
    # Red block (left 64px)
    draw.rectangle([(0, 0), (63, 255)], fill=(215, 35, 30, 255))
    # Bevel highlights on red block
    draw.line([(2, 2), (61, 2)], fill=(245, 90, 85, 255), width=2)
    draw.line([(2, 2), (2, 253)], fill=(245, 90, 85, 255), width=2)
    draw.line([(61, 2), (61, 253)], fill=(130, 20, 15, 255), width=2)
    
    # White block (right 64px)
    draw.rectangle([(64, 0), (127, 255)], fill=(230, 230, 225, 255))
    # Bevel highlights on white block
    draw.line([(66, 2), (125, 2)], fill=(255, 255, 255, 255), width=2)
    draw.line([(66, 2), (66, 253)], fill=(255, 255, 255, 255), width=2)
    draw.line([(125, 2), (125, 253)], fill=(150, 150, 145, 255), width=2)

    # Interlocking joint between blocks
    draw.line([(63, 0), (63, 255)], fill=(60, 60, 60, 255), width=2)
    draw.line([(0, 0), (0, 255)], fill=(60, 60, 60, 255), width=1)
    draw.line([(127, 0), (127, 255)], fill=(60, 60, 60, 255), width=1)

    # Top cap handles / fill ports
    draw.ellipse([(24, 16), (40, 32)], fill=(170, 25, 20, 255))
    draw.ellipse([(88, 16), (104, 32)], fill=(180, 180, 175, 255))
    
    front.save(tex_dir / "front_128x256.png")

    # Top: 128x64
    top = Image.new("RGBA", (128, 64), (215, 35, 30, 255))
    top_draw = ImageDraw.Draw(top)
    top_draw.rectangle([(0, 0), (63, 63)], fill=(225, 45, 40, 255))
    top_draw.rectangle([(64, 0), (127, 63)], fill=(240, 240, 235, 255))
    top_draw.ellipse([(24, 24), (40, 40)], fill=(160, 20, 15, 255))
    top_draw.ellipse([(88, 24), (104, 40)], fill=(170, 170, 165, 255))
    top.save(tex_dir / "top_128x64.png")

    # End
    end = Image.new("RGBA", (64, 256), (200, 30, 25, 255))
    end_draw = ImageDraw.Draw(end)
    end_draw.line([(0, 0), (63, 0)], fill=(245, 80, 75, 255), width=3)
    end_draw.line([(0, 0), (0, 255)], fill=(120, 15, 10, 255), width=2)
    end.save(tex_dir / "end_64x256.png")
    print(f"[asset] Generated plastic_blocks textures in {tex_dir}")


def main():
    print("[barriers] Creating directory structure and generating 90s textures...")
    
    # 1. Tire Black
    tb_tex, tb_mesh = ensure_dirs(BARRIERS_DIR / "tire_black")
    source_tb = REPO_ROOT / "assets-lowpoly-python" / "track_props" / "tire_barrier" / "textures"
    if source_tb.exists():
        import shutil
        for f in source_tb.glob("*.png"):
            shutil.copyfile(f, tb_tex / f.name)
        if (source_tb.parent / "meshes" / "tirewall_segment_8tris.glb").exists():
            shutil.copyfile(source_tb.parent / "meshes" / "tirewall_segment_8tris.glb", tb_mesh / "tirewall_black_8tris.glb")

    # 2. Tire Red & White
    trw_tex, trw_mesh = ensure_dirs(BARRIERS_DIR / "tire_red_white")
    create_tire_red_white_textures(trw_tex, tb_tex)

    # 3. Concrete Jersey
    cj_tex, cj_mesh = ensure_dirs(BARRIERS_DIR / "concrete_jersey")
    create_concrete_jersey_textures(cj_tex)

    # 4. Guardrail Armco
    ga_tex, ga_mesh = ensure_dirs(BARRIERS_DIR / "guardrail_armco")
    create_guardrail_armco_textures(ga_tex)

    # 5. Plastic Blocks
    pb_tex, pb_mesh = ensure_dirs(BARRIERS_DIR / "plastic_blocks")
    create_plastic_blocks_textures(pb_tex)

    manifest = {
        "schema_version": 1,
        "types": {
            "tire_black": {
                "name": "Standard Black Tire Wall",
                "width_m": 2.0, "height_m": 1.25, "depth_m": 0.45,
                "front": "tire_black/textures/tirewall_front_albedo_128x256.png",
                "top": "tire_black/textures/tirewall_top_albedo_128x64.png",
                "end": "tire_black/textures/tirewall_end_albedo_64x256.png",
                "color_rgb": [32, 56, 100]
            },
            "tire_red_white": {
                "name": "Red & White Striped Tire Wall",
                "width_m": 2.0, "height_m": 1.25, "depth_m": 0.45,
                "front": "tire_red_white/textures/front_128x256.png",
                "top": "tire_red_white/textures/top_128x64.png",
                "end": "tire_red_white/textures/end_64x256.png",
                "color_rgb": [180, 30, 30]
            },
            "concrete_jersey": {
                "name": "Weathered Concrete Jersey Barrier",
                "width_m": 2.0, "height_m": 1.0, "depth_m": 0.40,
                "front": "concrete_jersey/textures/front_128x256.png",
                "top": "concrete_jersey/textures/top_128x64.png",
                "end": "concrete_jersey/textures/end_64x256.png",
                "color_rgb": [210, 210, 210]
            },
            "guardrail_armco": {
                "name": "Armco Corrugated Steel Guardrail",
                "width_m": 2.0, "height_m": 0.82, "depth_m": 0.15,
                "front": "guardrail_armco/textures/front_128x128.png",
                "top": "guardrail_armco/textures/top_128x64.png",
                "end": "guardrail_armco/textures/post_64x128.png",
                "color_rgb": [140, 145, 150]
            },
            "plastic_blocks": {
                "name": "Modular Plastic Water Barriers",
                "width_m": 2.0, "height_m": 0.90, "depth_m": 0.40,
                "front": "plastic_blocks/textures/front_128x256.png",
                "top": "plastic_blocks/textures/top_128x64.png",
                "end": "plastic_blocks/textures/end_64x256.png",
                "color_rgb": [230, 130, 30]
            }
        }
    }
    manifest_path = BARRIERS_DIR / "source_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"[barriers] Wrote {manifest_path}")


if __name__ == "__main__":
    main()
