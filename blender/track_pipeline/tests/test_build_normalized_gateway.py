"""Stage 4: Validate & Build gateway + normalized-JSON Blender integration tests.

These tests run without Blender: the gateway's Blender step is replaced by a
stub ``.cmd`` where a real compiler run would be too expensive, and the
atomicity/determinism contracts are exercised through the gateway's pure
functions and crafted build directories. The real Blender smoke is documented
and run manually.
"""

from __future__ import annotations

import hashlib
import io
import json
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))

import build_normalized_common as common
import build_svg_track as gateway
from build_normalized_common import (
    BUILD_MANIFEST_FILE,
    ENVIRONMENT_GLB,
    VEGETATION_GLB,
    build_sha256,
    compiler_sha256,
    glb_node_names,
    validate_runtime_split,
)
from build_svg_track import (
    ACTIVATION_FILE,
    GatewayError,
    activate_pair,
    gateway_main,
    plan_build,
    sha256_file,
    validate_build_dir,
)
from compile_svg_track import (
    CANONICAL_ARTIFACT,
    MANIFEST_ARTIFACT,
    NORMALIZED_ARTIFACT,
)
from asset_registry import load_registry

FIXTURES = ROOT / "blender" / "track_pipeline" / "tests" / "fixtures"
SOURCE = FIXTURES / "compile_track.svg"
REGISTRY_PATH = FIXTURES / "asset_registry_test.json"
VALID_ASSET_GLB = FIXTURES / "assets" / "valid_tree.glb"
BASE = SOURCE.read_text(encoding="utf-8")


def _rmtree(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)


def _make_glb(names: list[str]) -> bytes:
    """Craft a minimal valid GLB whose JSON chunk lists the given node names."""
    document = {
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": list(range(len(names)))}],
        "nodes": [{"name": name} for name in names],
    }
    body = json.dumps(document).encode("utf-8")
    body += b" " * ((-len(body)) % 4)
    total = 12 + 8 + len(body)
    header = struct.pack("<4sII", b"glTF", 2, total)
    return header + struct.pack("<I", len(body)) + b"JSON" + body


def _registry() -> object:
    return load_registry(REGISTRY_PATH, repo_root=ROOT)


def _plan(tmp: Path, *, source_text: str = BASE, dirname: str = "staged") -> tuple[dict, Path]:
    out = tmp / dirname
    plan = plan_build(
        source_text.encode("utf-8"),
        source_name=SOURCE.name,
        registry=_registry(),
        registry_path=REGISTRY_PATH,
        output_dir=out,
    )
    return plan, out


def _run_gateway(args: list[str]) -> tuple[int, str]:
    out, err = io.StringIO(), io.StringIO()
    with redirect_stdout(out), redirect_stderr(err):
        rc = gateway_main(args)
    return rc, err.getvalue()


