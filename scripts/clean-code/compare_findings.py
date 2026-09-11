#!/usr/bin/env python
"""Compara dos informes static-analysis-summary.json.

Usa una clave estable (tool, regla, archivo, mensaje) para que los
desplazamientos de linea no generen falsos resueltos/nuevos; los cambios de
linea se reportan aparte como "moved".
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path


def load_summary(path: Path) -> dict:
    raw = path.read_bytes()
    if raw.startswith(b"\xef\xbb\xbf"):
        raw = raw[3:]
    return json.loads(raw.decode("utf-8"))


def stable_key(finding: dict) -> tuple:
    return (
        finding.get("tool"),
        finding.get("rule"),
        finding.get("file"),
        finding.get("message"),
    )


def group(findings: list) -> dict:
    grouped = {}
    for finding in findings:
        grouped.setdefault(stable_key(finding), []).append(finding)
    return grouped


def brief(finding: dict) -> dict:
    return {
        "tool": finding.get("tool"),
        "rule": finding.get("rule"),
        "category": finding.get("category"),
        "severity": finding.get("severity"),
        "file": finding.get("file"),
        "line": finding.get("line"),
        "message": finding.get("message"),
        "fingerprint": finding.get("fingerprint"),
        "status": finding.get("status", "open"),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--previous", required=True)
    parser.add_argument("--current", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    previous = load_summary(Path(args.previous))
    current = load_summary(Path(args.current))
    prev_groups = group(previous.get("findings", []))
    curr_groups = group(current.get("findings", []))

    resolved = []
    added = []
    pending = []
    moved = []
    status_changes = []

    for key in prev_groups.keys() - curr_groups.keys():
        resolved.extend(brief(f) for f in prev_groups[key])
    for key in curr_groups.keys() - prev_groups.keys():
        added.extend(brief(f) for f in curr_groups[key])

    for key in prev_groups.keys() & curr_groups.keys():
        p_list = sorted(prev_groups[key], key=lambda f: f.get("line") or 0)
        c_list = sorted(curr_groups[key], key=lambda f: f.get("line") or 0)
        for p_find, c_find in zip(p_list, c_list):
            pending.append(brief(c_find))
            if (p_find.get("line") or 0) != (c_find.get("line") or 0):
                moved.append(
                    {
                        "tool": c_find.get("tool"),
                        "rule": c_find.get("rule"),
                        "file": c_find.get("file"),
                        "message": c_find.get("message"),
                        "from_line": p_find.get("line"),
                        "to_line": c_find.get("line"),
                    }
                )
            if p_find.get("status", "open") != c_find.get("status", "open"):
                status_changes.append(
                    {
                        "fingerprint": c_find.get("fingerprint"),
                        "tool": c_find.get("tool"),
                        "rule": c_find.get("rule"),
                        "file": c_find.get("file"),
                        "from": p_find.get("status", "open"),
                        "to": c_find.get("status", "open"),
                    }
                )
        for extra in p_list[len(c_list):]:
            resolved.append(brief(extra))
        for extra in c_list[len(p_list):]:
            added.append(brief(extra))

    report = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "previous": {
            "path": str(Path(args.previous).resolve()),
            "run_id": previous.get("run_id"),
            "head": previous.get("head"),
            "findings": len(previous.get("findings", [])),
        },
        "current": {
            "path": str(Path(args.current).resolve()),
            "run_id": current.get("run_id"),
            "head": current.get("head"),
            "findings": len(current.get("findings", [])),
        },
        "counts": {
            "resolved": len(resolved),
            "new": len(added),
            "pending": len(pending),
            "moved": len(moved),
            "status_changes": len(status_changes),
        },
        "resolved": resolved,
        "new": added,
        "pending": pending,
        "moved": moved,
        "status_changes": status_changes,
    }

    out = Path(args.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    print(
        f"resolved={len(resolved)} new={len(added)} pending={len(pending)} "
        f"moved={len(moved)} status_changes={len(status_changes)} -> {out}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
