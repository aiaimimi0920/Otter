"""构建前后验证外部暂存身份和实际文件集合；外部预期摘要必须由调用者独立取得。"""

import os
import re
import sys
from pathlib import Path

from gn_profile import PROFILE
from stage_inventory import MAX_FILES, relative_name
from stage_pins import SOURCES
from toolchain_paths import checked_path, file_hash, read_json

JSON_LIMIT = 16 * 1024 * 1024


def own_sources():
    return {path.name: file_hash(path) for path in Path(__file__).resolve().parent.glob("*.py")}


def inventory_file(root, name, expected, size=None):
    relative_name(name)
    if not isinstance(expected, str) or not re.fullmatch("[0-9a-f]{64}", expected):
        raise ValueError("Invalid build input SHA-256")
    path = root / name
    if size is not None and (type(size) is not int or path.stat().st_size != size):
        raise ValueError("Build input size changed")
    if file_hash(path) != expected:
        raise ValueError(f"Build input identity changed: {name}")


def actual_files(root, *, built):
    result = set()
    for base, directories, files in os.walk(root, followlinks=False):
        for name in [*directories, *files]:
            path = Path(base) / name
            checked_path(str(path), directory=name in directories)
        for name in files:
            relative = (Path(base) / name).relative_to(root).as_posix()
            # 仅构建缓存允许新文件；源码/工具/profile 不能混入隐式 Python 或 GN 输入。
            if not built or not relative.startswith("src/out/otter/"):
                result.add(relative)
            if len(result) > MAX_FILES + 100:
                raise ValueError("Build input file count exceeded")
    return result


def verify_stage(stage, expected, *, built=False):
    stage = checked_path(str(stage))
    if stage.is_relative_to(Path(__file__).resolve().parents[2]):
        raise ValueError("Build stage must remain outside the repository")
    inventory_file(stage, "stage.json", expected)
    receipt = read_json(stage / "stage.json")
    if not isinstance(receipt, dict) or type(receipt.get("schemaVersion")) is not int or receipt.get("schemaVersion") != 1 or receipt.get("kind") != "otter-angle-fixed-build-stage" or receipt.get("profile") != PROFILE:
        raise ValueError("Pinned stage/profile required")
    pins = [{"key": row[0], "mount": row[1], "commit": row[2], "tree": row[3]} for row in SOURCES]
    if receipt.get("sources") != pins or receipt.get("stagingSourceSha256") != own_sources():
        raise ValueError("Staging pins or owner sources changed; create a fresh stage")
    if receipt.get("gitExecuted") is not True or any(receipt.get(key) is not False for key in (
        "buildToolsExecuted", "hooksExecuted", "downloaded", "completeSourceClosure", "completeToolchainClosure",
        "runtimeActivated", "releaseApproved")):
        raise ValueError("Invalid stage execution/approval boundaries")
    expected_files = {"stage.json", "files.json", "gitlinks.json", "profile/profile.json", "profile/adapter.json"}
    for name, key in (("files.json", "filesSha256"), ("gitlinks.json", "gitlinksSha256"), ("profile/profile.json", "profileSha256")):
        inventory_file(stage, name, receipt[key])
    files = read_json(stage / "files.json", limit=JSON_LIMIT)
    if not isinstance(files, list) or not 0 < len(files) <= MAX_FILES or receipt["fileCount"] != len(files):
        raise ValueError("Invalid staged file inventory")
    for row in files:
        if not isinstance(row, dict) or not isinstance(row.get("path"), str):
            raise ValueError("Invalid staged file record")
        name = row["path"]
        if not name.startswith(("src/", "tools/")) or name in expected_files:
            raise ValueError("Duplicate or unexpected build input")
        inventory_file(stage, name, row["sha256"], row["bytes"])
        expected_files.add(name)
    links = read_json(stage / "gitlinks.json", limit=JSON_LIMIT)
    if not isinstance(links, list) or len(links) != receipt["gitlinkCount"] or len(links) > 50000:
        raise ValueError("Invalid staged gitlink inventory")
    for row in links:
        if not isinstance(row, dict) or type(row.get("materializedSeparately")) is not bool or row.get("gitMode") != "160000":
            raise ValueError("Invalid staged gitlink record")
        relative_name(row["path"])
    if sum(not row["materializedSeparately"] for row in links) != receipt["unmaterializedGitlinks"]:
        raise ValueError("Invalid unmaterialized gitlink count")
    profile = read_json(stage / "profile/profile.json")
    if profile.get("kind") != "otter-angle-gn-profile" or profile.get("profile") != PROFILE:
        raise ValueError("Invalid staged profile receipt")
    inventory_file(stage, "profile/adapter.json", profile["adapterSha256"])
    for row in profile["files"]:
        name = "profile/" + row["path"]
        if name in expected_files:
            raise ValueError("Duplicate profile input")
        inventory_file(stage, name, row["sha256"])
        expected_files.add(name)
    python = checked_path(str(Path(sys.executable).resolve()), directory=False)
    if profile["python"] != {"path": str(python), "sha256": file_hash(python)}:
        raise ValueError("Build Python differs from staged profile")
    if built:
        expected_files = {name for name in expected_files if not name.startswith("src/out/otter/")}
    if actual_files(stage, built=built) != expected_files:
        raise ValueError("Unexpected or missing files in build stage")
    return receipt, profile


def new_output(output, stage, config):
    output = Path(output)
    checked_path(str(output.parent))
    relative_name(output.name)
    if not output.is_absolute() or re.search(r'[;"%!?&|<>$`]', output.name):
        raise ValueError("Absolute safe new build output required")
    protected = [Path(__file__).resolve().parents[2], stage, *(Path(config[key]) for key in (
        "visualStudioRoot", "sdkRoot", "systemRoot", "tempRoot"))]
    if any(output.is_relative_to(path) or path.is_relative_to(output) for path in protected):
        raise ValueError("Build output overlaps a protected input")
    return output
