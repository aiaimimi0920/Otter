"""固定 GN profile 的生成与拒绝合同；合成工具从不执行，真实 GN 门另行运行。"""

import json
import os
import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

from native_angle_fixture import fixture, write_json
from gn_profile import ANCHORS, PROFILE, gn_string, profile_files
from prepare_profile import prepare_profile as prepare
import prepare_profile
from toolchain_paths import file_hash


class GNProfileTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.angle = Path(os.environ["OTTER_ANGLE_SOURCE"])
        cls.build = Path(os.environ["OTTER_ANGLE_BUILD_SOURCE"])
        cls.originals = {name: (cls.angle / name if name == ".gn" else cls.build / name.removeprefix("build/")).read_bytes() for name in ANCHORS}

    def setUp(self):
        self.root, self.config = fixture()
        self.configuration = write_json(self.root / "config.json", self.config)
        self.output = self.root / "profile"

    def prepare(self):
        # 允许读取已经审阅的上游文件，但不能执行任何编译工具、GN 或发现脚本。
        with patch("subprocess.Popen", side_effect=AssertionError("Profile preparation executed a tool")):
            return prepare(self.angle, self.build, self.configuration, self.output)

    def test_fixed_clang_profile_has_no_generated_ninja_overlay_or_execution(self):
        receipt = self.prepare()
        self.assertEqual(receipt["profile"], PROFILE)
        for flag in ("toolsExecuted", "generatedNinjaPatched", "completeSourceClosure", "completeToolchainClosure", "runtimeActivated", "releaseApproved"):
            self.assertIs(receipt[flag], False)
        args = (self.output / "out/otter/args.gn").read_text("utf-8")
        for flag in ("is_clang = true", "use_lld = true", "angle_enable_d3d11 = true", "is_component_build = false", "symbol_level = 0"):
            self.assertIn(flag, args)
        self.assertNotIn("treat_warnings_as_errors", args)
        self.assertFalse(list(self.output.rglob("*.ninja")))

    def test_vs_sdk_and_static_crt_profile_guards_remain_explicit(self):
        self.prepare()
        vs = (self.output / "build/config/win/visual_studio_version.gni").read_text("utf-8")
        self.assertIn('visual_studio_runtime_dirs = "otter-no-runtime-dirs"', vs)
        self.assertIn('assert(wdk_path == ""', vs)
        self.assertNotIn("exec_script", vs)
        self.assertIn(gn_string(self.config["sdkRoot"]), vs)
        tools = (self.output / "build/toolchain/win/BUILD.gn").read_text("utf-8")
        self.assertIn("assert(!is_debug && !is_component_build && symbol_level == 0)", tools)
        self.assertIn("assert(is_clang && !use_custom_libcxx && use_lld)", tools)
        self.assertNotIn('"copy_dlls.py"', tools)

    def test_output_inventory_and_sources_are_bound_without_changing_inputs(self):
        receipt = self.prepare()
        for row in receipt["files"]:
            self.assertEqual(file_hash(self.output / row["path"]), row["sha256"])
        actual = {path.relative_to(self.output).as_posix() for path in self.output.rglob("*") if path.is_file()}
        self.assertEqual(actual, {"profile.json", "adapter.json", *(row["path"] for row in receipt["files"])})
        self.assertEqual(file_hash(self.output / "adapter.json"), receipt["adapterSha256"])
        self.assertEqual(receipt["python"]["sha256"], file_hash(Path(sys.executable).resolve()))
        self.assertEqual(receipt["inputSha256"], ANCHORS)

    def test_unknown_or_changed_upstream_identity_is_rejected(self):
        for name in ANCHORS:
            originals = {**self.originals, name: self.originals[name] + b"\n"}
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "identity mismatch"):
                profile_files(originals, self.config, sys.executable)
        with self.assertRaisesRegex(ValueError, "Unexpected"):
            profile_files({**self.originals, "extra": b""}, self.config, sys.executable)

    def test_bad_angle_source_fails_before_output_creation(self):
        angle = self.root / "wrong-angle"
        angle.mkdir()
        (angle / ".gn").write_bytes(self.originals[".gn"] + b"\n")
        with self.assertRaisesRegex(ValueError, "identity mismatch"):
            prepare(angle, self.build, self.configuration, self.output)
        self.assertFalse(self.output.exists())

    def test_single_architecture_cannot_masquerade_as_complete_gn_profile(self):
        self.config["architectures"] = ["x64"]
        write_json(Path(self.configuration), self.config)
        with self.assertRaisesRegex(ValueError, "both x86 and x64"):
            self.prepare()
        self.assertFalse(self.output.exists())

    def test_existing_output_and_angle_tree_are_not_overwritten(self):
        self.prepare()
        before = (self.output / "profile.json").read_bytes()
        with self.assertRaises(FileExistsError):
            self.prepare()
        self.assertEqual((self.output / "profile.json").read_bytes(), before)
        output = self.angle / "forbidden-profile"
        with self.assertRaisesRegex(ValueError, "protected ANGLE"):
            prepare(self.angle, self.build, self.configuration, output)
        self.assertFalse(output.exists())

    def test_gn_interpolation_and_config_injection_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "interpolation"):
            gn_string("C:/SDK/$root")
        self.config["sdkRoot"] += "$root"
        write_json(Path(self.configuration), self.config)
        with self.assertRaises(ValueError):
            self.prepare()
        self.assertFalse(self.output.exists())

    def test_late_input_change_keeps_failure_without_success_profile(self):
        count = 0

        def changing(path):
            nonlocal count
            if Path(path) == Path(self.configuration):
                count += 1
                if count > 1:
                    return "0" * 64
            return file_hash(path)

        with patch.object(prepare_profile, "file_hash", side_effect=changing):
            with self.assertRaisesRegex(ValueError, "inputs changed"):
                self.prepare()
        self.assertTrue((self.output / "FAILED.txt").is_file())
        self.assertFalse((self.output / "profile.json").exists())

    def test_fresh_cli_generates_profile_without_tool_execution(self):
        script = Path(prepare_profile.__file__)
        result = subprocess.run([sys.executable, "-I", "-B", str(script), "--angle-source", str(self.angle),
            "--build-source", str(self.build), "--config", self.configuration, "--output", str(self.output)],
            cwd=self.root, env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"}, capture_output=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", errors="replace"))
        self.assertEqual(json.loads(result.stdout)["profile"], PROFILE)


if __name__ == "__main__":
    unittest.main()
