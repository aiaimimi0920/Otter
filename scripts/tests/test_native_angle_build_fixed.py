"""固定构建编排的确定性合同；所有 GN/Ninja/编译工具执行均被 tripwire 阻断。"""

import os
import unittest
from pathlib import Path
from unittest.mock import patch

from native_angle_build_fixture import build_fixture, save
import build_fixed
from toolchain_paths import file_hash


class FixedBuildTests(unittest.TestCase):
    def setUp(self):
        self.root, self.config, self.stage, self.expected = build_fixture()
        self.output = self.root / "build-evidence"
        self.calls = []
        self.mutate = None
        self.warning = False

    def fake_run(self, runner, name, arguments, cwd, timeout):
        self.calls.append(name)
        result = {"gn": "generated", "plan": "clang-cl.exe /MT /WX\nlld-link.exe\n", "compile": "built", "no-work": "ninja: no work to do."}[name]
        if name == "gn":
            save(runner.output / "gn-trace.json", {"traceEvents": [{"cat": "script_exec", "name": value} for value in build_fixed.SCRIPTS]})
        if name == "compile":
            for dll in ("libEGL.dll", "libGLESv2.dll"):
                (self.stage / "src/out/otter" / dll).write_bytes(b"synthetic PE replacement; never execute")
            if self.mutate:
                self.mutate()
            if self.warning:
                result = "warning: must remain a failure"
        for channel, text in (("stdout", result), ("stderr", "")):
            (runner.output / f"{name}.{channel}.log").write_text(text, encoding="utf-8")
        runner.records.append({"name": name, "exitCode": 0, "status": "synthetic-no-tools-executed"})
        return result

    def fake_dll(self, path, required, system):
        self.assertEqual(len(required), 12 if path.name == "libEGL.dll" else 40)
        return {"path": path.name, "bytes": path.stat().st_size, "sha256": file_hash(path), "fixture": True}

    def execute(self):
        with patch.dict(os.environ, {"DEPOT_TOOLS_WIN_TOOLCHAIN": "0"}), \
             patch("build_fixed.run_job", side_effect=AssertionError("Synthetic tools must never execute")), \
             patch.object(build_fixed.Commands, "run", autospec=True, side_effect=self.fake_run), \
             patch("build_fixed.verify_dll", side_effect=self.fake_dll):
            return build_fixed.execute(self.stage, self.expected, self.output)

    def test_complete_mocked_flow_binds_stage_payload_and_boundaries(self):
        receipt = self.execute()
        self.assertEqual(self.calls, ["gn", "plan", "compile", "no-work"])
        self.assertEqual(receipt["stageSha256"], self.expected)
        self.assertEqual(receipt["compileCommands"], 1)
        for key in ("runtimeVerified", "sdkInstalled", "runtimeActivated", "releaseApproved", "completeSourceClosure", "completeToolchainClosure"):
            self.assertIs(receipt[key], False)
        for row in receipt["files"]:
            self.assertEqual(file_hash(self.output / "payload" / row["path"]), row["sha256"])
        self.assertFalse((self.output / "FAILED.txt").exists())

    def test_wrong_stage_identity_never_creates_output_or_executes(self):
        self.expected = "0" * 64
        with self.assertRaises(ValueError):
            self.execute()
        self.assertFalse(self.output.exists())
        self.assertEqual(self.calls, [])

    def test_existing_output_is_not_overwritten_or_marked_failed(self):
        self.output.mkdir()
        with self.assertRaises(FileExistsError):
            self.execute()
        self.assertEqual(list(self.output.iterdir()), [])
        self.assertEqual(self.calls, [])

    def test_late_source_change_preserves_failed_partial_output(self):
        self.mutate = lambda: (self.stage / "src/main.cc").write_bytes(b"changed during build")
        with self.assertRaisesRegex(ValueError, "changed"):
            self.execute()
        self.assertTrue((self.output / "FAILED.txt").is_file())
        self.assertTrue((self.output / "payload/libEGL.dll").is_file())
        self.assertFalse((self.output / "build.json").exists())

    def test_compiler_warning_does_not_get_a_success_receipt(self):
        self.warning = True
        with self.assertRaisesRegex(ValueError, "diagnostics"):
            self.execute()
        self.assertTrue((self.output / "FAILED.txt").is_file())
        self.assertFalse((self.output / "build.json").exists())

    def test_toolchain_anchor_change_is_rechecked_after_compilation(self):
        path = Path(self.config["visualStudioRoot"]) / "VC/Tools/MSVC" / self.config["msvcVersion"] / "include/vcruntime.h"
        self.mutate = lambda: path.write_bytes(b"changed anchor")
        with self.assertRaisesRegex(ValueError, "anchor"):
            self.execute()
        self.assertTrue((self.output / "FAILED.txt").exists())

    def test_command_exception_keeps_raw_logs_and_failure_record(self):
        self.output.mkdir()
        commands = build_fixed.Commands(self.output, {})
        with patch("build_fixed.run_job", side_effect=OSError("injected process setup failure")):
            with self.assertRaisesRegex(OSError, "injected"):
                commands.run("gn", ["synthetic.exe"], self.stage, 1)
        self.assertTrue((self.output / "gn.json").is_file())
        self.assertEqual(commands.records[0]["status"], "exception")
        self.assertTrue((self.output / "gn.stderr.log").is_file())

    def test_static_crt_wx_and_no_discovery_are_required(self):
        for plan in ("clang-cl.exe /MD /WX\nlld-link.exe", "clang-cl.exe /MT\nlld-link.exe",
                     "clang-cl.exe /MT /WX\nvcvarsall\nlld-link.exe", "clang-cl.exe /MT /WX"):
            with self.subTest(plan=plan), self.assertRaises(ValueError):
                build_fixed.check_plan(plan)


if __name__ == "__main__":
    unittest.main()
