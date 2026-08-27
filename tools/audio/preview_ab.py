"""Build and render a cheap current-vs-candidate audio human gate."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import uuid
from collections.abc import Sequence
from dataclasses import asdict
from pathlib import Path

from tools.audio.bank_generator import generate_bank
from tools.audio.bank_spec import DEFAULT_SOURCE_DIR
from tools.audio.bank_validator import Finding, validate_bank
from tools.audio.render_audio_scenario import BANK_DIR as DEFAULT_RUNTIME_BANK
from tools.audio.render_audio_scenario import render_scenario
from tools.audio.scenarios import SCENARIO_FACTORIES
from tools.common.content_flow import ContentFlowError, preview
from tools.common.output_policy import OutputMode, OutputPolicyError, validate_output_path

DEFAULT_OUTPUT = Path("scratch/audio/ab")
DEFAULT_SCENARIO = "idle_to_redline_with_shifts"


class AudioPreviewError(ValueError):
    """Raised when an A/B preview cannot be produced safely."""


def _resolve_input(root: Path, value: str | Path, label: str) -> Path:
    path = Path(value)
    if not path.is_absolute():
        path = root / path
    try:
        resolved = path.resolve(strict=True)
    except FileNotFoundError as error:
        raise AudioPreviewError(f"{label} does not exist: {path}") from error
    if root not in resolved.parents:
        raise AudioPreviewError(f"{label} must be inside the repository: {resolved}")
    return resolved


def _errors(findings: list[Finding]) -> list[Finding]:
    return [finding for finding in findings if finding.level == "error"]


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _relative(root: Path, path: Path) -> str:
    return path.relative_to(root).as_posix()


def build_ab_preview(
    repo_root: str | Path,
    source_dir: str | Path = DEFAULT_SOURCE_DIR,
    runtime_bank: str | Path = DEFAULT_RUNTIME_BANK,
    output_dir: str | Path = DEFAULT_OUTPUT,
    scenario_name: str = DEFAULT_SCENARIO,
) -> dict[str, object]:
    """Build a candidate and render the same scenario against both banks."""

    root = Path(repo_root).resolve(strict=True)
    source = _resolve_input(root, source_dir, "Source directory")
    current_bank = _resolve_input(root, runtime_bank, "Runtime bank")
    if not source.is_dir() or not current_bank.is_dir():
        raise AudioPreviewError("Source directory and runtime bank must be directories")
    scenario_factory = SCENARIO_FACTORIES.get(scenario_name)
    if scenario_factory is None:
        choices = ", ".join(sorted(SCENARIO_FACTORIES))
        raise AudioPreviewError(f"Unknown scenario {scenario_name!r}; choose: {choices}")

    output = validate_output_path(root, output_dir, OutputMode.PREVIEW).path
    staging_parent = root / "scratch" / ".audio-ab-staging"
    if output == staging_parent or staging_parent in output.parents:
        raise AudioPreviewError(f"Output overlaps internal staging: {output}")
    staging = staging_parent / uuid.uuid4().hex
    validate_output_path(root, staging, OutputMode.PREVIEW)
    staging.mkdir(parents=True)

    try:
        candidate_bank = staging / "candidate-bank"
        generate_bank(source, candidate_bank)
        candidate_findings = validate_bank(candidate_bank)
        current_findings = validate_bank(current_bank)
        validation_errors = _errors(candidate_findings) + _errors(current_findings)
        if validation_errors:
            detail = "; ".join(f"{item.file}: {item.code}" for item in validation_errors)
            raise AudioPreviewError(f"Bank validation failed: {detail}")

        scenario = scenario_factory()
        current_dir = staging / "current"
        candidate_dir = staging / "candidate"
        current_wav = current_dir / f"{scenario.name}.wav"
        candidate_wav = candidate_dir / f"{scenario.name}.wav"
        current_report = render_scenario(
            scenario,
            current_bank,
            current_wav,
            current_dir / f"{scenario.name}.csv",
            current_dir / f"{scenario.name}.json",
        )
        candidate_report = render_scenario(
            scenario_factory(),
            candidate_bank,
            candidate_wav,
            candidate_dir / f"{scenario.name}.csv",
            candidate_dir / f"{scenario.name}.json",
        )

        final_current_wav = output / "current" / current_wav.name
        final_candidate_wav = output / "candidate" / candidate_wav.name
        summary: dict[str, object] = {
            "schema_version": 1,
            "scenario": scenario.name,
            "description": scenario.description,
            "current": {
                "bank": _relative(root, current_bank),
                "wav": _relative(root, final_current_wav),
                "sha256": _sha256(current_wav),
                "peak": current_report["peak"],
                "rms": current_report["rms"],
                "validation_findings": [asdict(item) for item in current_findings],
            },
            "candidate": {
                "bank": _relative(root, output / "candidate-bank"),
                "wav": _relative(root, final_candidate_wav),
                "sha256": _sha256(candidate_wav),
                "peak": candidate_report["peak"],
                "rms": candidate_report["rms"],
                "validation_findings": [asdict(item) for item in candidate_findings],
            },
            "same_render": _sha256(current_wav) == _sha256(candidate_wav),
            "human_gate": "Listen to current and candidate; keep, reject, or iterate the candidate.",
        }
        (staging / "comparison.json").write_text(
            json.dumps(summary, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        preview(root, staging, output, replace=True)
        return summary
    finally:
        if staging.exists():
            shutil.rmtree(staging)


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Build a cheap audio A/B human gate")
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE_DIR)
    parser.add_argument("--runtime-bank", type=Path, default=DEFAULT_RUNTIME_BANK)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--scenario",
        choices=sorted(SCENARIO_FACTORIES),
        default=DEFAULT_SCENARIO,
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    try:
        summary = build_ab_preview(
            args.repo_root,
            args.source,
            args.runtime_bank,
            args.output,
            args.scenario,
        )
    except (AudioPreviewError, ContentFlowError, OSError, OutputPolicyError) as error:
        raise SystemExit(f"audio-preview: {error}") from error
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
