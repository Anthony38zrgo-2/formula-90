#!/usr/bin/env python3
"""Deterministic structural validation for Formula-90 agent governance."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


MANDATORY_HANDOFF_HEADERS = (
    "OBJECTIVE",
    "OBSERVED PROBLEM",
    "EXPECTED BEHAVIOR",
    "FAILURE SIGNATURE",
    "CURRENT EVIDENCE",
    "VERIFIED FACTS",
    "CURRENT HYPOTHESIS",
    "HYPOTHESIS STATUS",
    "CHEAPEST FALSIFICATION TEST",
    "EXPECTED SIGNAL",
    "RELEVANT FILES",
    "RELEVANT SYMBOLS",
    "OWNERSHIP",
    "DEPENDENCIES",
    "KNOWN CONSTRAINTS",
    "DO NOT MODIFY",
    "REJECTED HYPOTHESES",
    "VALIDATION COMMANDS",
    "BASELINE",
    "ACCEPTANCE CRITERIA",
    "ATTEMPT / HYPOTHESIS BUDGET",
    "ROLLBACK CONDITIONS",
    "ESCALATION CONDITIONS",
)

OBSOLETE_ATTEMPT_PATTERNS = (
    r"three meaningful failed hypotheses",
    r"attempting to fix .* 3 times",
    r"fails repeatedly",
    r"after multiple tweaks",
    r"within 2 attempts",
)


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8-sig")


def parse_frontmatter(path: Path, errors: list[str]) -> tuple[str, str]:
    text = read_text(path)
    match = re.match(r"^---\s*\n(.*?)\n---\s*\n", text, re.DOTALL)
    if not match:
        errors.append(f"invalid skill frontmatter: {path}")
        return "", ""
    fields: dict[str, str] = {}
    for line in match.group(1).splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            fields[key.strip()] = value.strip()
    if set(fields) != {"name", "description"}:
        errors.append(f"skill frontmatter must contain only name/description: {path}")
    if not fields.get("description") or "TODO" in fields.get("description", ""):
        errors.append(f"missing skill description: {path}")
    return fields.get("name", ""), fields.get("description", "")


def validate_markdown_links(root: Path, paths: list[Path], errors: list[str]) -> None:
    link_pattern = re.compile(r"\[[^\]]*\]\(([^)]+)\)")
    for path in paths:
        for target in link_pattern.findall(read_text(path)):
            target = target.strip().split("#", 1)[0]
            if not target or target.startswith(("http://", "https://", "mailto:")):
                continue
            resolved = (path.parent / target).resolve()
            try:
                resolved.relative_to(root.resolve())
            except ValueError:
                errors.append(f"link escapes repository: {path}: {target}")
                continue
            if not resolved.exists():
                errors.append(f"broken relative link: {path}: {target}")


def validate_inline_paths(root: Path, paths: list[Path], errors: list[str]) -> None:
    prefixes = (".agents/", "docs/", "tools/", "game/")
    for source in paths:
        for token in re.findall(r"`([^`\n]+)`", read_text(source)):
            candidate = token.strip().replace("\\", "/")
            if not candidate.startswith(prefixes) or any(mark in candidate for mark in "<*{"):
                continue
            candidate = candidate.split()[0].rstrip(".,;:")
            target = root / candidate
            if not target.exists():
                errors.append(f"reference to nonexistent repository path: {source}: {candidate}")


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    agents_root = root / ".agents"
    registry_path = agents_root / "registry" / "skills.json"
    try:
        registry = json.loads(read_text(registry_path))
    except (OSError, json.JSONDecodeError) as exc:
        return [f"cannot read skill registry: {exc}"]

    registry_ids: set[str] = set()
    manifest_paths: dict[str, Path] = {}
    for item in registry.get("skills", []):
        skill_id = item.get("id", "")
        manifest = root / item.get("manifest_path", "")
        if not skill_id or skill_id in registry_ids:
            errors.append(f"missing or duplicate registered skill id: {skill_id!r}")
            continue
        registry_ids.add(skill_id)
        manifest_paths[skill_id] = manifest
        if not manifest.is_file():
            errors.append(f"registered skill manifest missing: {skill_id}: {manifest}")

    skill_dirs = sorted(path for path in (agents_root / "skills").iterdir() if path.is_dir())
    disk_ids = {path.name for path in skill_dirs}
    for skill_dir in skill_dirs:
        manifest = skill_dir / "SKILL.md"
        if not manifest.is_file():
            errors.append(f"invalid skill directory (SKILL.md missing): {skill_dir}")
            continue
        name, _ = parse_frontmatter(manifest, errors)
        if name != skill_dir.name:
            errors.append(f"skill name/folder mismatch: {manifest}: {name!r}")
        if name not in registry_ids:
            errors.append(f"skill exists but is not registered: {name}")
    for skill_id in registry_ids - disk_ids:
        errors.append(f"registered skill directory missing: {skill_id}")

    for role_path in sorted((agents_root / "agents").glob("*.json")):
        try:
            role = json.loads(read_text(role_path))
        except json.JSONDecodeError as exc:
            errors.append(f"invalid role JSON: {role_path}: {exc}")
            continue
        for skill_id in role.get("skills", []):
            if skill_id not in registry_ids:
                errors.append(f"role references unknown skill: {role_path}: {skill_id}")

    routing_doc = read_text(agents_root / "AGENTS.md")
    root_doc = read_text(root / "AGENTS.md")
    referenced = set(re.findall(r"`([a-z0-9][a-z0-9-]+)`", routing_doc + "\n" + root_doc))
    for skill_id in referenced:
        if skill_id.endswith(("-physics", "-safety", "-validation", "-analysis", "-lookup", "-query", "-guardrails", "-handoff", "-collection", "-diagnostics", "-telemetry", "-aerodynamics", "-powertrain")) and skill_id not in registry_ids:
            errors.append(f"AGENTS.md references unknown skill: {skill_id}")

    template_path = root / "docs" / "ai" / "handoff_template.md"
    template_headers = set(re.findall(r"^# (.+)$", read_text(template_path), re.MULTILINE))
    for header in MANDATORY_HANDOFF_HEADERS:
        if header not in template_headers:
            errors.append(f"mandatory Context Bundle header missing: {header}")

    governance_paths = [agents_root / "AGENTS.md", *manifest_paths.values()]
    for path in governance_paths:
        if not path.is_file():
            continue
        lowered = read_text(path).lower()
        for pattern in OBSOLETE_ATTEMPT_PATTERNS:
            if re.search(pattern, lowered):
                errors.append(f"obsolete/conflicting attempt rule in {path}: {pattern}")

    graph: dict[str, set[str]] = {skill_id: set() for skill_id in registry_ids}
    skill_ref_pattern = re.compile(r"\.\./([a-z0-9-]+)/SKILL\.md")
    for skill_id, manifest in manifest_paths.items():
        if manifest.is_file():
            graph[skill_id].update(skill_ref_pattern.findall(read_text(manifest)))
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(node: str, chain: tuple[str, ...]) -> None:
        if node in visiting:
            errors.append("circular skill routing: " + " -> ".join((*chain, node)))
            return
        if node in visited:
            return
        visiting.add(node)
        for target in graph.get(node, set()):
            if target not in registry_ids:
                errors.append(f"skill references unknown sibling skill: {node}: {target}")
            else:
                visit(target, (*chain, node))
        visiting.remove(node)
        visited.add(node)

    for skill_id in sorted(registry_ids):
        visit(skill_id, ())

    markdown_paths = [root / "AGENTS.md", agents_root / "AGENTS.md"]
    markdown_paths.extend(path for path in (root / "docs" / "ai").glob("*.md"))
    markdown_paths.extend(path for path in manifest_paths.values() if path.is_file())
    validate_markdown_links(root, markdown_paths, errors)
    validate_inline_paths(root, markdown_paths, errors)
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()
    errors = validate(args.root.resolve())
    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        print(f"agent validation: FAIL ({len(errors)} error(s))")
        return 1
    print("agent validation: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
