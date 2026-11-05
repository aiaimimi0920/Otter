"""显式工具链清单的环境隔离、身份和输入拒绝合同。"""

import copy
import os
import unittest
from pathlib import Path
from unittest.mock import patch

from native_angle_fixture import fixture, write_json
from otter_toolchain_env import create_manifest, load_environment
from toolchain_paths import checked_path, read_json


class EnvironmentTests(unittest.TestCase):
    def setUp(self):
        self.root, self.config = fixture()
        self.manifest = create_manifest(self.config)
        self.file = write_json(self.root / "environment.json", self.manifest)
        self.environment = patch.dict(os.environ, {"DEPOT_TOOLS_WIN_TOOLCHAIN": "0"})
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def load(self, cpu="x64", manifest=None, store=False):
        if manifest is not None:
            write_json(Path(self.file), manifest)
        return load_environment(self.file, cpu, self.config["visualStudioRoot"], self.config["sdkRoot"], store)

    def test_environment_does_not_inherit_ambient_sdk_or_paths(self):
        before = dict(os.environ)
        with patch.dict(os.environ, {
            "INCLUDE": "OLD-SDK", "LIB": "OLD-LIB", "LIBPATH": "OLD-LIBPATH",
            "PATH": "POISONED-PATH", "GYP_MSVS_OVERRIDE_PATH": "POISONED-VS",
        }):
            poisoned = dict(os.environ)
            for cpu in ("x86", "x64"):
                env = self.load(cpu)
                self.assertEqual(set(env), {"INCLUDE", "LIB", "LIBPATH", "PATH", "PATHEXT", "VSLANG", "SYSTEMROOT", "TEMP", "TMP"})
                self.assertFalse(any("OLD-" in value or "POISONED" in value for value in env.values()))
                self.assertIn("10.0.26100.0", env["INCLUDE"])
                self.assertTrue(env["LIB"].split(";")[0].endswith(cpu))
                self.assertIn(str(Path(self.config["sdkRoot"]) / "bin/10.0.26100.0/x64"), env["PATH"])
                self.assertEqual(Path(env["PATH"].split(os.pathsep)[0]), Path(
                    self.config["visualStudioRoot"]
                ) / "VC/Tools/MSVC/14.44.35207/bin/Hostx64" / cpu)
                self.assertEqual(env["TMP"], self.config["tempRoot"])
            self.assertEqual(dict(os.environ), poisoned)
        self.assertEqual(dict(os.environ), before)

    def test_missing_manifest_never_falls_back(self):
        for value in (None, ""):
            with self.subTest(value=value), self.assertRaisesRegex(ValueError, "discovery is disabled"):
                load_environment(value, "x64", "ignored", "ignored", False)

    def test_msvc_language_request_is_explicit_and_not_inherited(self):
        with patch.dict(os.environ, {"VSLANG": "2052"}):
            for cpu in ("x86", "x64"):
                self.assertEqual(self.load(cpu).get("VSLANG"), "1033")

    def test_explicit_non_downloading_mode_is_required(self):
        for value in (None, "", "1", "false", "00"):
            with self.subTest(value=value), patch.dict(os.environ):
                if value is None:
                    os.environ.pop("DEPOT_TOOLS_WIN_TOOLCHAIN", None)
                else:
                    os.environ["DEPOT_TOOLS_WIN_TOOLCHAIN"] = value
                with self.assertRaisesRegex(ValueError, "DEPOT_TOOLS_WIN_TOOLCHAIN=0"):
                    self.load()

    def test_missing_architecture_and_store_are_rejected(self):
        for cpu in ("arm", "arm64", "unknown", ""):
            with self.subTest(cpu=cpu), self.assertRaisesRegex(ValueError, "Architecture"):
                self.load(cpu)
        with self.assertRaisesRegex(ValueError, "Store"):
            self.load(store=True)
        config = {**self.config, "architectures": ["x64"]}
        with self.assertRaisesRegex(ValueError, "Architecture"):
            self.load("x86", create_manifest(config))

    def test_gn_root_mismatch_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "GN toolchain roots differ"):
            load_environment(self.file, "x64", str(self.root), self.config["sdkRoot"], False)
        with self.assertRaisesRegex(ValueError, "GN toolchain roots differ"):
            load_environment(self.file, "x64", self.config["visualStudioRoot"], str(self.root), False)

    def test_every_anchor_is_checked_again_at_consumption(self):
        for name in self.manifest["anchorSha256"]:
            path = Path(name)
            before = path.read_bytes()
            with self.subTest(path=name):
                path.write_bytes(before + b"changed")
                with self.assertRaisesRegex(ValueError, "anchor identity changed"):
                    self.load()
                path.write_bytes(before)

    def test_missing_anchor_is_not_a_system_search(self):
        name = next(iter(self.manifest["anchorSha256"]))
        Path(name).rename(Path(name).with_suffix(".retained"))
        with self.assertRaises(OSError):
            self.load()

    def test_schema_unknown_fields_and_false_approval_are_rejected(self):
        for change in ({"schemaVersion": True}, {"releaseApproved": True}, {"completeToolchainClosure": True},
                       {"kind": "other"}, {"unknown": 1}, {"anchorSha256": {}}, {"anchorSha256": []}):
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.load(manifest={**self.manifest, **change})
        for change in ({"schemaVersion": True}, {"sdkVersion": "10.0.22621.0"}, {"msvcVersion": "latest"},
                       {"architectures": []}, {"architectures": ["x64", "x64"]}, {"architectures": [True]},
                       {"unknown": 1}):
            with self.subTest(config=change), self.assertRaises(ValueError):
                create_manifest({**self.config, **change})

    def test_paths_cannot_inject_environment_or_escape_to_unc(self):
        for value in ("relative", "C:relative", r"\\server\SDK", "//server/SDK", str(self.root) + ";old",
                      str(self.root) + "\nINCLUDE=old", str(self.root) + '%PATH%', str(self.root) + '"',
                      str(self.root / ".."), str(self.root) + ":stream", str(self.root) + "."):
            with self.subTest(path=value), self.assertRaises((ValueError, OSError)):
                checked_path(value)

    def test_linked_ancestor_is_rejected_without_following_it(self):
        alias = self.root / "linked"
        # 在无创建符号链接权限的 Windows 上用 lstat 元数据注入覆盖同一拒绝分支。
        info = self.root.lstat()
        linked = type("Info", (), {"st_mode": info.st_mode, "st_file_attributes": 0x400})()
        with patch.object(Path, "lstat", return_value=linked):
            with self.assertRaisesRegex(ValueError, "Linked"):
                checked_path(str(alias))

    def test_duplicate_oversize_and_bom_json_are_rejected(self):
        path = self.root / "bad.json"
        for data in (b'{"schemaVersion":1,"schemaVersion":1}', b" " * 65537, b"\xef\xbb\xbf{}"):
            with self.subTest(size=len(data)):
                path.write_bytes(data)
                with self.assertRaises(ValueError):
                    read_json(path)

    def test_inputs_are_not_mutated_by_snapshot_creation(self):
        config = copy.deepcopy(self.config)
        snapshot = create_manifest(config)
        self.assertEqual(snapshot, self.manifest)
        self.assertEqual(config, self.config)
        config["architectures"].clear()
        self.assertEqual(snapshot["config"], self.config)
        self.assertFalse(self.manifest["completeToolchainClosure"])
        self.assertFalse(self.manifest["releaseApproved"])


if __name__ == "__main__":
    unittest.main()
