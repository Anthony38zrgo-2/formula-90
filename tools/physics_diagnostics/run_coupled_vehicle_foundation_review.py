from __future__ import annotations

import argparse
import datetime
import hashlib
import json as structured_serialization
import math
import platform
import re
import subprocess
import sys as system_runtime
from pathlib import Path
from time import perf_counter

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
system_runtime.path.insert(0, str(REPOSITORY_ROOT))
from tools.common.output_policy import validate_output_path


def read_document(path: Path):
    return structured_serialization.loads(path.read_text(encoding="utf-8-sig"))


def write_document(path: Path, value):
    destination = validate_output_path(REPOSITORY_ROOT, path, "preview").path
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(
        structured_serialization.dumps(value, indent=2, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def write_text(path: Path, value: str):
    destination = validate_output_path(REPOSITORY_ROOT, path, "preview").path
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(value, encoding="utf-8")


def content_digest(path: Path):
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1048576), b""):
            digest.update(block)
    return digest.hexdigest()


def git_output(*arguments: str):
    result = subprocess.run(
        ["git", *arguments], cwd=REPOSITORY_ROOT, capture_output=True, check=True
    )
    return result.stdout.decode("utf-8", errors="replace")


def capture_source_provenance():
    tracked_paths = git_output(
        "ls-files", "-z", "--", "game/crates/vehicle-physics-engine",
        "game/crates/game-sim", "game/crates/formula90-core", "native",
        "game/crates/Cargo.toml", "game/crates/Cargo.lock", "game/project.godot",
        "game/data/vehicles/f1_2030", "game/tracks/fuji76_77/metadata/package.json",
        "scripts/run_f1_94.ps1",
    ).split("\0")
    additional_paths = [
        "tools/physics_diagnostics/run_coupled_vehicle_foundation_review.py",
        "docs/coupled_vehicle_architecture_contract.json",
        "docs/coupled_vehicle_validation_specification.json",
        "docs/coupled_vehicle_foundation_definition.json",
        "docs/coupled_vehicle_foundation_scenario.json",
        "game/assets/models/vehicles/f1-2030/manifest.json",
    ]
    reference_directory = REPOSITORY_ROOT / "game/crates/vehicle-physics-engine"
    additional_paths.extend(
        path.relative_to(REPOSITORY_ROOT).as_posix()
        for directory in ["src", "tests", "examples"]
        for path in (reference_directory / directory).glob("*suspension*.rs")
    )
    files = [
        {"path": relative_path, "content_digest": content_digest(REPOSITORY_ROOT / relative_path)}
        for relative_path in sorted(set(tracked_paths + additional_paths))
        if relative_path and (REPOSITORY_ROOT / relative_path).is_file()
    ]
    encoded_manifest = structured_serialization.dumps(files, sort_keys=True).encode()
    installed_binaries = [
        {"path": path.relative_to(REPOSITORY_ROOT).as_posix(), "content_digest": content_digest(path)}
        for path in sorted((REPOSITORY_ROOT / "game/addons/formula90s/bin").glob("*.dll"))
    ]
    return {
        "schema_version": 1,
        "captured_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "repository_root": str(REPOSITORY_ROOT),
        "branch": git_output("branch", "--show-current").strip(),
        "source_head": git_output("rev-parse", "HEAD").strip(),
        "source_fingerprint": hashlib.sha256(encoded_manifest).hexdigest(),
        "digest_algorithm": "SHA-256",
        "source_files": files,
        "status_porcelain": git_output("status", "--porcelain=v1", "--untracked-files=all"),
        "staged_diff_digest": hashlib.sha256(git_output("diff", "--cached", "--binary").encode()).hexdigest(),
        "installed_build_source": (REPOSITORY_ROOT / "game/BUILD_SOURCE").read_text().strip(),
        "installed_binaries": installed_binaries,
        "binary_source_parity": "unverified_for_dirty_source; installed libraries are not executed",
        "workspace_strategy": "Remain in inventoried checkout; fresh scratch Cargo output; no branch switch or binary promotion",
        "toolchain": {
            executable: subprocess.run(
                [executable, "--version"], cwd=REPOSITORY_ROOT,
                capture_output=True, text=True, check=True,
            ).stdout.strip()
            for executable in ["cargo", "rustc"]
        },
        "host": {
            "platform": platform.platform(),
            "processor": platform.processor(),
            "python_version": platform.python_version(),
        },
    }


