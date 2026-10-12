import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


REPOSITORY_DIRECTORY = Path(__file__).resolve().parents[2]
GEOMETRIC_PROFILE_PATH = REPOSITORY_DIRECTORY / "game/data/vehicles/f1_2030/profiles/geometric/vehicle.json"


def resolve_vehicle_profile(profile_path):
    profile_path = Path(profile_path).resolve()
    resolver_command = [
        "cargo", "run", "--quiet", "--manifest-path",
        str(REPOSITORY_DIRECTORY / "game/crates/Cargo.toml"),
        "--package", "vehicle_physics_engine", "--bin", "resolve_vehicle_configuration",
        "--", str(REPOSITORY_DIRECTORY), str(profile_path),
    ]
    completed = subprocess.run(
        resolver_command, cwd=REPOSITORY_DIRECTORY, capture_output=True,
        text=True, encoding="utf-8", errors="replace", env=os.environ.copy(),
    )
    if completed.returncode:
        raise RuntimeError(f"Vehicle profile resolution failed for {profile_path}: {completed.stderr}")
    return json.loads(completed.stdout)


def load_vehicle_configuration(profile_path):
    profile_path = Path(profile_path).resolve()
    document = json.loads(profile_path.read_text(encoding="utf-8-sig"))
    if "domains" in document:
        return resolve_vehicle_profile(profile_path)["configuration"]
    return document


def vehicle_profile_asset_contract(profile_path):
    profile_path = Path(profile_path).resolve()
    document = json.loads(profile_path.read_text(encoding="utf-8-sig"))
    if "domains" in document:
        resolved = resolve_vehicle_profile(profile_path)
        digest = resolved["provenance"]["global_digest_sha256"]
        digest_contract = "vehicle_profile_provenance_v1"
    else:
        digest = hashlib.sha256(profile_path.read_bytes()).hexdigest()
        digest_contract = "file_sha256"
    return {
        "physics_profile": "res://" + profile_path.relative_to(REPOSITORY_DIRECTORY / "game").as_posix(),
        "physics_sha256": digest.upper(),
        "physics_digest_contract": digest_contract,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("profile_path", type=Path)
    parser.add_argument("--digest", action="store_true")
    arguments = parser.parse_args()
    contract = vehicle_profile_asset_contract(arguments.profile_path)
    print(contract["physics_sha256"] if arguments.digest else json.dumps(contract, indent=2))


if __name__ == "__main__":
    main()
