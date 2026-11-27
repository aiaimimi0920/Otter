"""Isolated regressions for package-probe decisions; simulated process results do not replace actual EXE runtime gates."""
from contextlib import redirect_stdout
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import tomllib
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import zipfile

SPEC = importlib.util.spec_from_file_location(
    "native_package_runtime", Path(__file__).with_name("native-package-runtime.py"))
RUNTIME = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNTIME)


class PackageRuntimeTests(unittest.TestCase):
    def test_release_profile_preserves_panic_recovery(self):
        manifest = Path(__file__).resolve().parents[2] / "native/Cargo.toml"
        with manifest.open("rb") as source:
            profile = tomllib.load(source)["profile"]["release"]
        self.assertEqual(profile.get("panic", "unwind"), "unwind")

    def run_probe(self, fault=None, aliased_root=False, public=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            if aliased_root:
                # A real parent traversal reproduces path aliases without 8.3 names or symlink privileges.
                (root / "alias").mkdir()
                root = root / "alias" / ".."
            package = root / "package"
            package.mkdir()
            exe = b"isolated unit-test executable placeholder"
            digest = hashlib.sha256(exe).hexdigest()
            bundle = package / ("Otter-native-v1.2.3-windows-x64.zip" if public else "Otter-native-test-windows-x64-internal.zip")
            with zipfile.ZipFile(bundle, "w") as archive:
                archive.writestr("otter-native.exe", exe)
                archive.writestr("otter_live_hover_hook.dll", b"test DLL placeholder")
            (package / "archive-verification.json").write_text(
                json.dumps({"executableSha256": digest, "archive": bundle.name,
                            "archiveSha256": hashlib.sha256(bundle.read_bytes()).hexdigest()}), encoding="utf-8")
            evidence_arg = root / "evidence"
            evidence = evidence_arg.resolve()
            if aliased_root:
                self.assertNotEqual(evidence_arg, evidence)
            calls = []

            def run_job(command, work, env, out, err, timeout):
                label = work.name
                calls.append(label)
                self.assertEqual(Path(command[0]), evidence / "unpacked/otter-native.exe")
                arguments = {
                    "workspace-check": ["--workspace-check"],
                    "pinned-session-check": ["--session", str(evidence / "pinned-session.json"),
                                             "--session-assets", str(evidence / "assets"),
                                             "--session-check"],
                    "smoke-disabled": ["--smoke-test"],
                    "pin-missing-image": ["--pin"],
                    "unknown-option": ["--unknown-option"],
                }
                self.assertEqual(command[1:], arguments[label])
                self.assertEqual(Path(env["OTTER_NATIVE_PROFILE_DIR"]), work / "profile")
                self.assertEqual(Path(env["TEMP"]), work / "temp")
                self.assertNotIn("OTTER_RUNTIME_TEST_SENTINEL", env)
                self.assertNotIn("LOOM_RUNTIME_TEST_SENTINEL", env)
                success = label in ("workspace-check", "pinned-session-check")
                self.assertEqual(timeout, 60 if success else 15)
                markers = {
                    "workspace-check": (
                        "native-workspace-startup: loaded=false readonly=false manual-target=false\n"
                        "native-session-check: windows=1\n"
                        "native-session-render-check: paints=2 device-recreated=true"),
                    "pinned-session-check": (
                        "native-session-check: windows=1\n"
                        "native-session-render-check: paints=2 device-recreated=true"),
                    "smoke-disabled": "--smoke-test is disabled in release builds",
                    "pin-missing-image": "requires an image",
                    "unknown-option": "Usage:",
                }
                if fault == "missing-marker" and label == "workspace-check":
                    markers[label] = "native-workspace-startup: loaded=false"
                (out if success else err).write(markers[label].encode("utf-8"))
                if fault == "workspace-write" and label == "workspace-check":
                    (work / "profile/workspace.json").write_bytes(b"unexpected")
                if fault == "fixture-residue" and label == "workspace-check":
                    (work / "temp/otter-session-fixture-unreleased").mkdir()
                if fault == "image-write" and label == "pinned-session-check":
                    (evidence / "assets/pinned-fixture.png").write_bytes(b"unexpected")
                if fault == "session-write" and label == "pinned-session-check":
                    (evidence / "pinned-session.json").write_bytes(b"unexpected")
                code = 0 if success else 1
                if fault == "wrong-exit" and label == "smoke-disabled":
                    code = 0
                return {"status": "timeout" if fault == "non-exited" else "exited", "exitCode": code}

            spec = SimpleNamespace(loader=SimpleNamespace(exec_module=lambda module: None))
            job = SimpleNamespace(run_job=run_job)
            argv = ["probe", "--package-dir", str(package), "--evidence-dir", str(evidence_arg)]
            if public:
                argv.append("--public-release")
            with patch("sys.argv", argv), redirect_stdout(io.StringIO()), \
                    patch.dict("os.environ", OTTER_RUNTIME_TEST_SENTINEL="1",
                               LOOM_RUNTIME_TEST_SENTINEL="1"), \
                    patch.object(RUNTIME.importlib.util, "spec_from_file_location", return_value=spec), \
                    patch.object(RUNTIME.importlib.util, "module_from_spec", return_value=job):
                if fault:
                    with self.assertRaises(RuntimeError):
                        RUNTIME.main()
                    self.assertFalse((evidence / "runtime.json").exists())
                    return
                RUNTIME.main()
            receipt = json.loads((evidence / "runtime.json").read_text(encoding="utf-8"))
            self.assertEqual(receipt["executableSha256"], digest)
            self.assertFalse(receipt["visibleMenuTested"])
            self.assertEqual(calls, ["workspace-check", "pinned-session-check",
                                    "smoke-disabled", "pin-missing-image", "unknown-option"])
            self.assertTrue(receipt["checks"][0]["workspaceUnwritten"])
            self.assertTrue(receipt["checks"][1]["sourceUnchanged"])

    def test_isolated_release_probes_accept_only_expected_results(self):
        self.run_probe()
        self.run_probe(public=True)

    def test_isolated_release_probes_resolve_input_path_aliases(self):
        self.run_probe(aliased_root=True)

    def test_failed_checks_never_produce_a_success_receipt(self):
        for fault in ("missing-marker", "workspace-write", "image-write", "session-write",
                      "wrong-exit", "fixture-residue", "non-exited"):
            with self.subTest(fault=fault):
                self.run_probe(fault)


if __name__ == "__main__":
    unittest.main()