def run_command(output_directory: Path, identifier: str, command: list[str]):
    print(structured_serialization.dumps({"running": identifier}), flush=True)
    started = perf_counter()
    process = subprocess.run(
        command, cwd=REPOSITORY_ROOT, capture_output=True,
        text=True, encoding="utf-8", errors="replace",
    )
    elapsed = perf_counter() - started
    write_text(output_directory / (identifier + "_standard_output.txt"), process.stdout)
    write_text(output_directory / (identifier + "_standard_error.txt"), process.stderr)
    return {
        "identifier": identifier, "command": command,
        "working_directory": str(REPOSITORY_ROOT), "exit_code": process.returncode,
        "elapsed_wall_time_seconds": elapsed,
        "standard_output": process.stdout, "standard_error": process.stderr,
    }


def summarize_tests(result):
    summaries = re.findall(
        r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed;",
        result["standard_output"],
    )
    return {
        "passed": sum(int(passed) for passed, failed in summaries),
        "failed": sum(int(failed) for passed, failed in summaries),
        "failure_names": re.findall(
            r"^test ([^\s]+) \.\.\. FAILED$", result["standard_output"], re.MULTILINE
        ),
    }


def validate_contract(contract, specification, profile):
    if contract["schema_version"] != 1 or specification["schema_version"] != 1:
        raise ValueError("Unsupported foundation document version")
    mass = contract["mass_contract"]
    allocations = sum(group["mass_kilograms"] for group in mass["provisional_allocations"])
    tolerance = next(
        gate["absolute_tolerance_kilograms"]
        for gate in specification["foundation_gates"]
        if gate["identifier"] == "mass_accounting"
    )
    expected_unfueled = mass["base_budget_kilograms"] + 2 * (
        mass["front_wheel_assembly_kilograms"] + mass["rear_wheel_assembly_kilograms"]
    )
    declarations = [
        (allocations, mass["base_budget_kilograms"]),
        (expected_unfueled, mass["complete_unfueled_operating_mass_kilograms"]),
        (expected_unfueled + mass["initial_fuel_mass_kilograms"], mass["initial_operating_mass_kilograms"]),
        (expected_unfueled + mass["fuel_capacity_kilograms"], mass["full_fuel_operating_mass_kilograms"]),
        (profile["chassis"]["vehicle_mass"], mass["base_budget_kilograms"]),
        (profile["tires"]["front"]["wheel_mass"], mass["front_wheel_assembly_kilograms"]),
        (profile["tires"]["rear"]["wheel_mass"], mass["rear_wheel_assembly_kilograms"]),
        (profile["fuel"]["initial_kg"], mass["initial_fuel_mass_kilograms"]),
        (profile["fuel"]["capacity_kg"], mass["fuel_capacity_kilograms"]),
    ]
    if not profile["chassis"]["vehicle_mass_excludes_wheel_assemblies"]:
        raise ValueError("Active profile does not declare the wheel-excluded base contract")
    if any(not math.isfinite(actual) or abs(actual - expected) > tolerance for actual, expected in declarations):
        raise ValueError("Mass contract disagrees with allocations or actual profile")
    responsibilities = [entry["responsibility"] for entry in contract["ownership_contract"]]
    if len(responsibilities) != len(set(responsibilities)):
        raise ValueError("Duplicate ownership responsibilities")
    if contract["runtime_activation"]:
        raise ValueError("Foundation contract must not claim live activation")
    if contract["geometry_library"]["godot_custom_integrator_is_exclusive_collision_ownership"]:
        raise ValueError("Invalid Godot collision ownership assertion")


