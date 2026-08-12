#!/usr/bin/env python3
"""Create a deterministic flat-color or lightly dithered PNG texture."""

from __future__ import annotations

import argparse
from pathlib import Path
import random
import sys

from PIL import Image, ImageDraw


def _color(value: str) -> tuple[int, int, int, int]:
    text = value.lstrip("#")
    if len(text) not in (6, 8):
        raise ValueError("color must be RRGGBB or RRGGBBAA")
    channels = tuple(int(text[index : index + 2], 16) for index in range(0, len(text), 2))
    return channels if len(channels) == 4 else (*channels, 255)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--size", type=int, nargs=2, default=(256, 256), metavar=("WIDTH", "HEIGHT"))
    parser.add_argument("--color", default="#808080", help="Base RRGGBB or RRGGBBAA color")
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--grid", type=int, default=0, help="Optional grid cell size; 0 creates a flat texture")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    width, height = args.size
    if width < 1 or height < 1 or args.grid < 0:
        print("error: size must be positive and grid must be non-negative", file=sys.stderr)
        return 2
    try:
        base = _color(args.color)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    image = Image.new("RGBA", (width, height), base)
    if args.grid:
        draw = ImageDraw.Draw(image)
        rng = random.Random(args.seed)
        for top in range(0, height, args.grid):
            for left in range(0, width, args.grid):
                jitter = rng.choice((-8, -4, 0, 4, 8))
                color = tuple(max(0, min(255, channel + jitter)) for channel in base[:3]) + (base[3],)
                draw.rectangle((left, top, min(left + args.grid - 1, width - 1), min(top + args.grid - 1, height - 1)), fill=color)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    image.save(args.output, format="PNG", optimize=False)
    print(f"wrote {args.output.resolve()} ({width}x{height}, seed={args.seed})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
