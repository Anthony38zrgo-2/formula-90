#!/usr/bin/env python
"""Normaliza los resultados de los analizadores a un unico informe determinista.

Entradas esperadas en <run-dir>/sarif: clippy.sarif, audit.sarif, deny.sarif,
cargo-crap.sarif, clang-tidy.sarif, cppcheck.sarif, semgrep.sarif.
Entrada adicional en <run-dir>/raw: geiger.json.

Salidas:
  <run-dir>/static-analysis-summary.json
  <run-dir>/combined.sarif
  <repository-root>/static-analysis-summary.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import unquote, urlparse

CATEGORY_PRIORITY = {
    "errors": 1,
    "vulnerabilities": 2,
    "unsafe": 3,
    "warnings": 4,
    "performance": 5,
    "complexity": 6,
    "maintainability": 7,
}

SARIF_TOOLS = {
    "clippy.sarif": "clippy",
    "audit.sarif": "cargo-audit",
    "deny.sarif": "cargo-deny",
    "cargo-crap.sarif": "cargo-crap",
    "clang-tidy.sarif": "clang-tidy",
    "cppcheck.sarif": "cppcheck",
    "semgrep.sarif": "semgrep",
}

CANONICAL_TOOLS = [
    "clippy",
    "cargo-audit",
    "cargo-deny",
    "cargo-geiger",
    "cargo-crap",
    "clang-tidy",
    "cppcheck",
    "semgrep",
]

STATUS_ALIAS = {
    "cargo-audit": "audit",
    "cargo-deny": "deny",
    "cargo-geiger": "geiger",
    "cargo-crap": "crap",
}


def load_json(path: Path):
    raw = path.read_bytes()
    if raw.startswith(b"\xef\xbb\xbf"):
        raw = raw[3:]
    return json.loads(raw.decode("utf-8"))


def normalize_path(uri: str, repo_root: Path) -> str:
    if not uri:
        return ""
    value = unquote(uri)
    if value.startswith("file://"):
        value = urlparse(value).path
    value = value.lstrip("\\\\?\\").replace("\\", "/")
    if re.match(r"^/[a-zA-Z]:/", value):
        value = value[1:]
    if re.match(r"^[a-zA-Z]:/", value):
        value = value[2:].lstrip("/")
    root = repo_root.as_posix().lstrip("\\\\?\\")
    if re.match(r"^[a-zA-Z]:/", root):
        root = root[2:].lstrip("/")
    prefix = root.rstrip("/") + "/"
    low = value.lower()
    if low.startswith(prefix.lower()):
        value = value[len(prefix):]
    return value


def is_out_of_scope(file: str) -> bool:
    """Descarta hallazgos en rutas externas al repositorio (headers de sistema
    de MSVC, SDKs, etc.) que algunos analizadores reportan por transitividad."""
    if not file:
        return False
    value = file.replace("\\", "/")
    if re.match(r"^[a-zA-Z]:/", value):
        return True
    if re.match(r"^/+[a-zA-Z]:", value):
        return True
    if re.search(r"(^|/)Program Files/", value):
        return True
    # Codigo generado por Faust: excluido del analisis por decision de scope.
    if value.startswith("game/native/vehicle-audio-dsp/generated/"):
        return True
    return False


def find_line(region: dict) -> int:
    line = region.get("startLine")
    if isinstance(line, int) and line > 0:
        return line
    return 0


def rule_level(rule: dict) -> str:
    level = (rule.get("defaultConfiguration") or {}).get("level")
    return level or "warning"


def rule_tags(rule: dict) -> list:
    props = rule.get("properties") or {}
    tags = props.get("tags") or []
    return [str(t) for t in tags]


def rule_problem_severity(rule: dict) -> str:
    props = rule.get("properties") or {}
    for key in ("problem.severity", "problemSeverity", "severity"):
        value = props.get(key)
        if isinstance(value, str) and value:
            return value.lower()
    return ""


def classify(tool: str, rule_id: str, level: str, rules: dict) -> tuple:
    """Devuelve (category, severity_efectiva)."""
    lowered = (rule_id or "").lower()
    if tool == "cargo-audit":
        return "vulnerabilities", "error"
    if tool == "cargo-deny":
        if "advisories" in lowered:
            return "vulnerabilities", "error"
        if "unmaintained" in lowered:
            return "maintainability", "warning"
        return "warnings", "warning"
    if tool == "cargo-crap":
        return "complexity", "warning"
    if tool == "clippy":
        return ("errors" if level == "error" else "warnings"), level
    if tool == "clang-tidy":
        if level == "error" or lowered.startswith("clang-diagnostic-error"):
            return "errors", "error"
        if lowered.startswith("performance-"):
            return "performance", "warning"
        if lowered.startswith(("readability-", "modernize-", "llvm-")):
            return "maintainability", "warning"
        if lowered.startswith("clang-analyzer-"):
            return "warnings", "warning"
        return "warnings", "warning"
    if tool == "cppcheck":
        rule = rules.get(rule_id) or {}
        severity = rule_problem_severity(rule)
        props = rule.get("properties") or {}
        try:
            security = float(props.get("security-severity") or 0)
        except (TypeError, ValueError):
            security = 0.0
        if security >= 7.0:
            return "vulnerabilities", "error"
        mapping = {
            "error": ("errors", "error"),
            "warning": ("warnings", "warning"),
            "performance": ("performance", "warning"),
            "portability": ("warnings", "warning"),
            "style": ("maintainability", "warning"),
            "information": ("maintainability", "note"),
            "debug": ("maintainability", "note"),
        }
        return mapping.get(severity, ("warnings", level))
    if tool == "semgrep":
        rule = rules.get(rule_id) or {}
        tags = [t.lower() for t in rule_tags(rule)]
        if "security" in tags:
            return ("vulnerabilities" if level in ("error",) else "unsafe"), "error"
        if level == "error":
            return "errors", "error"
        if level in ("note", "info", "none"):
            return "maintainability", "note"
        return "warnings", "warning"
    return "warnings", level


def build_finding(tool: str, rule_id: str, level: str, message: str, file: str, line: int, category: str, severity: str) -> dict:
    fingerprint = hashlib.sha1(f"{tool}|{rule_id}|{file}|{line}|{message}".encode("utf-8")).hexdigest()[:16]
    return {
        "tool": tool,
        "rule": rule_id or "unknown",
        "severity": severity or level or "warning",
        "category": category,
        "priority": CATEGORY_PRIORITY[category],
        "file": file,
        "line": line,
        "message": message,
        "fingerprint": fingerprint,
        "status": "open",
    }


def parse_sarif(path: Path, tool: str, repo_root: Path) -> list:
    doc = load_json(path)
    findings = []
    for run in doc.get("runs", []) or []:
        rules = {}
        for rule in (run.get("tool", {}).get("driver", {}) or {}).get("rules", []) or []:
            if rule.get("id"):
                rules[rule["id"]] = rule
        for result in run.get("results", []) or []:
            rule_id = result.get("ruleId") or (result.get("rule") or {}).get("id") or "unknown"
            level = result.get("level") or rule_level(rules.get(rule_id) or {})
            message = ((result.get("message") or {}).get("text") or "").strip()
            if not message:
                rule = rules.get(rule_id) or {}
                message = ((rule.get("shortDescription") or {}).get("text") or rule_id).strip()
            file = ""
            line = 0
            locations = result.get("locations") or []
            if locations:
                physical = (locations[0] or {}).get("physicalLocation") or {}
                artifact = physical.get("artifactLocation") or {}
                file = normalize_path(artifact.get("uri") or "", repo_root)
                line = find_line(physical.get("region") or {})
            if is_out_of_scope(file):
                continue
            category, severity = classify(tool, rule_id, level, rules)
            findings.append(build_finding(tool, rule_id, level, message, file, line, category, severity))
    return findings


def parse_geiger(path: Path, repo_root: Path) -> list:
    doc = load_json(path)
    packages = doc.get("packages") or []
    findings = []
    seen = set()
    for entry in packages:
        package = entry.get("package") or {}
        pkg_id = package.get("id") or {}
        name = pkg_id.get("name") or "unknown"
        source = pkg_id.get("source") or {}
        if not isinstance(source, dict) or "Path" not in source:
            continue
        raw_path = unquote(str(source["Path"])).split("#")[0]
        if raw_path.startswith("file://"):
            raw_path = urlparse(raw_path).path
        rel = normalize_path(raw_path, repo_root)
        if not rel or is_out_of_scope(rel):
            continue
        key = (name, rel)
        if key in seen:
            continue
        seen.add(key)
        unsafety = entry.get("unsafety") or {}
        used = unsafety.get("used") or {}
        counters = {}
        total = 0
        for kind, values in used.items():
            count = int((values or {}).get("unsafe_") or 0)
            counters[kind] = count
            total += count
        if total <= 0:
            continue
        manifest = f"{rel.rstrip('/')}/Cargo.toml"
        detail = ", ".join(f"{kind}={count}" for kind, count in sorted(counters.items()) if count)
        message = (
            f"cargo-geiger detecto {total} usos de `unsafe` en el crate '{name}' "
            f"({detail}). Revisar si cada bloque unsafe esta justificado."
        )
        findings.append(
            build_finding("cargo-geiger", f"geiger/{name}", "warning", message, manifest, 1, "unsafe", "warning")
        )
    return findings


def load_suppressions(repo_root: Path) -> dict:
    path = repo_root / "scripts" / "clean-code" / "suppressions.json"
    if not path.exists():
        return {}
    try:
        doc = load_json(path)
    except (json.JSONDecodeError, OSError):
        return {}
    return doc.get("suppressions") or {}


def stable_key(finding: dict) -> str:
    return "|".join(
        [
            str(finding.get("tool", "")),
            str(finding.get("rule", "")),
            str(finding.get("file", "")),
            str(finding.get("message", "")),
        ]
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-dir", required=True)
    parser.add_argument("--repository-root", required=True)
    parser.add_argument("--skip-root-copy", action="store_true")
    args = parser.parse_args()

    run_dir = Path(args.run_dir).resolve()
    repo_root = Path(args.repository_root).resolve()
    sarif_dir = run_dir / "sarif"
    raw_dir = run_dir / "raw"

    findings = []
    merged_runs = []

    for filename, tool in SARIF_TOOLS.items():
        path = sarif_dir / filename
        if not path.exists():
            continue
        try:
            doc = load_json(path)
        except (json.JSONDecodeError, OSError) as exc:
            print(f"[warn] SARIF invalido {filename}: {exc}")
            continue
        merged_runs.extend(doc.get("runs", []) or [])
        findings.extend(parse_sarif(path, tool, repo_root))

    geiger_raw = raw_dir / "geiger.json"
    if geiger_raw.exists():
        try:
            findings.extend(parse_geiger(geiger_raw, repo_root))
        except (json.JSONDecodeError, OSError) as exc:
            print(f"[warn] geiger.json invalido: {exc}")

    deduped = {}
    for finding in findings:
        key = (finding["tool"], finding["rule"], finding["file"], finding["line"], finding["message"])
        if key not in deduped:
            deduped[key] = finding
    findings = sorted(
        deduped.values(),
        key=lambda f: (f["priority"], f["tool"], f["file"], f["line"], f["rule"], f["message"]),
    )

    suppressions = load_suppressions(repo_root)
    suppressed = 0
    for finding in findings:
        entry = suppressions.get(finding["fingerprint"]) or suppressions.get(stable_key(finding))
        if entry:
            finding["status"] = "suppressed"
            finding["suppression_reason"] = entry.get("reason", "")
            suppressed += 1

    tool_status = {}
    status_path = run_dir / "tool-status.json"
    if status_path.exists():
        try:
            tool_status = load_json(status_path)
        except (json.JSONDecodeError, OSError):
            tool_status = {}
    tool_versions = {}
    versions_path = run_dir / "tool-versions.json"
    if versions_path.exists():
        try:
            tool_versions = load_json(versions_path)
        except (json.JSONDecodeError, OSError):
            tool_versions = {}

    by_tool = Counter(f["tool"] for f in findings)
    by_category = Counter(f["category"] for f in findings)
    by_severity = Counter(f["severity"] for f in findings)
    by_status = Counter(f["status"] for f in findings)

    head = ""
    branch = ""
    try:
        import subprocess

        head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo_root, text=True).strip()
        branch = subprocess.check_output(["git", "branch", "--show-current"], cwd=repo_root, text=True).strip()
    except Exception:  # noqa: BLE001
        pass

    summary = {
        "schema_version": 1,
        "run_id": run_dir.name,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "branch": branch,
        "head": head,
        "repository_root": str(repo_root),
        "totals": {
            "findings": len(findings),
            "suppressed": suppressed,
            "open": len(findings) - suppressed,
            "by_tool": dict(sorted(by_tool.items())),
            "by_category": dict(sorted(by_category.items(), key=lambda kv: CATEGORY_PRIORITY.get(kv[0], 99))),
            "by_severity": dict(sorted(by_severity.items())),
            "by_status": dict(sorted(by_status.items())),
        },
        "tools": [
            {
                "name": name,
                "version": tool_versions.get(name, ""),
                "status": (tool_status.get(STATUS_ALIAS.get(name, name)) or {}).get("status", "unknown"),
                "command": (tool_status.get(STATUS_ALIAS.get(name, name)) or {}).get("command"),
                "findings": by_tool.get(name, 0),
            }
            for name in CANONICAL_TOOLS
            if name in tool_status or name in tool_versions or name in by_tool
        ],
        "findings": findings,
    }

    summary_path = run_dir / "static-analysis-summary.json"
    summary_path.write_text(json.dumps(summary, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    combined = {
        "$schema": "https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/schemas/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": merged_runs,
    }
    (run_dir / "combined.sarif").write_text(json.dumps(combined, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    if not args.skip_root_copy:
        (repo_root / "static-analysis-summary.json").write_text(
            json.dumps(summary, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )

    print(f"Findings: {len(findings)} (suppressed {suppressed}) -> {summary_path}")
    for category, count in summary["totals"]["by_category"].items():
        print(f"  {category}: {count}")
    for tool, count in summary["totals"]["by_tool"].items():
        print(f"  [{tool}] {count}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