def diagnostic_metrics(report):
    if not report["samples"]:
        raise ValueError("Empty reference capture")
    initial = report["samples"][0]
    final = report["samples"][-1]
    initial_state = initial["state"]
    final_state = final["state"]
    residual = final["diagnostics"]["total_energy_joules"] - initial["diagnostics"]["total_energy_joules"]
    for field, multiplier in [
        ("dissipated_energy_joules", 1),
        ("collision_energy_change_joules", -1),
        ("external_work_joules", -1),
        ("steering_actuator_work_joules", -1),
    ]:
        residual += multiplier * (final_state[field] - initial_state[field])
    initial_energy = initial["diagnostics"]["total_energy_joules"]
    metrics = {
        "sample_count": len(report["samples"]),
        "duration_seconds": final_state["simulated_time_seconds"] - initial_state["simulated_time_seconds"],
        "initial_total_energy_joules": initial_energy,
        "energy_residual_joules": residual,
        "relative_energy_residual": abs(residual) / max(abs(initial_energy), 1),
        "final_normal_force_sum_newtons": sum(final["diagnostics"]["normal_force_newtons"]),
        "final_body_origin_world_metres": final_state["body_origin_world_metres"],
        "final_wheel_travel_metres": final_state["wheel_travel_metres"],
        "elapsed_solver_wall_time_seconds": report["elapsed_wall_time_seconds"],
        "physical_property_classification": "diagnostic_only_five_component_estimate",
        "runtime_validation": False,
    }
    if any(isinstance(value, (int, float)) and not math.isfinite(value) for value in metrics.values()):
        raise ValueError("Nonfinite reference metrics")
    return metrics


def verify_preservation(pre_edit_provenance, before, after):
    intentionally_reviewed = {
        "instrucciones.md",
        "docs/suspension_coupled_dynamics_backlog.json",
        "docs/suspension_coupled_dynamics_implementation.md",
    }
    protected_changes = []
    for entry in pre_edit_provenance.get("preexisting_changes", []):
        relative_path = entry["path"].replace("\\", "/")
        if relative_path not in intentionally_reviewed:
            if content_digest(REPOSITORY_ROOT / relative_path) != entry["sha256"]:
                protected_changes.append(relative_path)
    binary_changes = before["installed_binaries"] != after["installed_binaries"]
    index_changed = before["staged_diff_digest"] != after["staged_diff_digest"]
    return {
        "changed_protected_paths": protected_changes,
        "installed_binaries_changed": binary_changes,
        "index_changed": index_changed,
        "build_source_changed": before["installed_build_source"] != after["installed_build_source"],
        "head_changed": before["source_head"] != after["source_head"],
        "branch_changed": before["branch"] != after["branch"],
        "source_fingerprint_changed_during_execution": before["source_fingerprint"] != after["source_fingerprint"],
        "passed": not protected_changes and not binary_changes and not index_changed
            and before["source_head"] == after["source_head"]
            and before["branch"] == after["branch"]
            and before["installed_build_source"] == after["installed_build_source"]
            and before["source_fingerprint"] == after["source_fingerprint"],
    }


