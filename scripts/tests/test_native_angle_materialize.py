"""流式 Git/归档与 Windows 暂存名称空间的拒绝、预算、清理合同。"""

import hashlib
import io
import os
import tarfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch

from native_angle_fixture import fixture
from native_angle_stage_fixture import artifacts, git_source
from stage_archives import tools
from stage_git import GitObjects, materialize, tree_entries
from stage_inventory import Materializer, relative_name


class MaterializeTests(unittest.TestCase):
    def setUp(self):
        self.root, self.config = fixture()
        self.output = self.root / "materialized"
        self.output.mkdir()
        self.writer = Materializer(self.output)

    def test_relative_paths_reject_windows_aliases_and_traversal(self):
        for value in ("/x", "../x", "a/./x", "a//b", "a\\b", "a:x", "a/CON.txt", ".git/config", "a./b", "a /b", "a/\x00",
                      "COM¹", "COM².txt", "COM³", "LPT¹", "LPT².txt", "LPT³", "CONIN$", "CONOUT$.txt"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                relative_name(value)
        self.assertEqual(str(relative_name("中文/a.txt")), "中文/a.txt")

    def test_namespace_rejects_case_duplicates_and_file_directory_conflicts(self):
        self.writer.bytes("Folder/data.txt", b"first", {})
        for name in ("folder/other.txt", "Folder/data.txt", "Folder", "Folder/data.txt/child"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.writer.bytes(name, b"second", {})
        self.assertEqual((self.output / "Folder/data.txt").read_bytes(), b"first")

    def test_bounded_streams_fail_without_inventoring_success(self):
        with self.assertRaisesRegex(ValueError, "Truncated"):
            self.writer.write("truncated", io.BytesIO(b"a"), 2, {})
        self.assertNotIn("truncated", self.writer.files)
        with patch("stage_inventory.MAX_TOTAL", 1), self.assertRaisesRegex(ValueError, "budget"):
            self.writer.bytes("too-large", b"ab", {})
        with patch("stage_inventory.MAX_FILES", 0), self.assertRaisesRegex(ValueError, "count"):
            self.writer.bytes("too-many", b"", {})

    def test_git_blob_digest_is_verified_and_overlay_records_old_identity(self):
        with self.assertRaisesRegex(ValueError, "blob content"):
            self.writer.write("wrong", io.BytesIO(b"a"), 1, {}, git_blob="0" * 40)
        original = self.writer.bytes("overlay", b"before", {"source": "fixture"})
        changed = self.writer.bytes("overlay", b"after", {"profile": "fixture"}, replace=True)
        self.assertEqual(changed["replacedSha256"], original["sha256"])
        with self.assertRaisesRegex(ValueError, "inventoried"):
            self.writer.bytes("unknown", b"", {}, replace=True)

    def test_real_git_object_materialization_ignores_worktree_and_reaps_reader(self):
        executable, source, commit, tree, data = git_source(self.root)
        reader = GitObjects(executable, source, Path(os.environ["SystemRoot"]))
        with reader:
            raw_commit, omitted = materialize(reader, self.writer, ("fixture", "", commit, tree))
        self.assertTrue(raw_commit.startswith(b"tree " + tree.encode()))
        self.assertEqual((self.output / "src/data.txt").read_bytes(), data)
        self.assertEqual((self.output / "src/nested/file.txt").read_bytes(), data)
        self.assertEqual(omitted, [{"path": "src/stub", "gitMode": "160000", "gitObject": "1" * 40}])
        self.assertFalse((self.output / "src/stub").exists())
        self.assertIsNotNone(reader.process.poll())
        self.assertFalse(reader.timer.is_alive())

    def test_missing_git_object_fails_and_process_is_reaped(self):
        executable, source, _, _, _ = git_source(self.root)
        reader = GitObjects(executable, source, Path(os.environ["SystemRoot"]))
        with self.assertRaisesRegex(ValueError, "Missing"), reader:
            reader.object("f" * 40, "commit")
        self.assertIsNotNone(reader.process.poll())
        self.assertFalse(reader.timer.is_alive())

    def test_tree_parser_rejects_truncated_and_case_alias_names(self):
        for data in (b"100644 missing", b"100644 x\0short", b"100644 ../x\0" + bytes(20),
                     b"100644 A\0" + bytes(20) + b"100644 a\0" + bytes(20)):
            with self.subTest(data=data), self.assertRaises(ValueError):
                list(tree_entries(data))

    def test_timer_start_failure_reaps_the_already_started_git_process(self):
        executable, source, _, _, _ = git_source(self.root)
        reader = GitObjects(executable, source, Path(os.environ["SystemRoot"]))
        try:
            with patch("stage_git.threading.Timer.start", side_effect=RuntimeError("timer unavailable")):
                with self.assertRaisesRegex(RuntimeError, "timer unavailable"):
                    reader.__enter__()
            self.assertIsNotNone(reader.process.poll())
            self.assertTrue(reader.process.stdin.closed)
            self.assertTrue(reader.process.stdout.closed)
        finally:
            # 先红的回归也必须回收本测试确切创建的进程，不留现场泄漏。
            if reader.process and reader.process.poll() is None:
                reader.process.kill()
                reader.process.wait(timeout=30)
            if reader.process:
                reader.process.stdin.close()
                reader.process.stdout.close()

    def test_alternate_git_object_store_is_rejected_without_execution(self):
        executable, source, _, _, _ = git_source(self.root)
        (source / ".git/objects/info/alternates").write_text(str(self.root / "outside"), encoding="utf-8")
        with patch("stage_git.subprocess.Popen", side_effect=AssertionError("Git must not execute")):
            with self.assertRaisesRegex(ValueError, "alternates"):
                GitObjects(executable, source, Path(os.environ["SystemRoot"]))

    def test_tool_archives_are_data_only_and_bound(self):
        paths, pins = artifacts(self.root)
        with patch("stage_archives.ARTIFACTS", pins):
            tools(paths, self.writer)
        self.assertEqual(len(self.writer.files), 5)
        self.assertIn("tools/gn.exe", self.writer.files)
        with paths["gn"].open("ab") as stream:
            stream.write(b"changed")
        with patch("stage_archives.ARTIFACTS", pins), self.assertRaisesRegex(ValueError, "identity"):
            tools(paths, self.writer)

    def test_zip_traversal_is_rejected_before_payload(self):
        paths, pins = artifacts(self.root)
        with zipfile.ZipFile(paths["gn"], "a") as archive:
            archive.writestr("../escape", b"bad")
        pins["gn"] = hashlib.sha256(paths["gn"].read_bytes()).hexdigest()
        with patch("stage_archives.ARTIFACTS", pins), self.assertRaises(ValueError):
            tools(paths, self.writer)
        self.assertFalse((self.root / "escape").exists())

    def test_tar_symlinks_never_materialize(self):
        paths, pins = artifacts(self.root)
        with tarfile.open(paths["clang"], "w:xz") as archive:
            member = tarfile.TarInfo("link")
            member.type = tarfile.SYMTYPE
            member.linkname = "target"
            archive.addfile(member)
        pins["clang"] = hashlib.sha256(paths["clang"].read_bytes()).hexdigest()
        with patch("stage_archives.ARTIFACTS", pins), self.assertRaisesRegex(ValueError, "Unsupported"):
            tools(paths, self.writer)
        self.assertFalse((self.output / "src/third_party/llvm-build/Release+Asserts/link").exists())


if __name__ == "__main__":
    unittest.main()
