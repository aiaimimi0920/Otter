"""完整暂存编排的输入/输出/失败合同；仅真实 Git 执行，profile 与工具均用合成数据。"""

import json
import unittest
from contextlib import ExitStack
from pathlib import Path
from unittest.mock import patch

from native_angle_stage_fixture import stage_fixture
import stage_build
from toolchain_paths import file_hash


class StagingTests(unittest.TestCase):
    def setUp(self):
        self.root, self.config_path, self.config, self.sources, self.pins, self.data = stage_fixture()
        self.output = self.root / "stage"

    def fake_profile(self, angle, build, config, output):
        output.mkdir()
        (output / "out/otter").mkdir(parents=True)
        (output / "out/otter/args.gn").write_bytes(b"synthetic profile; not executed\n")
        (output / "environment.json").write_text("{}", encoding="utf-8")
        receipt = {"profile": "fixture", "files": [{"path": name, "sha256": file_hash(output / name)}
                   for name in ("environment.json", "out/otter/args.gn")]}
        (output / "profile.json").write_text(json.dumps(receipt), encoding="utf-8")
        return receipt

    def stage(self):
        with ExitStack() as stack:
            stack.enter_context(patch("stage_build.SOURCES", self.sources))
            stack.enter_context(patch("stage_build.ARTIFACTS", self.pins))
            stack.enter_context(patch("stage_archives.ARTIFACTS", self.pins))
            stack.enter_context(patch("stage_build.prepare_profile", side_effect=self.fake_profile))
            return stage_build.stage(self.config_path, self.output)

    def update(self):
        self.config_path.write_text(json.dumps(self.config), encoding="utf-8")

    def test_inventory_actual_outputs_and_no_build_activation(self):
        receipt = self.stage()
        for key in ("buildToolsExecuted", "hooksExecuted", "downloaded", "completeSourceClosure", "completeToolchainClosure", "runtimeActivated", "releaseApproved"):
            self.assertIs(receipt[key], False)
        self.assertIs(receipt["gitExecuted"], True)
        rows = json.loads((self.output / "files.json").read_text("utf-8"))
        for row in rows:
            self.assertEqual(file_hash(self.output / row["path"]), row["sha256"])
        self.assertEqual(len(rows), receipt["fileCount"])
        self.assertEqual(receipt["filesSha256"], file_hash(self.output / "files.json"))
        self.assertEqual((self.output / "src/data.txt").read_bytes(), self.data)
        lastchange = (self.output / "src/build/util/LASTCHANGE").read_text("utf-8")
        self.assertIn(self.sources[1][2] + "-refs/heads/main@{#1}", lastchange)
        self.assertEqual(receipt["unmaterializedGitlinks"], 2)

    def test_existing_output_is_never_overwritten_or_marked_failed(self):
        self.stage()
        before = (self.output / "stage.json").read_bytes()
        with self.assertRaises(FileExistsError):
            self.stage()
        self.assertEqual(before, (self.output / "stage.json").read_bytes())
        self.assertFalse((self.output / "FAILED.txt").exists())

    def test_protected_source_overlap_is_rejected_before_creation(self):
        self.output = Path(self.config["sourceRoots"]["angle"]) / "forbidden"
        with self.assertRaisesRegex(ValueError, "overlaps"):
            self.stage()
        self.assertFalse(self.output.exists())

    def test_unknown_and_missing_inputs_are_not_inferred(self):
        self.config["sourceRoots"].pop("build")
        self.update()
        with self.assertRaisesRegex(ValueError, "source roots"):
            self.stage()
        self.assertFalse(self.output.exists())

    def test_changed_tool_pin_fails_before_creating_output(self):
        path = Path(self.config["artifacts"]["gn"])
        with path.open("ab") as stream:
            stream.write(b"changed")
        with self.assertRaisesRegex(ValueError, "identity"):
            self.stage()
        self.assertFalse(self.output.exists())

    def test_missing_pinned_source_object_preserves_failed_stage(self):
        self.sources = (self.sources[0][:2] + ("f" * 40, self.sources[0][3]), self.sources[1])
        with self.assertRaisesRegex(ValueError, "Missing"):
            self.stage()
        self.assertTrue((self.output / "FAILED.txt").is_file())
        self.assertFalse((self.output / "stage.json").exists())

    def test_late_configuration_change_is_not_success(self):
        count = 0

        def changing(path):
            nonlocal count
            if Path(path) == self.config_path:
                count += 1
                if count > 1:
                    return "0" * 64
            return file_hash(path)

        with patch("stage_build.file_hash", side_effect=changing), self.assertRaisesRegex(ValueError, "input changed"):
            self.stage()
        self.assertTrue((self.output / "FAILED.txt").is_file())
        self.assertFalse((self.output / "stage.json").exists())


if __name__ == "__main__":
    unittest.main()
