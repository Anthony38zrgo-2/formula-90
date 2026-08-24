




"""Pure VehicleDocument parameter compiler for VehicleBuildIR v1."""
from __future__ import annotations

from hashlib import sha256
import math
from typing import Any

from .canonical import canonical_json_bytes
from .domain import VehicleDocument


class BuildIRError(ValueError):
    def __init__(self, code: str, message: str):
        self.code = code
        super().__init__(message)


def compile_build_ir(
    document: VehicleDocument | dict[str, Any],
    *,
    parameter_values: dict[str, float] | None = None,
    width_policies: dict[str, str] | None = None,
) -> dict[str, Any]:
    vehicle = document if isinstance(document, VehicleDocument) else VehicleDocument.from_dict(document)
    canonical = vehicle.canonical_dict()
    document_bytes = vehicle.canonical_bytes()
    requested = parameter_values or {}
    policies = _validate_width_policies(width_policies or {})
    declared = {item["parameter_id"]: item for item in canonical["parameters"]}
    unknown = sorted(set(requested) - set(declared))
    if unknown:
        raise BuildIRError("BUILD_IR_UNKNOWN_PARAMETER", f"unknown parameter: {unknown[0]}")

    resolved = []
    operations = [{
        "operation_id": "op-0010-assert-source",
        "order": 10,
        "kind": "assert_source_hash",
        "depends_on": [],
        "inputs": {
            "source_path": canonical["source"]["path"],
            "source_sha256": canonical["source"]["sha256"],
            "materialization_source": canonical.get("metadata", {}).get("materialization_source"),
            "materialization_sources": canonical.get("metadata", {}).get("materialization_sources", []),
            "ground_y_m": canonical["coordinate_system"]["ground_y_m"],
        },
        "postconditions": ["source_hash_matches", "source_is_read_only"],
    }]
    previous = "op-0010-assert-source"
    operation_index = 100
    for parameter_id in sorted(declared):
        parameter = declared[parameter_id]
        value = requested.get(parameter_id, parameter.get("absolute_value"))
        _validate_parameter(parameter, value)
        resolved.append({
            "parameter_id": parameter_id,
            "semantic_role": parameter.get("semantic_role"),
            "unit": parameter.get("unit"),
            "baseline_value": parameter.get("baseline_value"),
            "absolute_value": value,
        })
        bindings = parameter.get("build_operations")
        if not isinstance(bindings, list) or not bindings:
            if value != parameter.get("baseline_value"):
                raise BuildIRError(
                    "BUILD_IR_MISSING_BINDING",
                    f"changed parameter {parameter_id!r} has no declared build operation",
                )
            continue
        for binding_index, binding in enumerate(bindings):
            if not isinstance(binding, dict) or not isinstance(binding.get("kind"), str):
                raise BuildIRError("BUILD_IR_INVALID_BINDING", f"invalid binding for {parameter_id!r}")
            operation_id = f"op-{operation_index:04d}-{_slug(parameter_id)}-{binding_index + 1}"
            operation = {
                "operation_id": operation_id,
                "order": operation_index,
                "kind": binding["kind"],
                "depends_on": [previous],
                "inputs": {
                    "parameter_id": parameter_id,
                    "semantic_role": parameter.get("semantic_role"),
                    "baseline_value": parameter.get("baseline_value"),
                    "target_value": value,
                    "unit": parameter.get("unit"),
                    **dict(binding.get("inputs", {})),
                    **({"width_policy": policies[parameter.get("semantic_role")]}
                       if parameter.get("semantic_role") in policies else {}),
                },
                "postconditions": sorted(set(binding.get("postconditions", []))),
            }
            if not operation["postconditions"]:
                raise BuildIRError("BUILD_IR_MISSING_POSTCONDITION", f"operation {operation_id!r} has no postcondition")
            operations.append(operation)
            previous = operation_id
            operation_index += 10

    operations.append({
        "operation_id": "op-0900-validate",
        "order": 900,
        "kind": "validate_vehicle",
        "depends_on": [previous],
        "inputs": {"tolerance_m": canonical["validation_policy"].get("tolerance_m", 0.000001)},
        "postconditions": [
            "topology_matches_source", "uv_membership_matches_source",
            "wheel_anchors_match", "ground_contact_matches",
        ],
    })
    operations.append({
        "operation_id": "op-0990-stage-export",
        "order": 990,
        "kind": "stage_variant_export",
        "depends_on": ["op-0900-validate"],
        "inputs": {"formats": ["blend", "glb"], "publish": False},
        "postconditions": ["artifacts_written_to_staging", "source_unchanged"],
    })
    result = {
        "schema_version": "vehicle-build-ir/v1",
        "document_sha256": sha256(document_bytes).hexdigest().upper(),
        "source_sha256": canonical["source"]["sha256"],
        "policy": {
            "topology_changes_allowed": False,
            "source_mutation_allowed": False,
            "output_scope": "staging_only",
        },
        "parameters": resolved,
        "operations": operations,
    }
    validate_build_ir(result)
    return result


