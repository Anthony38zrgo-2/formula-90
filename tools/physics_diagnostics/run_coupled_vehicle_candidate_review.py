import argparse
import hashlib
import json as structured_serialization
from pathlib import Path
import re
import subprocess
import sys
import time

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "tools"))
from common.output_policy import validate_output_path


def write_artifact(path, document):
    destination = validate_output_path(REPOSITORY_ROOT, path, "preview").path
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(document, encoding="utf-8")


def source_snapshot():
    result = subprocess.run(["git", "ls-files", "-co", "--exclude-standard", "game/crates/vehicle-physics-engine",
        "game/data/vehicles/f1_2030", "native/include/formula90s/core", "native/src/core", "native/src/register_types.cpp",
        "tools/physics_diagnostics", "docs", "instrucciones.md"], cwd=REPOSITORY_ROOT, capture_output=True, text=True, check=True)
    files = {}
    for relative in sorted(set(result.stdout.splitlines())):
        path = REPOSITORY_ROOT / relative
        if path.is_file():
            files[relative] = hashlib.sha256(path.read_bytes()).hexdigest()
    digest = hashlib.sha256(structured_serialization.dumps(files, sort_keys=True).encode("utf-8")).hexdigest()
    return {"source_digest": digest, "files": files}


def execute_command(command, log_path):
    started = time.monotonic()
    result = subprocess.run(command, cwd=REPOSITORY_ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace")
    document = result.stdout + "\n" + result.stderr
    write_artifact(log_path, document)
    summaries = re.findall(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored", document)
    report = {"command": command, "exit_code": result.returncode, "elapsed_wall_time_seconds": time.monotonic() - started,
        "log_path": str(log_path), "passed_tests": sum(int(summary[1]) for summary in summaries),
        "failed_tests": sum(int(summary[2]) for summary in summaries), "ignored_tests": sum(int(summary[3]) for summary in summaries)}
    for line in result.stdout.splitlines():
        if line.startswith("{"):
            report["diagnostic"] = structured_serialization.loads(line)
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--vehicle-profile", type=Path, default=Path("game/data/vehicles/f1_2030/f1_2030_v10_geometric.json"))
    parser.add_argument("--track-source", type=Path, default=Path("game/tracks/fuji76_77/fuji76_77_collision.glb"))
    parser.add_argument("--track-metadata", type=Path, default=Path("game/tracks/fuji76_77/metadata/track.json"))
    parser.add_argument("--duration-seconds", type=float, default=0.2)
    arguments = parser.parse_args()
    output = validate_output_path(REPOSITORY_ROOT, arguments.output, "preview").path
    initial_source = source_snapshot()
    branch = subprocess.check_output(["git", "branch", "--show-current"], cwd=REPOSITORY_ROOT, text=True).strip()
    source_head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=REPOSITORY_ROOT, text=True).strip()
    status = subprocess.check_output(["git", "status", "--porcelain=v1"], cwd=REPOSITORY_ROOT, text=True)
    write_artifact(output / "invocation_provenance.json", structured_serialization.dumps({"branch": branch, "source_head": source_head,
        "git_status": status, "source": initial_source}, indent=2))
    build_output = output / "fresh_cargo_output"
    if build_output.exists():
        raise ValueError("Candidate review requires an unused build directory; select a new output path")
    package_path = output / "fuji_physical_world.json"
    extraction = execute_command([sys.executable, "tools/physics_diagnostics/prepare_physical_world_package.py",
        "--source", str(arguments.track_source), "--output", str(package_path)], output / "physical_package.log")
    manifest = "game/crates/vehicle-physics-engine/Cargo.toml"
    print("Running complete Rust library, integration and example tests in a fresh build directory", flush=True)
    tests = execute_command(["cargo", "test", "--offline", "--no-fail-fast", "--jobs", "1", "--manifest-path", manifest,
        "--target-dir", str(build_output), "--config", "profile.test.debug=0", "--config", "profile.dev.debug=0",
        "--lib", "--tests", "--examples"], output / "rust_tests.log")
    print("Running the candidate world against the source-verified Fuji package", flush=True)
    diagnostic = execute_command(["cargo", "run", "--offline", "--release", "--jobs", "1", "--manifest-path", manifest,
        "--target-dir", str(build_output), "--example", "coupled_vehicle_world_diagnostic", "--", str(REPOSITORY_ROOT),
        str(arguments.vehicle_profile), str(package_path), str(arguments.track_metadata), str(arguments.duration_seconds)], output / "fuji_world.log") if extraction["exit_code"] == 0 else None
    final_source = source_snapshot()
    report = {"schema_version": 1, "branch": branch, "source_head": source_head, "source_digest": initial_source["source_digest"],
        "source_stable_during_review": final_source["source_digest"] == initial_source["source_digest"], "package_extraction": extraction,
        "tests": tests, "fuji_diagnostic": diagnostic, "runtime_activated": False, "human_approved": False,
        "complete_backlog_delivered": False}
    write_artifact(output / "review.json", structured_serialization.dumps(report, indent=2, allow_nan=False))
    print(structured_serialization.dumps({"review": str(output / "review.json"), "passed_tests": tests["passed_tests"],
        "failed_tests": tests["failed_tests"], "runtime_activated": False}), flush=True)
    return 0 if tests["exit_code"] == 0 and diagnostic and diagnostic["exit_code"] == 0 and report["source_stable_during_review"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