def _stub(tmp: Path, name: str, *, exit_code: int = 0, marker: Path | None = None) -> Path:
    path = tmp / name
    lines = ["@echo off"]
    if marker is not None:
        lines.append(f'echo invoked>> "{marker}"')
    lines.append(f"exit /b {exit_code}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return path


def _craft_valid_build(
    tmp: Path,
    builds_root: Path,
    *,
    source_text: str = BASE,
) -> tuple[Path, dict]:
    """Build a complete, validated ``builds/<build_sha>/`` without running Blender."""
    plan, staged = _plan(tmp, source_text=source_text)
    build_dir = builds_root / plan["build_sha"]
    shutil.copytree(staged, build_dir)
    (build_dir / "track.blend").write_bytes(b"crafted-blend-bytes")
    collision_names = {"GrassTerrainCollision-colonly"}
    asset_names = {"Asset_tree_a_tree_v2_01"}
    (build_dir / ENVIRONMENT_GLB).write_bytes(_make_glb(sorted(collision_names)))
    (build_dir / VEGETATION_GLB).write_bytes(_make_glb(sorted(asset_names)))

    def entry(name: str) -> dict:
        return {"file": name, "sha256": sha256_file(build_dir / name)}

    artifacts = {
        name: entry(name)
        for name in (
            CANONICAL_ARTIFACT,
            NORMALIZED_ARTIFACT,
            MANIFEST_ARTIFACT,
            "track.blend",
            ENVIRONMENT_GLB,
            VEGETATION_GLB,
        )
    }
    manifest = {
        "schema_version": 1,
        "build_sha256": plan["build_sha"],
        "track_id": plan["manifest"]["track_id"],
        "inputs": {
            "source": {"file": "track.svg", "sha256": plan["source_sha"]},
            "registry": {"file": REGISTRY_PATH.name, "sha256": plan["registry_sha"]},
            "normalized": {"file": NORMALIZED_ARTIFACT, "sha256": plan["normalized_sha"]},
            "compiler": {"name": "test", "version": "1.0.0", "sha256": plan["compiler_sha"]},
        },
        "artifacts": artifacts,
        "blender_version": "test",
        "stats": {
            "terrain_vertices": 0,
            "terrain_triangles": 0,
            "terrain_cell_m": 6.0,
            "barrier_segments": 1,
            "asset_instances": 2,
            "environment_objects": 4,
            "vegetation_objects": 2,
            "collision_proxies": len(collision_names),
            "collision_proxy_names": sorted(collision_names),
            "asset_root_names": sorted(asset_names),
        },
    }
    (build_dir / BUILD_MANIFEST_FILE).write_text(
        json.dumps(manifest, indent=2, sort_keys=True), encoding="utf-8"
    )
    return build_dir, plan


class BuildNormalizedCommonTests(unittest.TestCase):
    def test_compiler_sha256_is_stable_and_hex(self):
        self.assertEqual(len(compiler_sha256()), 64)
        int(compiler_sha256(), 16)
        self.assertEqual(compiler_sha256(), compiler_sha256())

    def test_build_sha256_is_deterministic(self):
        a = build_sha256("a" * 64, "b" * 64, "c" * 64, "d" * 64)
        b = build_sha256("a" * 64, "b" * 64, "c" * 64, "d" * 64)
        c = build_sha256("a" * 64, "b" * 64, "c" * 64, "e" * 64)
        self.assertEqual(a, b)
        self.assertNotEqual(a, c)

    def test_glb_node_names_parses_fixture(self):
        self.assertEqual(glb_node_names(VALID_ASSET_GLB), {"Cube"})

    def test_glb_node_names_rejects_bad_magic(self):
        bad = Path(tempfile.mkdtemp()) / "bad.glb"
        bad.write_bytes(b"NOTAGLB")
        self.addCleanup(_rmtree, bad.parent)
        with self.assertRaises(common.BuildManifestError):
            glb_node_names(bad)

    def test_validate_runtime_split_enforces_contract(self):
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(_rmtree, tmp)
        env = tmp / "env.glb"
        veg = tmp / "veg.glb"
        env.write_bytes(_make_glb(["A-colonly", "B-colonly", "C"]))
        veg.write_bytes(_make_glb(["V1", "V2"]))
        validate_runtime_split(env, veg, {"A-colonly", "B-colonly"}, {"V1", "V2"})
        with self.assertRaises(common.BuildManifestError):
            validate_runtime_split(env, veg, {"Missing-colonly"}, set())
        validate_runtime_split(env, veg, {"A-colonly"}, {"V1"})
        with self.assertRaises(common.BuildManifestError):
            validate_runtime_split(env, veg, {"A-colonly"}, {"NotThere"})
        veg.write_bytes(_make_glb(["V1", "A-colonly"]))
        with self.assertRaises(common.BuildManifestError):
            validate_runtime_split(env, veg, {"A-colonly"}, set())


class GatewayPlanTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="f90_gateway_"))
        self.addCleanup(_rmtree, self.tmp)

    def test_plan_build_deterministic(self):
        plan1, out1 = _plan(self.tmp, dirname="one")
        plan2, out2 = _plan(self.tmp, dirname="two")
        self.assertEqual(plan1["build_sha"], plan2["build_sha"])
        self.assertEqual(plan1["normalized_sha"], plan2["normalized_sha"])
        self.assertEqual(len(plan1["build_sha"]), 64)
        self.assertEqual(
            (out1 / NORMALIZED_ARTIFACT).read_bytes(),
            (out2 / NORMALIZED_ARTIFACT).read_bytes(),
        )

    def test_normalized_includes_track_config(self):
        _, out = _plan(self.tmp)
        normalized = json.loads((out / NORMALIZED_ARTIFACT).read_text(encoding="utf-8"))
        config = normalized["track_config"]
        self.assertEqual(config["road"]["width_m"], 12.0)
        self.assertEqual(config["road"]["surface_elevation_m"], 0.025)
        self.assertGreater(len(config["banking"]), 0)
        self.assertGreater(config["terrain"]["grid_cell_m"], 0)

    def test_plan_build_rejects_invalid_source(self):
        broken = BASE.replace("Z", "")
        with self.assertRaises(GatewayError) as ctx:
            plan_build(
                broken.encode("utf-8"),
                source_name="track.svg",
                registry=_registry(),
                registry_path=REGISTRY_PATH,
                output_dir=self.tmp / "bad",
            )
        self.assertIn("does not compile", str(ctx.exception))

    def test_plan_build_rejects_unknown_asset_registry(self):
        bad_registry_path = self.tmp / "bad_registry.json"
        bad_registry_path.write_text(
            json.dumps({
                "schema_version": 1,
                "assets": [{
                    "id": "ghost_01",
                    "kind": "vegetation",
                    "source": "blender/track_pipeline/tests/fixtures/assets/missing.glb",
                }],
            }),
            encoding="utf-8",
        )
        registry = load_registry(bad_registry_path, repo_root=ROOT)
        with self.assertRaises(GatewayError):
            plan_build(
                BASE.encode("utf-8"),
                source_name="track.svg",
                registry=registry,
                registry_path=bad_registry_path,
                output_dir=self.tmp / "bad2",
            )

    def test_build_sha_changes_when_source_changes(self):
        plan1, _ = _plan(self.tmp, source_text=BASE)
        variant = BASE.replace('data-degrees="3.5"', 'data-degrees="2.0"')
        plan2, _ = _plan(self.tmp, source_text=variant, dirname="staged2")
        self.assertNotEqual(plan1["build_sha"], plan2["build_sha"])


class GatewayMainTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="f90_gateway_main_"))
        self.addCleanup(_rmtree, self.tmp)
        self.root = self.tmp / "root"

    def _args(self, **overrides) -> list[str]:
        values = {
            "--source": str(SOURCE),
            "--registry": str(REGISTRY_PATH),
            "--output-root": str(self.root),
            "--blender-exe": str(_stub(self.tmp, "default_blender.cmd")),
        }
        values.update(overrides)
        args = []
        for key, value in values.items():
            if value is False:
                continue
            args.append(key)
            if value is not True:
                args.append(str(value))
        return args

    def test_reject_without_approved(self):
        rc, err = _run_gateway(self._args())
        self.assertEqual(rc, 2)
        self.assertIn("--approved", err)

    def test_reject_invalid_source(self):
        broken = self.tmp / "broken.svg"
        broken.write_text(BASE.replace("Z", ""), encoding="utf-8")
        rc, err = _run_gateway(self._args(**{"--source": str(broken), "--approved": True}))
        self.assertEqual(rc, 2)
        self.assertIn("source does not compile", err)

    def test_reject_invalid_registry(self):
        bad = self.tmp / "bad_registry.json"
        bad.write_text(
            json.dumps({
                "schema_version": 1,
                "assets": [{
                    "id": "ghost_01",
                    "kind": "vegetation",
                    "source": "blender/track_pipeline/tests/fixtures/assets/missing.glb",
                }],
            }),
            encoding="utf-8",
        )
        rc, err = _run_gateway(self._args(**{"--registry": str(bad), "--approved": True}))
        self.assertEqual(rc, 2)
        self.assertIn("registry invalid", err)

    def test_reject_blender_not_found(self):
        rc, err = _run_gateway(
            self._args(**{"--blender-exe": str(self.tmp / "missing.exe"), "--approved": True})
        )
        self.assertEqual(rc, 2)
        self.assertIn("blender not found", err)

    def test_reject_hash_mismatch_against_existing_build(self):
        plan, staged = _plan(self.tmp)
        build_dir = self.root / "builds" / plan["build_sha"]
        build_dir.mkdir(parents=True)
        shutil.copytree(staged, build_dir, dirs_exist_ok=True)
        (build_dir / NORMALIZED_ARTIFACT).write_bytes(b"TAMPERED")
        marker = self.tmp / "invoked.txt"
        blender = _stub(self.tmp, "blender.cmd", marker=marker)
        rc, err = _run_gateway(
            self._args(**{"--blender-exe": str(blender), "--approved": True})
        )
        self.assertEqual(rc, 2)
        self.assertIn("hash mismatch", err)
        self.assertFalse(marker.exists(), "Blender must not run after hash mismatch")

    def test_reject_source_changed_between_checks(self):
        real = gateway.compile_track
        calls = {"n": 0}

        def dirty(source_bytes, **kwargs):
            calls["n"] += 1
            manifest = real(source_bytes, **kwargs)
            if calls["n"] == 2:
                target = Path(kwargs["output_dir"]) / NORMALIZED_ARTIFACT
                target.write_bytes(target.read_bytes() + b"\n")
            return manifest

        gateway.compile_track = dirty
        try:
            rc, err = _run_gateway(self._args(**{"--approved": True}))
        finally:
            gateway.compile_track = real
        self.assertEqual(rc, 2)
        self.assertIn("not byte-identical", err)

    def test_reject_blender_failure_keeps_builds_untouched(self):
        blender = _stub(self.tmp, "blender.cmd", exit_code=1)
        rc, err = _run_gateway(self._args(**{"--blender-exe": str(blender), "--approved": True}))
        self.assertEqual(rc, 2)
        self.assertIn("blender failed", err)
        builds = self.root / "builds"
        if builds.exists():
            entries = [p.name for p in builds.iterdir() if p.is_dir() and p.name != ".staging"]
            self.assertEqual(entries, [], "no build dir may exist after a failed Blender run")
        self.assertFalse((self.root / "builds" / "_state.json").exists())

    def test_reject_blender_output_validation_failure(self):
        blender = _stub(self.tmp, "blender.cmd", exit_code=0)
        rc, err = _run_gateway(self._args(**{"--blender-exe": str(blender), "--approved": True}))
        self.assertEqual(rc, 2)
        self.assertIn("build validation failed", err)

    def test_reuses_validated_existing_build_without_blender(self):
        builds_root = self.root / "builds"
        builds_root.mkdir(parents=True)
        build_dir, plan = _craft_valid_build(self.tmp, builds_root)
        self.assertTrue(build_dir.is_dir())
        marker = self.tmp / "invoked.txt"
        blender = _stub(self.tmp, "blender.cmd", marker=marker)
        runtime = self.tmp / "runtime"
        rc, err = _run_gateway(
            self._args(
                **{
                    "--blender-exe": str(blender),
                    "--approved": True,
                    "--activate": True,
                    "--runtime-root": str(runtime),
                }
            )
        )
        self.assertEqual(rc, 0, err)
        self.assertFalse(marker.exists(), "Blender must not re-run for an identical build")
        self.assertEqual(
            (runtime / "compile_fixture.glb").read_bytes(),
            (build_dir / ENVIRONMENT_GLB).read_bytes(),
        )
        activation = json.loads((runtime / ACTIVATION_FILE).read_text(encoding="utf-8"))
        self.assertEqual(activation["build_sha256"], plan["build_sha"])

    def test_failed_candidate_never_replaces_active_build(self):
        runtime = self.tmp / "runtime"
        build_a = self.tmp / "build_a"
        build_a.mkdir()
        env_a, veg_a = b"ENV-ACTIVE", b"VEG-ACTIVE"
        (build_a / ENVIRONMENT_GLB).write_bytes(env_a)
        (build_a / VEGETATION_GLB).write_bytes(veg_a)
        sha_a = "a" * 64
        activate_pair(build_a, runtime, "compile_fixture", sha_a)
        self.assertEqual((runtime / "compile_fixture.glb").read_bytes(), env_a)

        variant = self.tmp / "variant.svg"
        variant.write_text(BASE.replace('data-degrees="3.5"', 'data-degrees="2.0"'), encoding="utf-8")
        blender = _stub(self.tmp, "blender.cmd", exit_code=1)
        rc, err = _run_gateway(
            [
                "--source", str(variant),
                "--registry", str(REGISTRY_PATH),
                "--output-root", str(self.tmp / "root2"),
                "--approved",
                "--activate",
                "--runtime-root", str(runtime),
                "--blender-exe", str(blender),
            ]
        )
        self.assertEqual(rc, 2)
        self.assertEqual((runtime / "compile_fixture.glb").read_bytes(), env_a)
        self.assertEqual((runtime / "compile_fixture_vegetation.glb").read_bytes(), veg_a)
        activation = json.loads((runtime / ACTIVATION_FILE).read_text(encoding="utf-8"))
        self.assertEqual(activation["build_sha256"], sha_a)

    def test_validate_build_dir_rejects_wrong_build_sha(self):
        builds_root = self.root / "builds"
        builds_root.mkdir(parents=True)
        build_dir, plan = _craft_valid_build(self.tmp, builds_root)
        manifest = json.loads((build_dir / BUILD_MANIFEST_FILE).read_text(encoding="utf-8"))
        manifest["build_sha256"] = "0" * 64
        (build_dir / BUILD_MANIFEST_FILE).write_text(
            json.dumps(manifest), encoding="utf-8"
        )
        with self.assertRaises(GatewayError):
            validate_build_dir(build_dir, plan["build_sha"], plan["manifest"]["track_id"])


class ActivationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="f90_activation_"))
        self.addCleanup(_rmtree, self.tmp)

    def _build(self, name: str, *, env: bytes, veg: bytes) -> Path:
        directory = self.tmp / name
        directory.mkdir()
        (directory / ENVIRONMENT_GLB).write_bytes(env)
        (directory / VEGETATION_GLB).write_bytes(veg)
        return directory

    def test_activation_atomic_success_and_replace(self):
        runtime = self.tmp / "runtime"
        a = self._build("build_a", env=b"ENV-A", veg=b"VEG-A")
        b = self._build("build_b", env=b"ENV-B", veg=b"VEG-B")
        activate_pair(a, runtime, "compile_fixture", "a" * 64)
        self.assertEqual((runtime / "compile_fixture.glb").read_bytes(), b"ENV-A")
        self.assertEqual((runtime / "compile_fixture_vegetation.glb").read_bytes(), b"VEG-A")
        activation = json.loads((runtime / ACTIVATION_FILE).read_text(encoding="utf-8"))
        self.assertEqual(activation["build_sha256"], "a" * 64)
        self.assertEqual(len(activation["members"]), 2)
        self.assertEqual(activation["members"][0]["runtime_file"], "compile_fixture.glb")

        activate_pair(b, runtime, "compile_fixture", "b" * 64)
        self.assertEqual((runtime / "compile_fixture.glb").read_bytes(), b"ENV-B")
        activation = json.loads((runtime / ACTIVATION_FILE).read_text(encoding="utf-8"))
        self.assertEqual(activation["build_sha256"], "b" * 64)
        leftovers = [p.name for p in runtime.iterdir() if ".activation-" in p.name or p.name.endswith(".prev")]
        self.assertEqual(leftovers, [])

    def test_activation_failure_rolls_back_previous_pair(self):
        runtime = self.tmp / "runtime"
        a = self._build("build_a", env=b"ENV-A", veg=b"VEG-A")
        b = self._build("build_b", env=b"ENV-B", veg=b"VEG-B")
        activate_pair(a, runtime, "compile_fixture", "a" * 64)

        original_replace = gateway.os.replace
        calls = {"n": 0}

        def flaky_replace(source, target):
            calls["n"] += 1
            if calls["n"] == 3:
                raise OSError("simulated member install failure")
            return original_replace(source, target)

        gateway.os.replace = flaky_replace
        try:
            with self.assertRaises(OSError):
                activate_pair(b, runtime, "compile_fixture", "b" * 64)
        finally:
            gateway.os.replace = original_replace

        self.assertEqual((runtime / "compile_fixture.glb").read_bytes(), b"ENV-A")
        self.assertEqual((runtime / "compile_fixture_vegetation.glb").read_bytes(), b"VEG-A")
        activation = json.loads((runtime / ACTIVATION_FILE).read_text(encoding="utf-8"))
        self.assertEqual(activation["build_sha256"], "a" * 64)
        leftovers = [p.name for p in runtime.iterdir() if ".activation-" in p.name or p.name.endswith(".prev")]
        self.assertEqual(leftovers, [])


if __name__ == "__main__":
    unittest.main()
