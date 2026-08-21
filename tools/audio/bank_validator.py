"""Bank validator — single responsibility: validate a generated bank on disk."""

from __future__ import annotations

import hashlib
import json
import struct
import wave
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Finding:
    file: str
    code: str
    level: str
    message: str


def _read_wav(path: Path):
    with wave.open(str(path), "rb") as r:
        params = r.getparams()
        raw = r.readframes(r.getnframes())
    vals = [v[0] / 32767.0 for v in struct.iter_unpack("<h", raw[: len(raw) // 2 * 2])] if raw else []
    return params, vals


def validate_bank(bank_dir: Path, manifest_name: str = "bank_manifest.json") -> list[Finding]:
    findings: list[Finding] = []
    manifest_path = bank_dir / manifest_name
    if not manifest_path.is_file():
        findings.append(Finding(manifest_name, "manifest.missing", "error", "bank_manifest.json not found"))
        return findings
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as e:
        findings.append(Finding(manifest_name, "manifest.invalid_json", "error", str(e)))
        return findings
    files = manifest.get("files", [])
    if not files:
        findings.append(Finding(manifest_name, "manifest.empty", "error", "no files listed"))
    for entry in files:
        fname = entry.get("file", "")
        fpath = bank_dir / fname
        if not fpath.is_file():
            findings.append(Finding(fname, "file.missing", "error", "listed file not found on disk"))
            continue
        actual_sha = hashlib.sha256(fpath.read_bytes()).hexdigest()
        expected_sha = entry.get("sha256", "")
        if expected_sha and actual_sha != expected_sha:
            findings.append(Finding(fname, "hash.mismatch", "error", f"sha256 {actual_sha} != manifest {expected_sha}"))
        try:
            params, vals = _read_wav(fpath)
        except (OSError, EOFError, wave.Error, struct.error) as e:
            findings.append(Finding(fname, "wav.unreadable", "error", str(e)))
            continue
        if params.nchannels != 1:
            findings.append(Finding(fname, "format.channels", "error", f"channels={params.nchannels} want 1"))
        if params.sampwidth != 2:
            findings.append(Finding(fname, "format.bits", "error", f"sampwidth={params.sampwidth} want 2"))
        if params.framerate not in (44100, 48000):
            findings.append(Finding(fname, "format.rate", "error", f"rate={params.framerate} want 44100"))
        elif params.framerate != 44100:
            findings.append(Finding(fname, "format.rate_warning", "warning", f"rate {params.framerate} accepted but bank contract is 44100"))
        is_loop = bool(entry.get("loop"))
        if vals:
            pk = max(abs(v) for v in vals)
            mean = sum(vals) / len(vals)
            if pk >= 0.99:
                findings.append(Finding(fname, "peak.clipping", "error", f"peak {pk:.4f} >= 0.99 (clipping)"))
            if abs(mean) > 0.005:
                findings.append(Finding(fname, "dc.offset", "error", f"DC {mean:.5f} > 0.005"))
            if is_loop:
                # Loop continuity: exact seam step at the wrap (sample[-1] -> sample[0]).
                thresh = 0.05
                disc = abs(vals[-1] - vals[0])
                if disc > thresh:
                    findings.append(Finding(fname, "loop.discontinuity", "error", f"loop seam step {disc:.3f} > {thresh}"))
    on_disk = {p.name for p in bank_dir.glob("*.wav")}
    manifest_files = {f.get("file") for f in files}
    for extra in sorted(on_disk - manifest_files):
        findings.append(Finding(extra, "file.untracked", "warning", "wav on disk not listed in manifest"))
    for missing in sorted(manifest_files - on_disk):
        findings.append(Finding(missing, "file.manifest_only", "error", "manifest entry has no file on disk"))
    for entry in files:
        prov = entry.get("provenance", "")
        synth = entry.get("synthesis", {})
        if synth.get("recipe"):
            if not isinstance(synth.get("seed"), int):
                findings.append(Finding(entry.get("file", ""), "manifest.synthesis_seed_missing", "error", "procedural entry has no integer seed"))
            continue
        if prov and "derived from original" not in prov.lower():
            findings.append(Finding(entry.get("file", ""), "provenance", "warning", f"provenance '{prov}' does not declare derived-from-original"))
        # Sample-derived entries must reference their source and its content hash.
        if not synth.get("source_file"):
            findings.append(Finding(entry.get("file", ""), "manifest.source_missing", "error", "entry has no source_file in synthesis metadata"))
        if not synth.get("source_sha256"):
            findings.append(Finding(entry.get("file", ""), "manifest.source_hash_missing", "error", "entry has no source_sha256 in synthesis metadata"))
    return findings


def main() -> int:
    import argparse
    ap = argparse.ArgumentParser(description="Validate v10_vehicle sound bank")
    ap.add_argument("--bank", type=Path, default=Path("game/sounds/banks/v10_vehicle"))
    args = ap.parse_args()
    findings = validate_bank(args.bank)
    if not findings:
        print("Bank OK - no findings")
        return 0
    for f in findings:
        print(f"[{f.level.upper()}] {f.file}: {f.code} - {f.message}")
    errs = sum(1 for f in findings if f.level == "error")
    return 1 if errs else 0


if __name__ == "__main__":
    raise SystemExit(main())
