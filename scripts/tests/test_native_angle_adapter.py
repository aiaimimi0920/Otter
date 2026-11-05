"""固定上游 setup_toolchain main 的真实消费测试；阻断全部进程发现/执行。"""

import contextlib
import importlib.util
import io
import json
import os
import re
import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

from native_angle_fixture import fixture, write_json
from prepare_toolchain import GN_HELPERS_SHA256, SETUP_PATH, SETUP_SHA256, adapted_setup, prepare
import prepare_toolchain
from toolchain_paths import file_hash


class UpstreamAdapterTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        value = os.environ.get("OTTER_ANGLE_BUILD_SOURCE")
        if not value:
            raise ValueError("Set OTTER_ANGLE_BUILD_SOURCE to the pinned, reviewed build checkout")
        cls.source = Path(value)
        if file_hash(cls.source / SETUP_PATH) != SETUP_SHA256 or file_hash(cls.source / "gn_helpers.py") != GN_HELPERS_SHA256:
            raise ValueError("Pinned upstream identity mismatch")

    def setUp(self):
        self.root, self.config = fixture()
        self.config_file = write_json(self.root / "config.json", self.config)
        self.output = self.root / "adapter"
        self.receipt = prepare(self.source, self.config_file, self.output)
        self.manifest = self.output / "environment.json"

    def module(self):
        script = self.source / SETUP_PATH if os.environ.get("OTTER_ANGLE_TEST_UPSTREAM_BASELINE") == "1" else (
            self.output / "build/toolchain/win/setup_toolchain.py"
        )
        spec = importlib.util.spec_from_file_location("otter_setup_contract", script)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module, sys.modules["gn_helpers"]

    def invoke(self, cpu="x64", runtime_dirs="", filename="none", target="win", manifest_path=None):
        arguments = ["setup_toolchain.py", self.config["visualStudioRoot"], self.config["sdkRoot"], runtime_dirs, target, cpu, filename]
        environment = {
            "DEPOT_TOOLS_WIN_TOOLCHAIN": "0", "OTTER_ANGLE_TOOLCHAIN_MANIFEST": (
                str(self.manifest) if manifest_path is None else manifest_path
            ),
            "GYP_MSVS_OVERRIDE_PATH": self.config["visualStudioRoot"],
            "INCLUDE": "OLD-SDK", "LIB": "OLD-SDK", "LIBPATH": "OLD-SDK", "PATH": "OLD-SDK",
        }
        stdout = io.StringIO()
        # 从生成目录实际导入，不预注入原始 checkout 的 helper 或仓库内 adapter。
        with patch.dict(sys.modules), patch.object(sys, "path", [
            str(self.output / "build/toolchain/win"), str(self.output / "build"), *sys.path
        ]):
            for name in ("gn_helpers", "toolchain_paths", "otter_toolchain_env"):
                sys.modules.pop(name, None)
            module, helper = self.module()
            with patch.dict(os.environ, environment), patch.object(sys, "argv", arguments), contextlib.redirect_stdout(stdout), (
                patch.object(module, "_LoadEnvFromBat", side_effect=AssertionError("External toolchain discovery attempted"))
            ), patch("subprocess.Popen", side_effect=AssertionError("External process execution attempted")):
                module.main()
            self.assertEqual(Path(helper.__file__), self.output / "build/gn_helpers.py")
            self.assertEqual(Path(sys.modules["otter_toolchain_env"].__file__), self.output / "build/toolchain/win/otter_toolchain_env.py")
            return helper.FromGNArgs(stdout.getvalue()), stdout.getvalue()

    def test_real_main_emits_explicit_gn_flags_without_discovery_or_tools(self):
        for cpu in ("x86", "x64"):
            with self.subTest(cpu=cpu):
                result, stdout = self.invoke(cpu)
                self.assertEqual(set(result), {"vc_bin_dir", "include_flags_I", "include_flags_I_list", "include_flags_imsvc",
                    "include_flags_imsvc_list", "paths", "libpath_flags", "libpath_flags_list", "libpath_lldlink_flags",
                    "libpath_lldlink_flags_list", "msvc_dir", "windows_kits_dir"})
                self.assertNotIn("OLD-SDK", stdout)
                self.assertNotIn("winsysroot", stdout)
                self.assertEqual(Path(result["vc_bin_dir"]), Path(self.config["visualStudioRoot"]) / "VC/Tools/MSVC/14.44.35207/bin/Hostx64" / cpu)
                # 上游 FromGNArgs 保留 GN 的 UTF-8 $0xNN 转义；不能据此把中文路径改成 ASCII。
                sdk_include = re.sub(r"(?:\$0x[0-9A-F]{2})+", lambda match: bytes(
                    int(value, 16) for value in re.findall(r"\$0x([0-9A-F]{2})", match[0])
                ).decode("utf-8"), result["windows_kits_dir"])
                self.assertEqual(Path(sdk_include).resolve(), Path(self.config["sdkRoot"]) / "Include/10.0.26100.0")
                self.assertEqual(len(result["include_flags_imsvc_list"]), 5)
                self.assertEqual(len(result["libpath_flags_list"]), 3)

    def test_main_produces_real_nul_terminated_environment_block(self):
        # 上游写入由本测试明确拥有的 linshi 文件，不运行环境块中的任何 EXE。
        path = self.root / "environment.x64"
        self.invoke(filename=str(path))
        block = path.read_bytes().decode("utf-8")
        self.assertTrue(block.endswith("\0\0"))
        env = dict(item.split("=", 1) for item in block.rstrip("\0").split("\0"))
        self.assertEqual(env["TEMP"], self.config["tempRoot"])
        self.assertEqual(env["VSLANG"], "1033")
        self.assertNotIn("OLD-SDK", block)
        self.assertNotIn("10.0.22621.0", block)

    def test_explicit_gn_runtime_marker_matches_a_real_empty_argument(self):
        expected, _ = self.invoke()
        actual, stdout = self.invoke(runtime_dirs="otter-no-runtime-dirs")
        self.assertEqual(actual, expected)
        self.assertNotIn("otter-no-runtime-dirs", stdout)

    def test_nonempty_gn_marker_survives_a_fresh_python_cli_argument_list(self):
        script = self.output / "build/toolchain/win/setup_toolchain.py"
        path = self.root / "marker.environment"
        environment = {**os.environ, "DEPOT_TOOLS_WIN_TOOLCHAIN": "0",
            "OTTER_ANGLE_TOOLCHAIN_MANIFEST": str(self.manifest)}
        result = subprocess.run([sys.executable, "-B", str(script), self.config["visualStudioRoot"],
            self.config["sdkRoot"], "otter-no-runtime-dirs", "win", "x64", str(path)],
            env=environment, cwd=self.root, capture_output=True, timeout=15, check=False)
        self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8"))
        block = path.read_bytes().decode("utf-8")
        self.assertTrue(block.endswith("\0\0"))
        self.assertNotIn("otter-no-runtime-dirs", block)

    def test_main_rejects_external_runtime_path_and_store(self):
        for value in ("OLD-SDK", "otter-no-runtime-dirs ", "otter-no-runtime-dirs;OLD-SDK"):
            with self.subTest(runtime_dirs=value), self.assertRaisesRegex(ValueError, "runtime_dirs"):
                self.invoke(runtime_dirs=value)
        with self.assertRaisesRegex(ValueError, "Store"):
            self.invoke(target="winuwp")
        with self.assertRaisesRegex(ValueError, "discovery is disabled"):
            self.invoke(manifest_path="")

    def test_cwd_cl_exe_cannot_shadow_the_pinned_compiler(self):
        (self.root / "cl.exe").write_text("synthetic CWD tripwire; never execute", encoding="utf-8")
        with contextlib.chdir(self.root):
            result, _ = self.invoke()
        self.assertEqual(Path(result["vc_bin_dir"]), Path(self.config["visualStudioRoot"]) / "VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64")
        self.assertNotIn("", result["paths"].split(os.pathsep))

    def test_generated_script_imports_in_a_fresh_isolated_python_process(self):
        script = self.output / "build/toolchain/win/setup_toolchain.py"
        code = """import pathlib, runpy, subprocess, sys
script = pathlib.Path(sys.argv[1])
sys.path.insert(0, str(script.parent))
def reject(*args, **kwargs):
    raise AssertionError('External toolchain process execution attempted')
subprocess.Popen = reject
sys.argv = sys.argv[1:]
namespace = runpy.run_path(str(script), run_name='__main__')
for name in ('gn_helpers', 'toolchain_paths', 'otter_toolchain_env'):
    assert pathlib.Path(sys.modules[name].__file__).is_relative_to(script.parents[2])
"""
        environment = {**os.environ, "DEPOT_TOOLS_WIN_TOOLCHAIN": "0", "OTTER_ANGLE_TOOLCHAIN_MANIFEST": str(self.manifest)}
        result = subprocess.run([sys.executable, "-I", "-B", "-c", code, str(script), self.config["visualStudioRoot"],
            self.config["sdkRoot"], "", "win", "x64", "none"], env=environment, cwd=self.root,
            capture_output=True, timeout=15, check=False)
        self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", errors="replace"))
        self.assertIn(b"include_flags_imsvc_list", result.stdout)
        self.assertNotIn(b"winsysroot", result.stdout)

    def test_original_inputs_unchanged_and_receipt_binds_only_data_preparation(self):
        self.assertEqual(file_hash(self.source / SETUP_PATH), SETUP_SHA256)
        self.assertEqual(file_hash(self.source / "gn_helpers.py"), GN_HELPERS_SHA256)
        self.assertFalse(self.receipt["toolsExecuted"])
        self.assertFalse(self.receipt["sdkInstalled"])
        self.assertFalse(self.receipt["completeToolchainClosure"])
        self.assertFalse(self.receipt["releaseApproved"])
        for entry in self.receipt["files"]:
            self.assertEqual(file_hash(self.output / entry["path"]), entry["sha256"])
        self.assertEqual(set(path.relative_to(self.output).as_posix() for path in self.output.rglob("*") if path.is_file()),
            {"adapter.json", *(entry["path"] for entry in self.receipt["files"])})

    def test_changed_upstream_is_rejected_before_output_is_created(self):
        with self.assertRaisesRegex(ValueError, "identity mismatch"):
            adapted_setup((self.source / SETUP_PATH).read_bytes() + b"\n")
        source = self.root / "wrong-source"
        (source / "toolchain/win").mkdir(parents=True)
        (source / SETUP_PATH).write_bytes((self.source / SETUP_PATH).read_bytes())
        (source / "gn_helpers.py").write_text("raise RuntimeError('must never run')", encoding="utf-8")
        destination = self.root / "rejected"
        with self.assertRaisesRegex(ValueError, "gn_helpers.py identity mismatch"):
            prepare(source, self.config_file, destination)
        self.assertFalse(destination.exists())

    def test_existing_or_protected_outputs_are_never_overwritten(self):
        before = (self.output / "adapter.json").read_bytes()
        with self.assertRaises(FileExistsError):
            prepare(self.source, self.config_file, self.output)
        self.assertEqual((self.output / "adapter.json").read_bytes(), before)
        for path in (Path(__file__).parents[1] / "new-adapter", self.source / "new-adapter", Path(self.config["sdkRoot"]) / "new-adapter"):
            with self.subTest(path=path), self.assertRaisesRegex(ValueError, "protected"):
                prepare(self.source, self.config_file, path)
            self.assertFalse(path.exists())

    def test_input_change_leaves_failed_directory_without_a_success_receipt(self):
        destination = self.root / "changed-input"
        calls = 0

        def changing_hash(path):
            nonlocal calls
            if Path(path) == Path(self.config_file):
                calls += 1
                if calls > 1:
                    return "0" * 64
            return file_hash(path)

        with patch.object(prepare_toolchain, "file_hash", side_effect=changing_hash):
            with self.assertRaisesRegex(ValueError, "inputs changed"):
                prepare(self.source, self.config_file, destination)
        self.assertTrue((destination / "FAILED.txt").is_file())
        self.assertFalse((destination / "adapter.json").exists())

    def test_preparation_cli_is_runnable_and_refuses_existing_output(self):
        destination = self.root / "cli-adapter"
        script = Path(prepare_toolchain.__file__)
        arguments = [sys.executable, "-X", "utf8", "-B", str(script), "--build-source", str(self.source),
            "--config", self.config_file, "--output", str(destination)]
        result = subprocess.run(arguments, capture_output=True, timeout=15, check=False)
        self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8"))
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt, json.loads((destination / "adapter.json").read_text(encoding="utf-8")))
        self.assertFalse(receipt["toolsExecuted"])
        before = (destination / "adapter.json").read_bytes()
        repeated = subprocess.run(arguments, capture_output=True, timeout=15, check=False)
        self.assertEqual(repeated.returncode, 1)
        self.assertIn(b"angle-toolchain:", repeated.stderr)
        self.assertEqual((destination / "adapter.json").read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
