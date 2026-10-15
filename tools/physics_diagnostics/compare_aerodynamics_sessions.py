import argparse
import csv
import json
import math
from pathlib import Path


def percentile(values, fraction):
    ordered = sorted(values)
    position = (len(ordered) - 1) * fraction
    lower = math.floor(position)
    upper = math.ceil(position)
    return ordered[lower] + (ordered[upper] - ordered[lower]) * (position - lower)


def load_session(directory, expected_backend):
    metadata = json.loads((directory / "run_metadata.json").read_text(encoding="utf-8-sig"))
    summary = json.loads((directory / "session_summary.json").read_text(encoding="utf-8-sig"))
    if metadata["backend"] != expected_backend or summary["backend"] != expected_backend:
        raise ValueError(f"{directory}: unexpected backend")
    with (directory / "physics_samples.csv").open(newline="", encoding="utf-8-sig") as stream:
        samples = [{key: float(value) for key, value in row.items()} for row in csv.DictReader(stream)]
    if len(samples) < 120:
        raise ValueError(f"{directory}: at least 120 samples after warmup are required")
    for sample in samples:
        if not all(math.isfinite(value) for value in sample.values()):
            raise ValueError(f"{directory}: nonfinite measurement")
        if sample["vehicle_count"] != 1 or sample["advancing_vehicles"] != 1:
            raise ValueError(f"{directory}: the player vehicle must advance in every measured tick")
    moving_fraction = sum(sample["moving_vehicles"] == 1 for sample in samples) / len(samples)
    if moving_fraction < 0.9:
        raise ValueError(f"{directory}: the player must drive during at least 90% of samples ({moving_fraction:.1%} valid)")
    metrics = {}
    for metric in ["contact_sampling_microseconds", "simulation_step_microseconds", "godot_physics_microseconds", "godot_process_microseconds"]:
        values = [sample[metric] for sample in samples]
        metrics[metric] = {label: percentile(values, fraction) for label, fraction in [("p50", 0.5), ("p95", 0.95), ("p99", 0.99)]}
    combined = [sample["contact_sampling_microseconds"] + sample["simulation_step_microseconds"] for sample in samples]
    metrics["native_contacts_and_rust_microseconds"] = {label: percentile(combined, fraction) for label, fraction in [("p50", 0.5), ("p95", 0.95), ("p99", 0.99)]}
    return metadata, summary, metrics


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("standard_directory", type=Path)
    parser.add_argument("simplified_directory", type=Path)
    arguments = parser.parse_args()
    standard_metadata, standard_summary, standard_metrics = load_session(arguments.standard_directory, "standard")
    simplified_metadata, simplified_summary, simplified_metrics = load_session(arguments.simplified_directory, "simplified")
    for field in ["source", "processor", "vehicle_count", "physics_frequency_hertz", "binary_sha256", "physical_profile_sha256"]:
        if standard_metadata[field] != simplified_metadata[field]:
            raise ValueError(f"Incompatible session metadata: {field}")
    for field in ["physics_frequency_hertz", "audio_policy"]:
        if standard_summary[field] != simplified_summary[field]:
            raise ValueError(f"Incompatible session settings: {field}")
    report = {"standard": standard_metrics, "simplified": simplified_metrics, "reduction_percent": {},
              "scope": "one player vehicle in the existing Fuji session; Rust timing includes all physics systems and facade work",
              "limitation": "Interactive sessions do not reproduce identical player inputs or trajectories",
              "human_driving_approval": "pending"}
    for metric, statistics in standard_metrics.items():
        report["reduction_percent"][metric] = {label: 100.0 * (1.0 - simplified_metrics[metric][label] / value)
                                               if value > 0.0 else None for label, value in statistics.items()}
    print(json.dumps(report, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
