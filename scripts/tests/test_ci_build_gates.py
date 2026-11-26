"""Protect the product check identity and the present packaging gates."""
from pathlib import Path
from unittest.mock import patch
import re
import os
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = (ROOT / ".github/workflows/build-otter-exe.yml").read_text(encoding="utf-8")


def job(name):
    match = re.search(r"(?ms)^  " + re.escape(name) + r":\n(.*?)(?=^  [\w-]+:|\Z)", WORKFLOW)
    if not match:
        raise AssertionError(f"Missing job: {name}")
    return match[1]


def aggregate_bash(platform=os.name):
    # Use Bash from the same Git installation rather than resolving the WSL launcher.
    if platform == "nt":
        git_exec = subprocess.check_output(["git", "--exec-path"], text=True, encoding="utf-8").strip()
        return str(Path(git_exec).parents[2] / "bin" / "bash.exe")
    return "bash"


class BuildGateTests(unittest.TestCase):
    def test_windows_shell_never_resolves_the_wsl_launcher(self):
        with patch("subprocess.check_output", return_value="C:/Program Files/Git/mingw64/libexec/git-core"):
            self.assertEqual(aggregate_bash("nt"),
                             str(Path("C:/Program Files") / "Git" / "bin" / "bash.exe"))
        self.assertEqual(aggregate_bash("posix"), "bash")

    def test_stable_check_fails_closed_before_promotion(self):
        aggregate = job("build-windows-exe")
        self.assertIn("needs: [change-scope, build-windows-candidate]", aggregate)
        self.assertIn("if: ${{ always() }}", aggregate)
        self.assertIn('BUILD: ${{ needs.build-windows-candidate.result }}', aggregate)
        self.assertLess(aggregate.index('test "$BUILD" = success'), aggregate.index("Download same-run"))
        self.assertIn("name: otter-build-candidate", aggregate)
        self.assertIn("name: otter-portable-windows-x64", aggregate)
        self.assertEqual(aggregate.count("if:"), 3)
        self.assertEqual(aggregate.count("if: needs.change-scope.outputs.full == 'true'"), 2)
        self.assertIn('test "$SCOPE" = success', aggregate)
        self.assertIn('test "$FULL" = true', aggregate)
        self.assertIn("shell: bash", aggregate)
        for override in ("run-id:", "repository:", "github-token:"):
            self.assertNotIn(override, aggregate)
        self.assertNotIn("otter-portable-windows-x64", job("build-windows-candidate"))
        self.assertNotIn("continue-on-error", WORKFLOW)

    def test_docs_lane_is_offline_and_never_promotes_a_package(self):
        docs = job("change-scope")
        self.assertIn("python3 -m unittest discover", docs)
        self.assertIn("if: steps.scope.outputs.full == 'false'", docs)
        self.assertIn("python3 scripts/check_changed_docs.py", docs)
        for command in ("npm ci", "node_modules", "cargo test", "cargo nextest", "upload-artifact"):
            self.assertNotIn(command, docs)
        self.assertNotIn("paths-ignore:", WORKFLOW)

    def test_aggregate_truth_table(self):
        # Execute the actual workflow shell, including fail-fast Actions behavior.
        aggregate = job("build-windows-exe")
        script = aggregate.split("        run: |\n", 1)[1].split("      - name:", 1)[0]
        script = "\n".join(line[10:] for line in script.splitlines())
        cases = []
        for scope in ("success", "failure", "cancelled", ""):
            for full in ("true", "false", ""):
                for build in ("success", "failure", "cancelled", "skipped", ""):
                    success = scope == "success" and ((full == "true" and build == "success") or
                                                       (full == "false" and build == "skipped"))
                    cases.append((scope, full, build, success))
        with tempfile.TemporaryDirectory() as directory:
            for scope, full, build, success in cases:
                env = dict(os.environ, SCOPE=scope, FULL=full, BUILD=build,
                           GITHUB_STEP_SUMMARY=(Path(directory) / "summary").as_posix())
                result = subprocess.run([aggregate_bash(), "-e", "-c", script], env=env)
                self.assertEqual(result.returncode == 0, success,
                                 (scope, full, build))

    def test_cache_never_skips_native_build_or_test_gates(self):
        candidate = job("build-windows-candidate")
        for command in ("assert-release-version.ps1", "cargo fetch --locked",
                        "cargo nextest run --locked --offline --workspace",
                        "cargo test --doc --locked --offline --workspace", "build-native-candidate.ps1",
                        "package-native-candidate.ps1", "verify-native-package.ps1",
                        "native-package-runtime.py", "test_native_package_runtime.py",
                        "cargo fmt", "--mode strict"):
            self.assertIn(command, candidate)
        self.assertNotIn("build-local-otter-exe.ps1", candidate)
        self.assertIn("workspaces: native", candidate)
        self.assertNotIn("cache-hit", candidate)
        self.assertNotIn("npm ci", candidate)
        self.assertIn("candidate/*-internal.zip", job("build-windows-exe"))
        self.assertNotIn("candidate/otter.exe", job("build-windows-exe"))

    def test_isolated_runner_keeps_full_selection_and_explicit_failure_propagation(self):
        candidate = job("build-windows-candidate")
        self.assertIn("tool: cargo-nextest@0.9.140", candidate)
        self.assertRegex(candidate, r"uses: taiki-e/install-action@[0-9a-f]{40}")
        tests = candidate.split("name: Run native tests with the locked graph", 1)[1]
        tests = tests.split("      - name:", 1)[0]
        for command in ("cargo nextest run", "cargo test --doc"):
            self.assertRegex(tests, re.escape(command) + r"[^\n]+\n\s+if \(\$LASTEXITCODE -ne 0\) \{ exit \$LASTEXITCODE \}")
        self.assertIn("--config-file native/.config/nextest.toml --profile default", tests)
        self.assertIn("--test-threads 1 --no-tests=fail", tests)
        for excluded in ("--profile headless", "--features", "--filter", " -E ", "--skip", "--retries"):
            self.assertNotIn(excluded, tests)
        self.assertLess(candidate.index("cargo test --doc"), candidate.index("build-native-candidate.ps1"))


if __name__ == "__main__":
    unittest.main()
