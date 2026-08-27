"""Registro de remasters en el banco: renombra original a _backup, coloca el
remaster, limpia .import obsoletos y actualiza bank_manifest.json (sha256 + metadatos).

Mantiene `synthesis.source_file` / `source_sha256` y la substring de provenance que
exigen los tests (source-derived => "derived from original"; recipe => "procedural synthesis").
"""

from __future__ import annotations

import hashlib
import json
import shutil
from pathlib import Path

import numpy as np
import soundfile as sf

DEFAULT_BANK = Path("scratch/audio/v10_vehicle")


def _wav_metrics(path: Path):
    data, sr = sf.read(str(path), always_2d=False)
    if data.ndim > 1:
        data = data.mean(axis=1)
    data = np.asarray(data, dtype=np.float64)
    return sr, float(np.max(np.abs(data))), float(np.mean(data)), data


def update_manifest_entry(bank_dir, out_name, sha, dur, peak, dc, lufs, kind, recipe="impact_remaster_v1"):
    bank_dir = Path(bank_dir)
    manifest_path = bank_dir / "bank_manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    for e in manifest["files"]:
        if e.get("file") == out_name:
            e["duration_s"] = round(dur, 6)
            e["peak"] = round(peak, 6)
            e["dc_offset"] = round(dc, 6)
            if lufs is not None:
                e["loudness_dbfs"] = round(lufs, 2)
            e["sha256"] = sha
            is_recipe = bool(e.get("synthesis", {}).get("recipe"))
            prov = e.get("provenance", "")
            if is_recipe:
                # Entrada procedural (ej. impact_scrape): conservar "procedural synthesis".
                if "procedural synthesis" not in prov.lower():
                    prov = "deterministic procedural synthesis; " + prov
                else:
                    prov = prov + "; remastered with modular synthesis (impact stages)"
            else:
                if "derived from original" not in prov.lower():
                    prov = "derived from original samples (source-assets/audio/legacy-f1-1998); " + prov
                else:
                    prov = "derived from original samples (source-assets/audio/legacy-f1-1998); remastered with modular synthesis (impact stages)"
            e["provenance"] = prov
            e.setdefault("synthesis", {})["remaster"] = {"recipe": recipe, "kind": kind}
            break
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")


def apply_remaster(bank_dir, out_name, build_fn, kind, recipe="impact_remaster_v1"):
    bank_dir = Path(bank_dir)
    src = bank_dir / out_name
    backup = bank_dir / (out_name[:-4] + "_backup.wav")
    tmp = bank_dir.parent / f".remaster_{out_name}"

    build_fn(str(src), str(tmp))

    if backup.exists():
        backup.unlink()
    src.replace(backup)
    shutil.move(str(tmp), str(bank_dir / out_name))

    # .import obsoleto: el loader Rust no lo usa, pero lo limpiamos para el editor.
    imp = bank_dir / (out_name + ".import")
    if imp.exists():
        imp.unlink()

    sr, peak, dc, data = _wav_metrics(bank_dir / out_name)
    dur = len(data) / sr
    try:
        import pyloudnorm as pyln

        lufs = float(pyln.Meter(sr).integrated_loudness(data))
    except Exception:  # noqa: BLE001
        lufs = None
    sha = hashlib.sha256((bank_dir / out_name).read_bytes()).hexdigest()
    update_manifest_entry(bank_dir, out_name, sha, dur, peak, dc, lufs, kind, recipe)
    print(f"[ok] {out_name}: sr={sr} dur={dur:.4f}s peak={peak:.3f} dc={dc:+.5f} "
          f"lufs={None if lufs is None else round(lufs, 2)}")
    print(f"      original -> {backup.name}")
