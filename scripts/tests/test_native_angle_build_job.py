"""真实 Python 子进程验证 Job 生命周期；不执行合成编译工具或产品程序。"""

import ctypes as C
import os
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

from native_angle_fixture import fixture
import build_job


class BuildJobTests(unittest.TestCase):
    def setUp(self):
        self.root, _ = fixture()
        self.environment = {"SYSTEMROOT": os.environ["SYSTEMROOT"], "TEMP": str(self.root), "TMP": str(self.root)}

    def run_python(self, script, *, timeout=10, max_log=1024 * 1024):
        with (self.root / "stdout").open("xb") as stdout, (self.root / "stderr").open("xb") as stderr:
            return build_job.run_job([str(Path(sys.executable).resolve()), "-I", "-B", "-c", script], self.root,
                self.environment, stdout, stderr, timeout=timeout, max_log=max_log)

    def assert_exited(self, pid):
        open_process = build_job.api("OpenProcess", [build_job.W.DWORD, build_job.W.BOOL, build_job.W.DWORD], build_job.W.HANDLE)
        handle = open_process(0x100000 | 0x1000, False, pid)
        if handle:
            try:
                # Job 的 active=0 可先于被终止进程对象完成 rundown；等待同一 handle 的有界信号。
                self.assertEqual(build_job.wait(handle, 30000), 0, f"Process {pid} did not finish termination")
            finally:
                build_job.close(handle)
        else:
            self.assertEqual(C.get_last_error(), 87)

    def spawning(self, *, sleep):
        child = self.root / "child-pid"
        return ("import subprocess,sys,time; from pathlib import Path; "
            "p=subprocess.Popen([sys.executable,'-I','-B','-c','import time; time.sleep(60)']); "
            f"Path({str(child)!r}).write_text(str(p.pid)); " + ("time.sleep(60)" if sleep else "sys.exit(0)"))

    def test_explicit_environment_logs_and_closed_stdin(self):
        with patch.dict(os.environ, {"OTTER_POISON": "must-not-inherit"}):
            result = self.run_python("import os,sys; print(os.getenv('OTTER_POISON')); print(repr(sys.stdin.read())); sys.stderr.write('separate\\n')")
        self.assertEqual((result["exitCode"], result["status"]), (0, "exited"))
        self.assertIn("None", (self.root / "stdout").read_text("utf-8"))
        self.assertIn("''", (self.root / "stdout").read_text("utf-8"))
        self.assertEqual((self.root / "stderr").read_text("utf-8").strip(), "separate")

    def test_timeout_reaps_root_and_spawned_child(self):
        result = self.run_python(self.spawning(sleep=True), timeout=3)
        self.assertEqual(result["status"], "timeout")
        self.assert_exited(result["pid"])
        self.assert_exited(int((self.root / "child-pid").read_text("utf-8")))

    def test_successful_root_cannot_leave_a_live_descendant(self):
        result = self.run_python(self.spawning(sleep=False))
        self.assertEqual(result["exitCode"], 0)
        self.assert_exited(int((self.root / "child-pid").read_text("utf-8")))

    def test_log_limit_also_applies_if_process_exits_before_poll(self):
        result = self.run_python("import sys; sys.stdout.write('x'*200000)", max_log=1024)
        self.assertEqual(result["status"], "output-limit")
        self.assert_exited(result["pid"])

    def test_assignment_failure_never_resumes_and_reaps_suspended_root(self):
        captured = []
        get_pid = build_job.api("GetProcessId", [build_job.W.HANDLE], build_job.W.DWORD)

        def fail(job, process):
            captured.append(get_pid(process))
            C.set_last_error(5)
            return False

        with patch.object(build_job, "assign", side_effect=fail), patch.object(build_job, "resume", side_effect=AssertionError("Must stay suspended")):
            with self.assertRaises(OSError):
                self.run_python("raise AssertionError('must never execute')")
        self.assertEqual(len(captured), 1)
        self.assert_exited(captured[0])
        self.assertEqual((self.root / "stdout").stat().st_size, 0)

    def test_mutex_blocks_a_different_process_without_lock_files(self):
        module = str(Path(build_job.__file__).parent)
        script = f"import sys; from pathlib import Path; sys.path.insert(0,{module!r}); from build_job import stage_lock\nwith stage_lock(Path({str(self.root)!r})): pass"
        with build_job.stage_lock(self.root):
            result = self.run_python(script)
        self.assertNotEqual(result["exitCode"], 0)
        self.assertIn("already owned", (self.root / "stderr").read_text("utf-8"))
        self.assertFalse((self.root / ".otter-build.lock").exists())

    def test_cleanup_failure_still_closes_other_handles(self):
        original = build_job.drain_job

        def failed_cleanup(job):
            original(job)
            raise OSError("injected cleanup failure")

        with patch.object(build_job, "drain_job", side_effect=failed_cleanup), patch.object(build_job, "close", wraps=build_job.close) as closing:
            with self.assertRaisesRegex(OSError, "injected cleanup"):
                self.run_python("pass")
        self.assertEqual(closing.call_count, 6)


if __name__ == "__main__":
    unittest.main()
