"""Keep compilation-only evidence separate from the unchanged product gates."""
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = (ROOT / ".github/workflows/compile-windows.yml").read_text(encoding="utf-8")
PRODUCT = (ROOT / ".github/workflows/build-otter-exe.yml").read_text(encoding="utf-8")
SPEC = importlib.util.spec_from_file_location("ci_compilation_receipt",
                                            ROOT / "scripts/ci_compilation_receipt.py")
COMPILATION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COMPILATION)


class CompilationWorkflowTests(unittest.TestCase):
    def test_manual_and_main_build_are_independent_of_product_checks(self):
        self.assertIn("  workflow_dispatch:", WORKFLOW)
        self.assertIn("  push:\n    branches:\n      - main", WORKFLOW)
        self.assertIn("  compile-windows:\n    runs-on: windows-latest", WORKFLOW)
        self.assertNotIn("build-windows-exe:", WORKFLOW)
        for forbidden in ("needs:", "workflow_run:", "workflow_call:", "continue-on-error",
                          "if: always()", "if: ${{ always() }}"):
            self.assertNotIn(forbidden, WORKFLOW)
        self.assertIn("cargo test --locked --offline --workspace", PRODUCT)
        self.assertIn("package-native-candidate.ps1", PRODUCT)
        self.assertIn("verify-native-package.ps1", PRODUCT)
        self.assertIn("native-package-runtime.py", PRODUCT)
        self.assertNotIn("compile-windows", PRODUCT)

    def test_build_command_and_resource_fallback_match_the_real_product(self):
        package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
        self.assertEqual(COMPILATION.BUILD_COMMAND, package["scripts"]["build"])
        self.assertIn(COMPILATION.BUILD_COMMAND, WORKFLOW)
        self.assertIn('CARGO_TARGET_DIR: ${{ github.workspace }}/native/target', WORKFLOW)
        self.assertIn('toolchain: "1.95.0"', WORKFLOW)
        self.assertIn("Get-Command rc.exe,llvm-rc.exe", WORKFLOW)
        self.assertIn("Windows resource compiler not found", WORKFLOW)
        self.assertIn("test_ci_compilation.py", WORKFLOW)
        self.assertIn('if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }', WORKFLOW)
        self.assertNotIn("cache-hit", WORKFLOW)
        self.assertNotIn("--features", WORKFLOW)
        self.assertLess(WORKFLOW.index(COMPILATION.BUILD_COMMAND),
                        WORKFLOW.index("python scripts/ci_compilation_receipt.py"))
        self.assertIn('--expected-sha "$env:GITHUB_SHA"', WORKFLOW)

    def test_only_receipt_is_uploaded_with_existing_pinned_actions(self):
        actions = re.findall(r"uses: ([^\s]+)", WORKFLOW)
        self.assertEqual(len(actions), 4)
        for action in actions:
            self.assertRegex(action, r"@[0-9a-f]{40}$")
            self.assertIn("uses: " + action, PRODUCT)
        self.assertIn("permissions:\n  contents: read", WORKFLOW)
        self.assertIn("persist-credentials: false", WORKFLOW)
        upload = WORKFLOW.split("      - name: Upload compilation receipt only\n", 1)[1]
        self.assertEqual(WORKFLOW.count("actions/upload-artifact@"), 1)
        self.assertIn("path: ${{ runner.temp }}/otter-compilation-receipt.json", upload)
        self.assertIn("if-no-files-found: error", upload)
        for forbidden in (".exe", ".dll", ".zip", "*", "write", "release"):
            self.assertNotIn(forbidden, upload)
        for forbidden in ("package-native", "build-native-candidate", "gh release",
                          "softprops/action-gh-release", "contents: write"):
            self.assertNotIn(forbidden, WORKFLOW)


class CompilationReceiptTests(unittest.TestCase):
    def setUp(self):
        # Owned disposable fixture; no product files or existing Git state are changed.
        parent = ROOT / ".tmp" / "ci-compilation"
        self.assertTrue(parent.resolve().is_relative_to(ROOT.resolve()))
        parent.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=parent)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.assertTrue(self.root.resolve().is_relative_to(ROOT.resolve()))
        self.git("init", "-q")
        self.git("config", "user.email", "ci@example.invalid")
        self.git("config", "user.name", "CI compilation test")
        self.write("native/Cargo.lock", b"locked graph\n")
        self.git("add", "native/Cargo.lock")
        self.git("commit", "-qm", "compilation fixture")
        self.head = self.git("rev-parse", "HEAD").strip()
        for index, relative in enumerate(COMPILATION.OUTPUTS):
            self.write(relative, b"MZ fixture bytes\n" + bytes([index]))

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root,
                                       stderr=subprocess.PIPE, text=True, encoding="utf-8")

    def write(self, relative, data):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)

    def test_exact_sha_and_nonempty_file_digests_never_imply_package_approval(self):
        result = COMPILATION.receipt(self.root, self.head)
        self.assertIs(result["compilationOnly"], True)
        for flag in ("testsVerified", "packageVerified", "runtimeVerified", "formalReleaseApproved"):
            self.assertIs(result[flag], False)
        self.assertEqual(result["sourceCommit"], self.head)
        self.assertEqual(result["githubSha"], self.head)
        self.assertEqual(result["buildCommand"], COMPILATION.BUILD_COMMAND)
        self.assertEqual(result["cargoLockSha256"], hashlib.sha256(b"locked graph\n").hexdigest())
        self.assertEqual([item["path"] for item in result["files"]], list(COMPILATION.OUTPUTS))
        for item in result["files"]:
            content = (self.root / item["path"]).read_bytes()
            self.assertGreater(item["sizeBytes"], 0)
            self.assertEqual(item["sizeBytes"], len(content))
            self.assertEqual(item["sha256"], hashlib.sha256(content).hexdigest())

    def test_bad_or_different_commit_is_rejected(self):
        for sha in ("", "HEAD", "bad-sha", "a" * 40):
            with self.subTest(sha=sha), self.assertRaises(ValueError):
                COMPILATION.receipt(self.root, sha)

    def test_cli_writes_utf8_json_once(self):
        output = self.root / "receipt.json"
        argv = ["ci_compilation_receipt.py", "--expected-sha", self.head,
                "--output", str(output)]
        with patch.object(COMPILATION, "__file__", str(self.root / "scripts/ci_compilation_receipt.py")), \
                patch("sys.argv", argv):
            COMPILATION.main()
            content = output.read_bytes()
            self.assertFalse(content.startswith(b"\xef\xbb\xbf"))
            self.assertEqual(json.loads(content), COMPILATION.receipt(self.root, self.head))
            with self.assertRaises(FileExistsError):
                COMPILATION.main()
        self.assertEqual(output.read_bytes(), content)

    def test_modified_tracked_input_is_rejected(self):
        self.write("native/Cargo.lock", b"changed graph\n")
        with self.assertRaises(subprocess.CalledProcessError):
            COMPILATION.receipt(self.root, self.head)

    def test_missing_or_empty_binary_is_rejected(self):
        for relative in COMPILATION.OUTPUTS:
            with self.subTest(relative=relative):
                target = self.root / relative
                original = target.read_bytes()
                target.unlink()
                with self.assertRaisesRegex(ValueError, "Missing regular"):
                    COMPILATION.receipt(self.root, self.head)
                target.write_bytes(b"")
                with self.assertRaisesRegex(ValueError, "Empty compilation"):
                    COMPILATION.receipt(self.root, self.head)
                target.write_bytes(original)


if __name__ == "__main__":
    unittest.main()
