"""Rejection contracts for expected external-stage identity, exact file sets, post-build immutability, and pure PE checks."""

import hashlib
import unittest
from pathlib import Path
from unittest.mock import patch

from native_angle_build_fixture import build_fixture, pe
from build_binary import Image, verify_dll
from build_inputs import new_output, verify_stage
from toolchain_paths import anchors, file_hash, layout, read_json


class BuildInputsTests(unittest.TestCase):
    def setUp(self):
        self.root, self.config, self.stage, self.expected = build_fixture()

    def test_valid_synthetic_stage_is_data_only_and_bound(self):
        with patch("subprocess.Popen", side_effect=AssertionError("No tool execution")):
            receipt, _ = verify_stage(self.stage, self.expected)
        self.assertEqual(receipt["fileCount"], 4)

    def test_expected_external_stage_identity_is_not_inferred(self):
        for value in ("0" * 64, "", "x" * 64):
            with self.subTest(value=value), self.assertRaises(ValueError):
                verify_stage(self.stage, value)

    def test_uninventoried_input_or_prior_cache_is_rejected(self):
        for name in ("src/shadow.py", "src/out/otter/build.ninja"):
            path = self.stage / name
            path.write_text("not reviewed", encoding="utf-8")
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "Unexpected"):
                verify_stage(self.stage, self.expected)
            # Delete only files this test created within its explicit temporary root.
            path.unlink()

    def test_built_cache_cannot_hide_source_changes(self):
        (self.stage / "src/out/otter/generated").write_bytes(b"permitted cache")
        verify_stage(self.stage, self.expected, built=True)
        (self.stage / "src/main.cc").write_bytes(b"changed")
        with self.assertRaises(ValueError):
            verify_stage(self.stage, self.expected, built=True)

    def test_owner_change_requires_fresh_stage(self):
        with patch("build_inputs.own_sources", return_value={}), self.assertRaisesRegex(ValueError, "owner"):
            verify_stage(self.stage, self.expected)

    def test_new_output_rejects_overlap_and_windows_device_name(self):
        for output in (self.stage / "evidence", self.root / "CON", self.root / "a."):
            with self.subTest(output=output), self.assertRaises(ValueError):
                new_output(output, self.stage, self.config)

    def test_json_extension_retains_default_bound_and_duplicate_rejection(self):
        path = self.root / "large.json"
        path.write_text('"' + "a" * 65536 + '"', encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "size"):
            read_json(path)
        self.assertEqual(len(read_json(path, limit=100000)), 65536)
        path.write_text('{"x":1,"x":2}', encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "Duplicate"):
            read_json(path, limit=100000)
        for limit in (0, True, 16 * 1024 * 1024 + 1):
            with self.subTest(limit=limit), self.assertRaises(ValueError):
                read_json(path, limit=limit)

    def test_pe_has_real_addressed_exports_and_static_imports(self):
        system = self.root / "system/System32"
        system.mkdir(parents=True)
        (system / "KERNEL32.dll").write_bytes(b"synthetic system data")
        record = verify_dll(pe(self.root / "valid.dll"), ["eglExample"], system.parent)
        self.assertEqual(record["machine"], "x64")
        self.assertFalse(record["dynamicMsvcCrt"])

    def test_export_names_without_valid_function_binding_are_rejected(self):
        for parameters in ({"function_count": 0}, {"ordinal": 1}, {"function_rva": 0}, {"function_rva": 0x999999}, {"function_rva": 0x1040}):
            with self.subTest(parameters=parameters), self.assertRaises(ValueError):
                Image(pe(self.root / "invalid.dll", **parameters)).exports()

    def test_delay_loaded_dynamic_crt_is_rejected(self):
        system = self.root / "system/System32"
        system.mkdir(parents=True)
        (system / "KERNEL32.dll").write_bytes(b"synthetic system data")
        for name in ("VCRUNTIME140.dll", "MSVCP140.dll", "ucrtbase.dll", "api-ms-win-crt-runtime-l1-1-0.dll"):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "CRT"):
                verify_dll(pe(self.root / "delay.dll", delay=name), ["eglExample"], system.parent)

    def test_required_export_into_nonexecutable_data_is_rejected(self):
        system = self.root / "system/System32"
        system.mkdir(parents=True)
        (system / "KERNEL32.dll").write_bytes(b"synthetic system data")
        with self.assertRaisesRegex(ValueError, "executable"):
            verify_dll(pe(self.root / "data-export.dll", executable=False), ["eglExample"], system.parent)

    def test_optional_data_export_is_not_required_to_be_executable(self):
        system = self.root / "system/System32"
        system.mkdir(parents=True)
        (system / "KERNEL32.dll").write_bytes(b"synthetic system data")
        path = pe(self.root / "optional-data.dll", data_export=True)
        self.assertEqual(set(Image(path).exports()), {"eglExample", "optionalData"})
        verify_dll(path, ["eglExample"], system.parent)
        with self.assertRaisesRegex(ValueError, "executable"):
            verify_dll(path, ["optionalData"], system.parent)


    def test_pe_receipt_hash_binds_parsed_bytes_and_rechecks_path(self):
        system = self.root / "system/System32"
        system.mkdir(parents=True)
        (system / "KERNEL32.dll").write_bytes(b"synthetic system data")
        path = pe(self.root / "valid.dll")
        alternate = self.root / "alternate.dll"
        original = path.read_bytes()
        alternate.write_bytes(original + b"different snapshot")
        record = verify_dll(path, ["eglExample"], system.parent)
        self.assertEqual((record["bytes"], record["sha256"]), (len(original), hashlib.sha256(original).hexdigest()))
        real_open = Path.open
        reads = 0

        def different_second_open(candidate, *args, **kwargs):
            nonlocal reads
            if candidate == path:
                reads += 1
                if reads == 2:
                    return real_open(alternate, *args, **kwargs)
            return real_open(candidate, *args, **kwargs)

        with patch.object(Path, "open", different_second_open), self.assertRaisesRegex(ValueError, "changed during PE verification"):
            verify_dll(path, ["eglExample"], system.parent)
        self.assertEqual(reads, 2)


    def test_pe_snapshot_and_recheck_enforce_actual_byte_limits(self):
        path = pe(self.root / "bounded.dll")
        size = path.stat().st_size
        with patch("build_binary.MAX_HASH_BYTES", size):
            self.assertEqual(len(Image(path).data), size)
        with patch("build_binary.MAX_HASH_BYTES", size - 1), self.assertRaisesRegex(ValueError, "size limit"):
            Image(path)
        alternate = self.root / "larger.dll"
        alternate.write_bytes(path.read_bytes() + b"x")
        real_open = Path.open

        def larger_open(candidate, *args, **kwargs):
            return real_open(alternate if candidate == path else candidate, *args, **kwargs)

        with patch("toolchain_paths.MAX_HASH_BYTES", size):
            self.assertEqual(file_hash(path), hashlib.sha256(path.read_bytes()).hexdigest())
            with patch.object(Path, "open", larger_open), self.assertRaisesRegex(ValueError, "size limit"):
                file_hash(path)


    def test_shared_anchors_are_hashed_once_per_scan_not_across_scans(self):
        roots = layout(self.config)
        with patch("toolchain_paths.file_hash", wraps=file_hash) as read:
            first = anchors(self.config, roots)
            self.assertEqual(len(first), 17)
            self.assertEqual(read.call_count, len(first))
            target = roots["msvc"] / "include/vcruntime.h"
            target.write_bytes(b"changed shared anchor")
            second = anchors(self.config, roots)
            self.assertEqual(read.call_count, len(first) * 2)
            self.assertEqual({key for key in first if first[key] != second[key]}, {str(target)})


if __name__ == "__main__":
    unittest.main()
