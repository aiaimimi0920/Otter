"""材料编排、独立封存核验和失败保留；合成 DLL 不可当作真实构建/运行证据。"""

import unittest
from unittest.mock import patch

from native_angle_materials_fixture import angle_materials, materials_fixture, no_tools, save
from angle_materials_evidence import FALSE_BOUNDARIES
from angle_materials_verify import verify
from angle_sbom import documents
from toolchain_paths import file_hash, read_json


class MaterialsTests(unittest.TestCase):
    def setUp(self):
        self.root, self.stage, self.stage_sha, self.build, self.build_sha = materials_fixture()
        self.output = self.root / "materials"

    def create(self):
        with no_tools():
            return angle_materials.create(self.stage, self.stage_sha, self.build, self.build_sha, self.output)

    def seal(self, manifest):
        # 反例显式提供新摘要：不是只测外层 hash 拒绝。
        save(self.output / "materials.json", manifest)
        return file_hash(self.output / "materials.json")

    def test_create_and_standalone_verify_without_original_inputs(self):
        result = self.create()
        self.assertEqual(result["notices"], 3)
        self.stage.rename(self.root / "stage-unavailable")
        self.build.rename(self.root / "build-unavailable")
        with no_tools(), patch("angle_materials.reviewed_build", side_effect=AssertionError("No original inputs")):
            manifest = verify(self.output, result["materialsSha256"])
        for key in FALSE_BOUNDARIES:
            self.assertIs(manifest[key], False)
        carriers = [row for row in manifest["notices"] if row["selection"] == "reviewed-inline-carrier"]
        self.assertEqual(len(carriers), 1)
        for row in manifest["notices"]:
            self.assertEqual((self.output / row["path"]).read_bytes(), (self.root / "stage-unavailable" / row["sourcePath"]).read_bytes())

    def test_wrong_stage_or_build_identity_never_creates_output(self):
        for attribute in ("stage_sha", "build_sha"):
            previous = getattr(self, attribute)
            setattr(self, attribute, "0" * 64)
            with self.subTest(attribute=attribute), self.assertRaises(ValueError):
                self.create()
            self.assertFalse(self.output.exists())
            setattr(self, attribute, previous)

    def test_existing_output_is_not_overwritten_or_marked_failed(self):
        self.output.mkdir()
        with self.assertRaises(FileExistsError):
            self.create()
        self.assertEqual(list(self.output.iterdir()), [])

    def test_changed_producer_preserves_failed_partial_output(self):
        sources = angle_materials.own_sources()
        with patch("angle_materials.own_sources", side_effect=[sources, {}]), self.assertRaisesRegex(ValueError, "sources changed"):
            self.create()
        self.assertTrue((self.output / "FAILED.txt").is_file())
        self.assertTrue((self.output / "payload/libEGL.dll").is_file())
        self.assertFalse((self.output / "materials.json").exists())

    def test_write_budget_rejects_before_copying_overbudget_file(self):
        with patch("angle_materials.MAX_FILE", 1), self.assertRaisesRegex(ValueError, "write budget"):
            self.create()
        self.assertTrue((self.output / "FAILED.txt").is_file())
        self.assertFalse((self.output / "evidence").exists())

    def test_failed_or_aborted_build_cannot_be_collected(self):
        for name in ("FAILED.txt", "ABORTED.json"):
            (self.build / name).write_text("failure evidence\n", encoding="utf-8")
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "Failed or aborted"):
                self.create()
            self.assertFalse(self.output.exists())

    def test_false_approvals_are_rejected_even_with_new_external_digest(self):
        self.create()
        original = read_json(self.output / "materials.json")
        for key in FALSE_BOUNDARIES:
            for value in (True, 0, None):
                manifest = {**original, key: value}
                with self.subTest(key=key, value=value), self.assertRaisesRegex(ValueError, "falsely approve"):
                    verify(self.output, self.seal(manifest))

    def test_extra_file_or_empty_directory_is_rejected(self):
        result = self.create()
        (self.output / "unexpected-empty-directory").mkdir()
        with self.assertRaisesRegex(ValueError, "entries"):
            verify(self.output, result["materialsSha256"])

    def test_unlisted_sdk_dll_is_rejected(self):
        result = self.create()
        (self.output / "payload/d3dcompiler_47.dll").write_bytes(b"never load")
        with self.assertRaisesRegex(ValueError, "entries"):
            verify(self.output, result["materialsSha256"])

    def test_inventoried_extra_dll_is_still_not_allowed(self):
        self.create()
        path = self.output / "payload/d3dcompiler_47.dll"
        path.write_bytes(b"never load")
        manifest = read_json(self.output / "materials.json")
        manifest["files"].append({"path": "payload/d3dcompiler_47.dll", "bytes": path.stat().st_size, "sha256": file_hash(path)})
        manifest["files"].sort(key=lambda row: row["path"])
        with self.assertRaisesRegex(ValueError, "allowlist"):
            verify(self.output, self.seal(manifest))

    def test_payload_or_log_mutation_is_rejected(self):
        result = self.create()
        for name in ("payload/libEGL.dll", "evidence/build/compile.stdout.log"):
            with (self.output / name).open("ab") as stream:
                stream.write(b"changed")
            with self.subTest(name=name), self.assertRaises(ValueError):
                verify(self.output, result["materialsSha256"])

    def test_traversal_alias_boolean_size_and_duplicate_path_are_rejected(self):
        self.create()
        original = read_json(self.output / "materials.json")
        for name in ("../escape", "payload/CON.txt", "payload/a:stream", "payload/a."):
            manifest = {**original, "files": [{**original["files"][0], "path": name}, *original["files"][1:]]}
            with self.subTest(name=name), self.assertRaises(ValueError):
                verify(self.output, self.seal(manifest))
        for replacement in ({**original["files"][0], "bytes": True}, original["files"][1]):
            manifest = {**original, "files": [replacement, *original["files"][1:]]}
            with self.assertRaises(ValueError):
                verify(self.output, self.seal(manifest))

    def test_sboms_cover_material_files_without_dependency_or_license_claims(self):
        self.create()
        manifest = read_json(self.output / "materials.json")
        cdx, spdx = (read_json(self.output / name) for name in ("sbom.cdx.json", "sbom.spdx.json"))
        expected = {row["path"] for row in manifest["files"]} - {"sbom.cdx.json", "sbom.spdx.json"}
        self.assertEqual({row["name"] for row in cdx["components"]}, expected)
        self.assertEqual({row["fileName"][2:] for row in spdx["files"]}, expected)
        self.assertTrue(all(row["licenseInfoInFiles"] == ["NOASSERTION"] for row in spdx["files"]))
        self.assertNotIn("dependencies", cdx)
        self.assertNotIn("packages", spdx)
        second = documents([], self.build_sha)
        self.assertNotEqual(second["sbom.spdx.json"]["documentNamespace"], spdx["documentNamespace"])


if __name__ == "__main__":
    unittest.main()