def main():
    parser = argparse.ArgumentParser(description="Reproduce Sprint 0 reference evidence without activating runtime physics.")
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument("--pre-edit-provenance", type=Path)
    arguments = parser.parse_args()
    output_directory = validate_output_path(REPOSITORY_ROOT, arguments.output_directory, "preview").path
    output_directory.mkdir(parents=True, exist_ok=True)
    cargo_output = validate_output_path(REPOSITORY_ROOT, output_directory / "fresh_cargo_output", "preview").path
    if cargo_output.exists():
        raise ValueError("Cargo output already exists; choose a new output directory to prevent cache reuse")
    if (output_directory / "foundation_validation_report.json").exists():
        raise ValueError("Foundation evidence already exists; choose a new output directory")
    contract = read_document(REPOSITORY_ROOT / "docs/coupled_vehicle_architecture_contract.json")
    specification = read_document(REPOSITORY_ROOT / "docs/coupled_vehicle_validation_specification.json")
    profile_path = REPOSITORY_ROOT / contract["vehicle_profile"]
    profile = read_document(profile_path)
    validate_contract(contract, specification, profile)
    gates = {gate["identifier"]: gate for gate in specification["foundation_gates"]}
    before = capture_source_provenance()
    write_document(output_directory / "execution_provenance.json", before)
    pre_edit = read_document(arguments.pre_edit_provenance) if arguments.pre_edit_provenance else {
        "preexisting_changes": [
            {"path": entry["path"], "sha256": entry["content_digest"]}
            for entry in before["source_files"]
        ]
    }
    manifest_path = "game/crates/vehicle-physics-engine/Cargo.toml"
    cargo_prefix = ["cargo", "test", "--locked", "--offline", "--manifest-path", manifest_path,
        "--target-dir", str(cargo_output)]
    required_tests = run_command(output_directory, "required_reference_tests", cargo_prefix + [
        "--test", "suspension_multibody_test", "--test", "suspension_mass_audit_test",
        "--test", "fuel_model_test",
    ])
    known_tests = run_command(output_directory, "geometric_regression_tests", cargo_prefix + [
        "--test", "suspension_f1_2030_geometric_test",
    ])
    build = run_command(output_directory, "release_reference_build", [
        "cargo", "build", "--locked", "--offline", "--release", "--manifest-path", manifest_path,
        "--target-dir", str(cargo_output), "--example", "suspension_mass_audit",
        "--example", "coupled_suspension_reference",
    ])
    required_summary = summarize_tests(required_tests)
    geometric_summary = summarize_tests(known_tests)
    checks = {
        "contract_allocation_and_profile_validation": True,
        "required_reference_tests": required_tests["exit_code"] == 0
            and required_summary["passed"] == gates["existing_reference_tests"]["expected_pass_count"]
            and required_summary["failed"] == 0,
        "geometric_suite_has_only_known_failure": (
            known_tests["exit_code"] == 0 and geometric_summary["passed"] == 7
        ) or (
            geometric_summary["passed"] == gates["known_geometric_manifest_failure"]["expected_pass_count"]
            and geometric_summary["failed"] == 1
            and geometric_summary["failure_names"] == gates["known_geometric_manifest_failure"]["allowed_failure_names"]
        ),
        "release_reference_build": build["exit_code"] == 0,
    }
    captures = {}
    commands = [required_tests, known_tests, build]
    if build["exit_code"] == 0:
        executable_suffix = ".exe" if platform.system() == "Windows" else ""
        executable_directory = cargo_output / "release/examples"
        audit_result = run_command(output_directory, "mass_audit", [
            str(executable_directory / ("suspension_mass_audit" + executable_suffix)), str(profile_path),
        ])
        commands.append(audit_result)
        if audit_result["exit_code"] != 0:
            raise ValueError("Mass audit execution failed")
        audit = structured_serialization.loads(audit_result["standard_output"])
        write_document(output_directory / "mass_audit.json", audit)
        expected_mass = contract["mass_contract"]["initial_operating_mass_kilograms"]
        checks["actual_profile_mass_accounting"] = (
            abs(audit["dry_vehicle_mass_kilograms"] - contract["mass_contract"]["complete_unfueled_operating_mass_kilograms"])
            <= gates["mass_accounting"]["absolute_tolerance_kilograms"]
            and abs(audit["total_vehicle_mass_kilograms"] - expected_mass)
            <= gates["mass_accounting"]["absolute_tolerance_kilograms"]
            and audit["suspended_mass_kilograms"] is None
        )
        weight = expected_mass * 9.80665
        checks["static_load_accounting"] = abs(
            sum(corner["static_normal_force_newtons"] for corner in audit["corners"]) - weight
        ) / max(weight, gates["static_weight_accounting"]["reference_floor_newtons"]) <= gates["static_weight_accounting"]["relative_tolerance"]
        definition = read_document(REPOSITORY_ROOT / specification["fixture_policy"]["reference_definition"])
        base_scenario = read_document(REPOSITORY_ROOT / specification["fixture_policy"]["reference_scenario"])
        for identifier, duration in [("short_settling", 0.3), ("extended_settling", 2.0),
            ("nonlinear_coarse", 0.01), ("nonlinear_fine", 0.01)]:
            scenario = structured_serialization.loads(structured_serialization.dumps(base_scenario))
            scenario["duration_seconds"] = duration
            selected_definition = structured_serialization.loads(structured_serialization.dumps(definition))
            if identifier.startswith("nonlinear"):
                scenario["input"]["contacts"] = [None] * 4
                selected_definition["settings"]["gravity_world_metres_per_second_squared"] = {
                    "x": 0, "y": 0, "z": 0,
                }
            if identifier == "nonlinear_fine":
                selected_definition["settings"]["maximum_substep_seconds"] *= 0.5
            scenario_path = output_directory / (identifier + "_scenario.json")
            definition_path = output_directory / (identifier + "_diagnostic_definition.json")
            write_document(scenario_path, scenario)
            write_document(definition_path, selected_definition)
            result = run_command(output_directory, identifier, [
                str(executable_directory / ("coupled_suspension_reference" + executable_suffix)),
                str(profile_path), str(definition_path), str(scenario_path),
            ])
            commands.append(result)
            if result["exit_code"] != 0:
                raise ValueError("Reference capture failed: " + identifier)
            reference_report = structured_serialization.loads(result["standard_output"])
            write_document(output_directory / (identifier + "_capture.json"), reference_report)
            captures[identifier] = diagnostic_metrics(reference_report)
            captures[identifier]["input_hashes"] = {
                "vehicle_profile": content_digest(profile_path),
                "diagnostic_definition": content_digest(definition_path),
                "scenario": content_digest(scenario_path),
            }
        checks["short_settling_energy"] = captures["short_settling"]["relative_energy_residual"] <= gates["estimated_settling_energy"]["relative_tolerance"]
        checks["extended_settling_energy"] = captures["extended_settling"]["relative_energy_residual"] <= gates["estimated_settling_energy"]["relative_tolerance"]
        support_error = abs(captures["extended_settling"]["final_normal_force_sum_newtons"] - weight) / weight
        captures["extended_settling"]["relative_support_error"] = support_error
        checks["extended_settling_support"] = support_error <= gates["estimated_settling_support"]["relative_tolerance"]
        coarse_position = captures["nonlinear_coarse"]["final_body_origin_world_metres"]
        fine_position = captures["nonlinear_fine"]["final_body_origin_world_metres"]
        position_difference = math.sqrt(sum(
            (coarse_position[axis] - fine_position[axis]) ** 2 for axis in ["x", "y", "z"]
        ))
        captures["nonlinear_fine"]["coarse_fine_position_difference_metres"] = position_difference
        checks["nonlinear_position_convergence"] = position_difference < gates["nonlinear_short_reference_convergence"]["absolute_position_tolerance_metres"]
        checks["nonlinear_fine_energy"] = captures["nonlinear_fine"]["relative_energy_residual"] < gates["nonlinear_short_reference_convergence"]["energy_relative_tolerance"]
    after = capture_source_provenance()
    preservation = verify_preservation(pre_edit, before, after)
    checks["protected_file_preservation"] = preservation["passed"]
    report = {
        "schema_version": 1, "sprint": 0, "items": [1, 2, 3, 4],
        "delivery_state": "validated_awaiting_human_approval" if all(checks.values()) else "validation_failed",
        "human_approval": "pending", "runtime_activation": False,
        "source_head": before["source_head"], "source_fingerprint": before["source_fingerprint"],
        "required_tests": required_summary, "geometric_tests": geometric_summary,
        "observed_failures": geometric_summary["failure_names"],
        "known_failures": ["godot_scene_points_at_geometric_by_default"],
        "newly_discovered_baseline_failures": [
            name for name in geometric_summary["failure_names"]
            if name != "godot_scene_points_at_geometric_by_default"
        ],
        "known_failure_classification": "preexisting_manifest_profile_linkage_pending_item_10",
        "checks": checks, "captures": captures, "preservation": preservation,
        "commands": [
            {key: value for key, value in result.items() if key not in ["standard_output", "standard_error"]}
            for result in commands
        ],
        "limitations": [
            "Diagnostic captures use estimated five-component properties, not production calibration",
            "Reference normal contact is planar; brush friction, wheel spin, world impacts and runtime parity remain pending",
            "Analytical heave test uses its own velocity-Verlet stepping; production reference advancement uses semi-implicit Euler",
            "Reference timings exclude full race systems",
            "Installed runtime binaries were not executed, rebuilt or promoted",
        ],
    }
    write_document(output_directory / "foundation_validation_report.json", report)
    print(structured_serialization.dumps({
        "report": str(output_directory / "foundation_validation_report.json"),
        "delivery_state": report["delivery_state"],
        "required_tests": required_summary, "geometric_tests": geometric_summary,
        "checks": checks, "captures": captures,
    }, indent=2), flush=True)
    return 0 if all(checks.values()) else 1


if __name__ == "__main__":
    raise SystemExit(main())
