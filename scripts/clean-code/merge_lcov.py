#!/usr/bin/env python
"""Normaliza el LCOV del gate cargo-crap (CLEAN-12).

`cargo llvm-cov --workspace` (lib + bins + integracion) carga dos/mas
instanciaciones del mismo modulo en crates ["rlib","cdylib"]: la del rlib con
`#[cfg(test)]` (con hits) y la del rlib no-test que enlazan bins/integracion
(0 hits). El escritor LCOV de llvm-cov emite ENTRADAS DUPLICADAS por funcion y
por linea dentro del mismo bloque SF (una por instanciacion), y para funciones
`#[no_mangle]` el simbolo identico colapsa el contador a 0 aunque los unit
tests las ejecuten.

Este normalizador:
  1. Deduplica FN/FNDA/DA/BRDA por clave (nombre demangled / linea / branch)
     conservando el count maximo, de modo que el LCOV reporte una sola
     instanciacion por funcion y linea.
  2. Recalcula FNF/FNH y LF/LH coherentes con las entradas deduplicadas.
  3. Si se pasa `--abi <lcov>`, remapea funciones `#[no_mangle]` con FNDA:0
     (y su cobertura de linea) desde el lcov de una pasada `--lib`, que
     registra los contadores reales de los unit tests del ABI.
"""
from __future__ import annotations

import argparse
import re
from pathlib import Path

HASH_RE = re.compile(r"(_RN[^_]*?Cs)[^_]+(_.*)")

NO_MANGLE_PREFIXES = ("f90_core_", "sim_world_", "f1_94_", "vehicle_audio_", "psx_art_")


def demangled_key(name: str) -> str:
    m = HASH_RE.match(name)
    if m:
        return m.group(1) + m.group(2)
    return name


def is_unmangled(name: str) -> bool:
    return name.startswith(NO_MANGLE_PREFIXES)


def parse_counts(path: Path) -> tuple[dict[str, int], dict[int, int]]:
    """(name->fnda, line->da) para remapear no_mangle desde la pasada --lib."""
    fnda: dict[str, int] = {}
    da: dict[int, int] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        m = re.match(r"^FNDA:(\d+),(.+)$", line)
        if m:
            fnda[m.group(2)] = max(fnda.get(m.group(2), 0), int(m.group(1)))
        m = re.match(r"^DA:(\d+),(\d+)", line)
        if m:
            da[int(m.group(1))] = max(da.get(int(m.group(1)), 0), int(m.group(2)))
    return fnda, da


def normalize_block(lines: list[str], abi_fnda: dict[str, int], abi_da: dict[int, int]) -> list[str]:
    if not any(l.startswith(("FN:", "FNDA:", "DA:")) for l in lines):
        return lines
    keep: list[str] = []
    fn_best: dict[str, int] = {}
    fnda_best: dict[str, int] = {}
    da_best: dict[int, int] = {}
    brda_best: dict[tuple, int] = {}
    for line in lines:
        if line.startswith("FN:"):
            name = line.split(",", 1)[1]
            keep.append(line)
        elif line.startswith("FNDA:"):
            m = re.match(r"^FNDA:(\d+),(.+)$", line)
            if not m:
                continue
            count, name = int(m.group(1)), m.group(2)
            fnda_best[name] = max(fnda_best.get(name, 0), count)
        elif line.startswith("DA:"):
            m = re.match(r"^DA:(\d+),(\d+)", line)
            if not m:
                continue
            ln = int(m.group(1))
            da_best[ln] = max(da_best.get(ln, 0), int(m.group(2)))
        elif line.startswith("BRDA:"):
            m = re.match(r"^BRDA:(\d+),(\d+),(\d+),(\d+)", line)
            if not m:
                continue
            key = (int(m.group(1)), int(m.group(2)), int(m.group(3)))
            brda_best[key] = max(brda_best.get(key, 0), int(m.group(4)))
        elif line.startswith(("FNF:", "FNH:", "LF:", "LH:", "BRF:", "BRH:")):
            continue  # recalculadas abajo
        elif line.startswith(("end_of_record", "SF:")):
            continue  # se reescriben por el llamador
        else:
            keep.append(line)
    # remap no_mangle desde la pasada --lib
    for name in fnda_best:
        if is_unmangled(name) and abi_fnda.get(name, 0) > 0 and fnda_best.get(name, 0) == 0:
            fnda_best[name] = abi_fnda[name]
    for ln in da_best:
        if abi_da.get(ln, 0) > 0 and da_best[ln] == 0:
            da_best[ln] = abi_da[ln]
    # dedup FN por nombre demangled (una sola instanciacion)
    by_key: dict[str, list[str]] = {}
    for line in keep:
        if line.startswith("FN:"):
            by_key.setdefault(demangled_key(line.split(",", 1)[1]), []).append(line)
    fn_lines: list[str] = []
    seen: dict[str, str] = {}
    for key, variants in by_key.items():
        if len(variants) < 2:
            fn_lines.extend(variants)
            for line in variants:
                seen[demangled_key(line.split(",", 1)[1])] = line.split(",", 1)[1]
            continue
        best = max(variants, key=lambda v: fnda_best.get(v.split(",", 1)[1], 0))
        for line in variants:
            if line == best:
                fn_lines.append(line)
                seen[demangled_key(line.split(",", 1)[1])] = line.split(",", 1)[1]
            else:
                fnda_best.pop(line.split(",", 1)[1], None)
    out = fn_lines + [f"FNDA:{c},{n}" for n, c in sorted(fnda_best.items())]
    out += [f"DA:{ln},{c}" for ln, c in sorted(da_best.items())]
    out += [f"BRDA:{a},{b},{t},{c}" for (a, b, t), c in sorted(brda_best.items())]
    fn_found = len(fn_lines)
    fn_hit = sum(1 for _, name in seen.items() if fnda_best.get(name, 0) > 0)
    line_found = len(da_best)
    line_hit = sum(1 for c in da_best.values() if c > 0)
    br_found = len(brda_best)
    br_hit = sum(1 for c in brda_best.values() if c > 0)
    out += [f"FNF:{fn_found}", f"FNH:{fn_hit}", f"LF:{line_found}", f"LH:{line_hit}"]
    if brda_best:
        out += [f"BRF:{br_found}", f"BRH:{br_hit}"]
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lcov", required=True, help="lcov de la pasada --tests a normalizar")
    parser.add_argument("--abi", help="lcov opcional de la pasada --lib para remapear ABI")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    abi_fnda: dict[str, int] = {}
    abi_da: dict[int, int] = {}
    if args.abi:
        abi_fnda, abi_da = parse_counts(Path(args.abi))

    text = Path(args.lcov).read_text(encoding="utf-8")
    blocks = text.split("end_of_record")
    out_blocks: list[str] = []
    for block in blocks:
        if not block.strip():
            out_blocks.append(block)
            continue
        lines = block.splitlines()
        sf = [l for l in lines if l.startswith("SF:")]
        rest = [l for l in lines if not l.startswith("SF:")]
        norm = normalize_block(rest, abi_fnda, abi_da)
        out_blocks.append("\n".join(sf + norm) + "\n")
    merged = "end_of_record\n".join(out_blocks) + "end_of_record"
    Path(args.output).write_text(merged, encoding="utf-8")

    remapped = sum(1 for name, count in abi_fnda.items() if is_unmangled(name) and count > 0)
    print(f"LCOV normalize: {args.lcov} -> {args.output}"
          + (f" (no_mangle fns con count en --lib: {remapped})" if args.abi else ""))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())