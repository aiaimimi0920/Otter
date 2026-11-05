"""独立审核摘要、原始构建证据、声明原字节及 SBOM 的拒绝门。"""

import unittest
from unittest.mock import patch

from native_angle_materials_fixture import angle_materials, materials_fixture, no_tools, save
from angle_intake import reviewed_build
from angle_materials_verify import verify
from angle_notices import selected_notices
from toolchain_paths import file_hash, read_json


class IntakeTests(unittest.TestCase):
    def setUp(self):
        self.root, self.stage, self.stage_sha, self.build, self.build_sha = materials_fixture()
        self.output = self.root / "materials"

    def review(self):
        with no_tools():
            return reviewed_build(self.stage, self.stage_sha, self.build, self.build_sha)

    def edit_receipt(self, callback):
        value = read_json(self.build / "build.json")
        callback(value)
        save(self.build / "build.json", value)
        self.build_sha = file_hash(self.build / "build.json")

    def test_command_failure_or_boolean_exit_code_is_rejected(self):
        for value in (1, False):
            self.edit_receipt(lambda receipt: receipt["commands"][2].update(exitCode=value))
            with self.subTest(value=value), self.assertRaisesRegex(ValueError, "Unsuccessful"):
                self.review()

    def test_command_set_cannot_be_partial(self):
        self.edit_receipt(lambda receipt: receipt["commands"].pop())
        with self.assertRaisesRegex(ValueError, "Incomplete"):
            self.review()

    def test_false_runtime_approval_in_build_is_rejected(self):
        self.edit_receipt(lambda receipt: receipt.update(runtimeActivated=True))
        with self.assertRaisesRegex(ValueError, "falsely approved"):
            self.review()

    def test_builder_source_digest_cannot_be_relabelled(self):
        self.edit_receipt(lambda receipt: receipt["sourceSha256"].update({"build_binary.py": "0" * 64}))
        with self.assertRaisesRegex(ValueError, "owner"):
            self.review()

    def test_bound_log_mutation_is_rejected(self):
        (self.build / "compile.stdout.log").write_bytes(b"not the bound log")
        with self.assertRaisesRegex(ValueError, "identity changed"):
            self.review()

    def test_diagnostics_reject_even_rebound_success_receipt(self):
        path = self.build / "compile.stdout.log"
        path.write_bytes(b"warning: retained diagnostic\n")
        self.edit_receipt(lambda receipt: receipt["commands"][2].update(stdoutSha256=file_hash(path)))
        with self.assertRaisesRegex(ValueError, "diagnostics"):
            self.review()

    def test_cache_dll_cannot_differ_from_copied_payload(self):
        (self.stage / "src/out/otter/libEGL.dll").write_bytes(b"changed cached DLL")
        with self.assertRaises(ValueError):
            self.review()

    def test_notice_bom_is_not_silently_removed(self):
        path = self.stage / "src/LICENSE"
        path.write_bytes(b"\xef\xbb\xbflicense\n")
        rows = read_json(self.stage / "files.json")
        row = next(row for row in rows if row["path"] == "src/LICENSE")
        row.update(bytes=path.stat().st_size, sha256=file_hash(path))
        save(self.stage / "files.json", rows)
        with self.assertRaisesRegex(ValueError, "BOM"):
            selected_notices(self.stage)
        self.assertTrue(path.read_bytes().startswith(b"\xef\xbb\xbf"))

    def test_notice_budget_and_required_root_are_enforced(self):
        with patch("angle_notices.MAX_TEXT", 1), self.assertRaisesRegex(ValueError, "budget"):
            selected_notices(self.stage)
        rows = [row for row in read_json(self.stage / "files.json") if row["path"] != "src/LICENSE"]
        save(self.stage / "files.json", rows)
        with self.assertRaisesRegex(ValueError, "root LICENSE"):
            selected_notices(self.stage)

    def test_rebound_sbom_cannot_claim_dependency_closure(self):
        with no_tools():
            angle_materials.create(self.stage, self.stage_sha, self.build, self.build_sha, self.output)
        path = self.output / "sbom.cdx.json"
        cdx = read_json(path)
        cdx["dependencies"] = [{"ref": "invented-runtime-dependency"}]
        save(path, cdx)
        manifest = read_json(self.output / "materials.json")
        row = next(row for row in manifest["files"] if row["path"] == path.name)
        row.update(bytes=path.stat().st_size, sha256=file_hash(path))
        save(self.output / "materials.json", manifest)
        with self.assertRaisesRegex(ValueError, "SBOM inventory or scope"):
            verify(self.output, file_hash(self.output / "materials.json"))

    def test_total_budget_applies_before_each_write(self):
        with no_tools(), patch("angle_materials.MAX_TOTAL", 4), self.assertRaisesRegex(ValueError, "write budget"):
            angle_materials.create(self.stage, self.stage_sha, self.build, self.build_sha, self.output)
        self.assertEqual({path.name for path in self.output.iterdir()}, {"FAILED.txt"})


if __name__ == "__main__":
    unittest.main()