def build_ir_bytes(build_ir: dict[str, Any]) -> bytes:
    validate_build_ir(build_ir)
    return canonical_json_bytes(build_ir)


def validate_build_ir(build_ir: dict[str, Any]) -> None:
    if build_ir.get("schema_version") != "vehicle-build-ir/v1":
        raise BuildIRError("BUILD_IR_SCHEMA_VERSION", "unsupported BuildIR schema")
    policy = build_ir.get("policy", {})
    if policy.get("topology_changes_allowed") is not False or policy.get("source_mutation_allowed") is not False:
        raise BuildIRError("BUILD_IR_UNSAFE_POLICY", "BuildIR safety policy is invalid")
    operations = build_ir.get("operations")
    if not isinstance(operations, list) or not operations:
        raise BuildIRError("BUILD_IR_OPERATIONS", "operations must be non-empty")
    ids = [item.get("operation_id") for item in operations]
    if len(ids) != len(set(ids)):
        raise BuildIRError("BUILD_IR_DUPLICATE_OPERATION", "operation IDs must be unique")
    positions = {operation_id: index for index, operation_id in enumerate(ids)}
    orders = [item.get("order") for item in operations]
    if orders != sorted(orders) or len(orders) != len(set(orders)):
        raise BuildIRError("BUILD_IR_OPERATION_ORDER", "operation order must be strict")
    for index, operation in enumerate(operations):
        for dependency in operation.get("depends_on", []):
            if dependency not in positions or positions[dependency] >= index:
                raise BuildIRError("BUILD_IR_DEPENDENCY_ORDER", "dependency must precede operation")
        if not operation.get("postconditions"):
            raise BuildIRError("BUILD_IR_MISSING_POSTCONDITION", "every operation requires postconditions")


def _validate_parameter(parameter: dict[str, Any], value: Any) -> None:
    if not isinstance(value, (int, float)) or not math.isfinite(value):
        raise BuildIRError("BUILD_IR_PARAMETER_VALUE", "parameter value must be finite")
    if not parameter["minimum"] <= value <= parameter["maximum"]:
        raise BuildIRError("BUILD_IR_PARAMETER_BOUNDS", f"{parameter['parameter_id']!r} is outside bounds")


def _slug(value: str) -> str:
    return value.removeprefix("parameter-").replace("_", "-")



def _validate_width_policies(values: dict[str, str]) -> dict[str, str]:
    if not isinstance(values, dict):
        raise BuildIRError("BUILD_IR_WIDTH_POLICY", "width_policies must be an object")
    allowed_roles = {"front_tire_width", "rear_tire_width"}
    allowed_policies = {"centered", "inboard_fixed", "outboard_fixed"}
    unknown = sorted(set(values) - allowed_roles)
    if unknown:
        raise BuildIRError("BUILD_IR_WIDTH_POLICY", f"unknown width policy role: {unknown[0]}")
    result = {role: values.get(role, "centered") for role in sorted(allowed_roles)}
    if any(policy not in allowed_policies for policy in result.values()):
        raise BuildIRError("BUILD_IR_WIDTH_POLICY", "unsupported tire width policy")
    return result
