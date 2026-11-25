"""Verify launcher manifest boundaries and process-tree handshakes without requesting or occupying actual foreground."""

import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest


SPEC = importlib.util.spec_from_file_location(
    "native_surface_foreground", Path(__file__).with_name("native-surface-foreground.py"))
LAUNCHER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(LAUNCHER)


@unittest.skipUnless(sys.platform == "win32", "Win32 process Job required")
class ForegroundLauncherTests(unittest.TestCase):
    def setUp(self):
        LAUNCHER.TEMP_ROOT.mkdir(parents=True, exist_ok=True)
        self.work = tempfile.TemporaryDirectory(dir=LAUNCHER.TEMP_ROOT)
        self.addCleanup(self.work.cleanup)
        self.root = Path(self.work.name)

    def test_inventory_rejects_duplicates_and_nonexact_candidates(self):
        path = self.root / "cases.json"
        for names in (["a::b", "a::b"], ["a::b.*"], ["--list"]):
            with self.subTest(names=names):
                manifest = {"driverSha256": "a" * 64, "count": len(names),
                            "cases": [{"test": name} for name in names]}
                path.write_text(json.dumps(manifest), encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "Invalid exact case inventory"):
                    LAUNCHER.inventory(path)

    def handshake(self, allow):
        run_job = LAUNCHER.job_runner()
        path = self.root / "stdout.txt"
        history = []
        rfd, wfd = os.pipe()
        with (os.fdopen(rfd, "rb") as stdin,
              os.fdopen(wfd, "wb", buffering=0) as permission,
              path.open("xb") as out,
              (self.root / "stderr.txt").open("xb") as err):
            def poll(pid):
                if LAUNCHER.READY not in path.read_bytes().splitlines() or history:
                    return True
                history.append(pid)
                if allow:
                    permission.write(b"G")
                return allow

            result = run_job([sys.executable, "-c",
                              "import sys; print('OTTER_FOREGROUND_READY', flush=True);"
                              " assert sys.stdin.buffer.read(1) == b'G'; print('granted')"],
                             self.root, dict(os.environ), out, err, timeout=5,
                             stdin=stdin, on_poll=poll)
        self.assertEqual(history, [result["pid"]])
        return result, path.read_bytes()

    def test_job_bounded_permission_pipe(self):
        result, output = self.handshake(True)
        self.assertEqual((result["status"], result["exitCode"]), ("exited", 0))
        self.assertIn(b"granted", output)

    def test_job_cancels_when_not_authorized(self):
        result, output = self.handshake(False)
        self.assertEqual(result["status"], "cancelled")
        self.assertNotEqual(result["exitCode"], 0)
        self.assertNotIn(b"granted", output)


if __name__ == "__main__":
    unittest.main()
