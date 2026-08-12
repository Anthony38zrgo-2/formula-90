"""Stage 5: headless Godot-load validation wrapper and gateway gate tests.

Unit tests never launch a real Godot binary: ``subprocess.run`` is mocked and
the gateway's ``--godot`` gate is exercised with the validator patched, so the
suite stays fast and hermetic. The real Godot smoke (PASS on a fresh build,
FAIL on missing/corrupt GLBs) is documented in ``validate_godot_load.py`` and
run manually against temporary runtime roots.
"""

from __future__ import annotations

import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))

import build_svg_track as gateway
import validate_godot_load as vgl
from build_normalized_common import ENVIRONMENT_GLB, VEGETATION_GLB
from validate_godot_load import (
    EXIT_GODOT_NOT_FOUND,
    EXIT_OK,
    VALIDATION_SCRIPT,
    GodotNotFoundError,
    build_validation_command,
    find_godot_exe,
    main,
    resolve_godot_exe,
    run_godot_load_validation,
)
from test_build_normalized_gateway import (
    REGISTRY_PATH,
    SOURCE,
    _craft_valid_build,
    _rmtree,
)


def _fake_godot(directory: Path, name: str = "Godot_v4.7.1-stable_win64_console.exe") -> Path:
    path = directory / name
    path.write_bytes(b"fake godot")
    return path


class GodotResolutionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="f90_vgl_resolve_"))
        self.addCleanup(_rmtree, self.tmp)

    def test_search_prefers_console_build(self):
        console = _fake_godot(self.tmp)
        non_console = _fake_godot(self.tmp, "Godot_v4.7.1-stable_win64.exe")
        self.assertEqual(vgl._search_godot_dir(self.tmp), console.resolve())
        self.assertNotEqual(console, non_console)

    def test_search_returns_none_for_empty_dir(self):
        self.assertIsNone(vgl._search_godot_dir(self.tmp))

    def test_search_returns_any_exe_when_no_console(self):
        _fake_godot(self.tmp, "Godot_v4.7.1-stable_win64.exe")
        self.assertEqual(vgl._search_godot_dir(self.tmp).name, "Godot_v4.7.1-stable_win64.exe")

    def test_search_ignores_missing_dir(self):
        self.assertIsNone(vgl._search_godot_dir(self.tmp / "missing"))

    def test_resolve_explicit_path(self):
        godot = _fake_godot(self.tmp)
        self.assertEqual(resolve_godot_exe(godot), godot.resolve())

    def test_resolve_missing_path_raises(self):
        missing = self.tmp / "missing.exe"
        with self.assertRaises(GodotNotFoundError):
            resolve_godot_exe(missing)

    def test_find_godot_exe_real_repo(self):
        tools = ROOT / ".tools" / "godot"
        if not tools.is_dir():
            self.skipTest(".tools/godot not present in this checkout")
        resolved = find_godot_exe()
        self.assertIsNotNone(resolved)
        self.assertTrue(resolved.is_file())
        self.assertIn("console", resolved.name.lower())


class CommandBuildTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="f90_vgl_cmd_"))
        self.addCleanup(_rmtree, self.tmp)

    def test_command_contains_headless_script_and_user_args(self):
        godot = _fake_godot(self.tmp)
        env = self.tmp / "env.glb"
        veg = self.tmp / "veg.glb"
        env.write_bytes(b"x")
        veg.write_bytes(b"x")
        command = build_validation_command(
            env, veg, godot=godot, project=self.tmp, track_id="compile_fixture"
        )
        self.assertEqual(command[0], str(godot.resolve()))
        self.assertIn("--headless", command)
        self.assertEqual(command[command.index("--path") + 1], str(self.tmp))
        self.assertEqual(command[command.index("--script") + 1], VALIDATION_SCRIPT)
        separator = command.index("--")
        user_args = command[separator + 1:]
        self.assertEqual(user_args[0], "--env-glb")
        self.assertEqual(user_args[1], str(env.resolve()))
        self.assertEqual(user_args[2], "--veg-glb")
        self.assertEqual(user_args[3], str(veg.resolve()))
        self.assertIn("--track-id", user_args)

    def test_command_omits_track_id_when_absent(self):
        godot = _fake_godot(self.tmp)
        command = build_validation_command(
            self.tmp / "env.glb", self.tmp / "veg.glb", godot=godot, project=self.tmp
        )
        self.assertNotIn("--track-id", command)


class RunValidationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="f90_vgl_run_"))
        self.addCleanup(_rmtree, self.tmp)
        self.godot = _fake_godot(self.tmp)

    def _run(self, returncode: int, stdout: str = "") -> int:
        fake = mock.Mock(returncode=returncode, stdout=stdout, stderr="")
        out, err = io.StringIO(), io.StringIO()
        with mock.patch.object(vgl.subprocess, "run", return_value=fake) as run_mock:
            with redirect_stdout(out), redirect_stderr(err):
                rc = run_godot_load_validation(
                    self.tmp / "env.glb",
                    self.tmp / "veg.glb",
                    godot_exe=self.godot,
                    project=self.tmp,
                )
        return rc, run_mock, out.getvalue()

    def test_success_propagates_zero_and_relays_report(self):
        rc, run_mock, output = self._run(0, "RESULT: PASS\n")
        self.assertEqual(rc, EXIT_OK)
        self.assertIn("RESULT: PASS", output)
        run_mock.assert_called_once()

    def test_failure_propagates_one(self):
        rc, _, _ = self._run(1, "RESULT: FAIL (1 invariant(s) failed)\n")
        self.assertEqual(rc, 1)

    def test_failure_relayed_even_for_missing_glbs(self):
        rc, run_mock, _ = self._run(1)
        self.assertEqual(rc, 1)
        env_arg = run_mock.call_args.args[0]
        self.assertIn(str((self.tmp / "env.glb").resolve()), env_arg)

    def test_main_missing_godot_returns_3(self):
        env = self.tmp / "env.glb"
        env.write_bytes(b"x")
        rc = main([
            "--env-glb", str(env),
            "--veg-glb", str(self.tmp / "veg.glb"),
            "--godot", str(self.tmp / "nope.exe"),
        ])
        self.assertEqual(rc, EXIT_GODOT_NOT_FOUND)

    def test_main_default_godot_auto_detects_but_fails_without_tools(self):
        env = self.tmp / "env.glb"
        env.write_bytes(b"x")
        fake = mock.Mock(returncode=0, stdout="RESULT: PASS\n", stderr="")
        with mock.patch.object(vgl.subprocess, "run", return_value=fake):
            with mock.patch.object(
                vgl, "find_godot_exe", return_value=None
            ) as find_mock, mock.patch.object(vgl.shutil, "which", return_value=None):
                rc = main(["--env-glb", str(env), "--veg-glb", str(self.tmp / "veg.glb")])
        self.assertEqual(rc, EXIT_GODOT_NOT_FOUND)
        find_mock.assert_called_once()


class GatewayGodotGateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="f90_gateway_godot_"))
        self.addCleanup(_rmtree, self.tmp)
        self.root = self.tmp / "root"
        self.runtime = self.tmp / "runtime"

    def _blender_stub(self) -> Path:
        path = self.tmp / "blender.cmd"
        path.write_text("@echo off\nexit /b 0\n", encoding="utf-8")
        return path

    def _base_args(self) -> list[str]:
        return [
            "--source", str(SOURCE),
            "--registry", str(REGISTRY_PATH),
            "--output-root", str(self.root),
            "--blender-exe", str(self._blender_stub()),
            "--approved",
            "--activate",
            "--runtime-root", str(self.runtime),
            "--godot", str(_fake_godot(self.tmp, "gate_godot.exe")),
        ]

    def _run(self, args: list[str]) -> tuple[int, str]:
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            rc = gateway.gateway_main(args)
        return rc, err.getvalue()

    def test_gate_passes_then_activates(self):
        builds_root = self.root / "builds"
        builds_root.mkdir(parents=True)
        build_dir, plan = _craft_valid_build(self.tmp, builds_root)
        with mock.patch.object(
            gateway, "run_godot_load_validation", return_value=0
        ) as gate:
            rc, err = self._run(self._base_args())
        self.assertEqual(rc, 0, err)
        gate.assert_called_once()
        args = gate.call_args.args
        self.assertEqual(Path(args[0]), build_dir / ENVIRONMENT_GLB)
        self.assertEqual(Path(args[1]), build_dir / VEGETATION_GLB)
        self.assertEqual((self.runtime / "compile_fixture.glb").read_bytes(),
                         (build_dir / ENVIRONMENT_GLB).read_bytes())

    def test_gate_failure_blocks_activation(self):
        builds_root = self.root / "builds"
        builds_root.mkdir(parents=True)
        _craft_valid_build(self.tmp, builds_root)
        with mock.patch.object(
            gateway, "run_godot_load_validation", return_value=1
        ):
            rc, err = self._run(self._base_args())
        self.assertEqual(rc, 2)
        self.assertIn("godot-load validation failed", err)
        if self.runtime.exists():
            entries = [p.name for p in self.runtime.iterdir()]
            self.assertEqual(entries, [], "no runtime member may exist after a failed gate")

    def test_gate_failure_never_replaces_active_build(self):
        builds_root = self.root / "builds"
        builds_root.mkdir(parents=True)
        build_a, plan_a = _craft_valid_build(self.tmp, builds_root)
        with mock.patch.object(gateway, "run_godot_load_validation", return_value=0):
            rc, _ = self._run(self._base_args())
        self.assertEqual(rc, 0)
        active_env = (self.runtime / "compile_fixture.glb").read_bytes()
        self.assertEqual(active_env, (build_a / ENVIRONMENT_GLB).read_bytes())

        variant = self.tmp / "variant.svg"
        variant.write_text(SOURCE.read_text(encoding="utf-8").replace(
            'data-degrees="3.5"', 'data-degrees="2.0"'
        ), encoding="utf-8")
        build_b, plan_b = _craft_valid_build(
            self.tmp, builds_root, source_text=variant.read_text(encoding="utf-8")
        )
        self.assertNotEqual(plan_a["build_sha"], plan_b["build_sha"])
        with mock.patch.object(gateway, "run_godot_load_validation", return_value=1):
            rc, err = self._run([
                "--source", str(variant),
                "--registry", str(REGISTRY_PATH),
                "--output-root", str(self.root),
                "--blender-exe", str(self._blender_stub()),
                "--approved",
                "--activate",
                "--runtime-root", str(self.runtime),
                "--godot", str(_fake_godot(self.tmp, "gate_godot.exe")),
            ])
        self.assertEqual(rc, 2)
        self.assertEqual((self.runtime / "compile_fixture.glb").read_bytes(), active_env)
        activation = json.loads((self.runtime / "activation.json").read_text(encoding="utf-8"))
        self.assertEqual(activation["build_sha256"], plan_a["build_sha"])

    def test_gate_missing_godot_rejects(self):
        builds_root = self.root / "builds"
        builds_root.mkdir(parents=True)
        _craft_valid_build(self.tmp, builds_root)
        with mock.patch.object(
            gateway, "run_godot_load_validation",
            side_effect=GodotNotFoundError("no godot"),
        ):
            rc, err = self._run(self._base_args())
        self.assertEqual(rc, 2)
        self.assertIn("no godot", err)
        self.assertFalse(self.runtime.exists())

    def test_gate_skipped_without_godot_flag(self):
        builds_root = self.root / "builds"
        builds_root.mkdir(parents=True)
        build_dir, plan = _craft_valid_build(self.tmp, builds_root)
        args = self._base_args()
        args.remove("--godot")
        args.pop()  # the godot value
        with mock.patch.object(
            gateway, "run_godot_load_validation", return_value=0
        ) as gate:
            rc, err = self._run(args)
        self.assertEqual(rc, 0, err)
        gate.assert_not_called()
        self.assertEqual((self.runtime / "compile_fixture.glb").read_bytes(),
                         (build_dir / ENVIRONMENT_GLB).read_bytes())


if __name__ == "__main__":
    unittest.main()
