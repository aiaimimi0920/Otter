"""从固定本地 Git 对象和工具归档生成全新 ANGLE 构建树；不下载或执行构建工具。"""

import argparse
import datetime
import json
import re
import sys
from pathlib import Path

if __name__ == "__main__":
    sys.path.insert(0, str(Path(__file__).resolve().parent))

from otter_toolchain_env import create_manifest
from prepare_profile import prepare_profile
from prepare_toolchain import write_json
from stage_archives import tools
from stage_git import GitObjects, materialize
from stage_inventory import Materializer, relative_name
from stage_pins import ARTIFACTS, SOURCES
from toolchain_paths import checked_path, file_hash, read_json


def configuration(path, output):
    path = checked_path(str(path), directory=False)
    config = read_json(path)
    if not isinstance(config, dict) or set(config) != {"schemaVersion", "sourceRoots", "artifacts", "toolchainConfig", "gitExecutable"}:
        raise ValueError("Unexpected staging configuration fields")
    if type(config["schemaVersion"]) is not int or config["schemaVersion"] != 1:
        raise ValueError("Unsupported staging schema")
    if not isinstance(config["sourceRoots"], dict) or set(config["sourceRoots"]) != {row[0] for row in SOURCES}:
        raise ValueError("Exact pinned source roots required")
    if not isinstance(config["artifacts"], dict) or set(config["artifacts"]) != set(ARTIFACTS):
        raise ValueError("Exact pinned tool artifacts required")
    roots = {key: checked_path(value) for key, value in config["sourceRoots"].items()}
    artifacts = {key: checked_path(value, directory=False) for key, value in config["artifacts"].items()}
    toolchain_path = checked_path(config["toolchainConfig"], directory=False)
    toolchain = read_json(toolchain_path)
    create_manifest(toolchain)
    if set(toolchain["architectures"]) != {"x86", "x64"}:
        raise ValueError("Both toolchain architectures required")
    git = checked_path(config["gitExecutable"], directory=False)
    checked_path(str(output.parent))
    relative_name(output.name)
    if not output.is_absolute() or re.search(r'[;"%!?&|<>$`]', output.name):
        raise ValueError("Absolute safe new staging output required")
    protected = [Path(__file__).resolve().parents[2], path, toolchain_path, git,
        *roots.values(), *artifacts.values(), *(Path(toolchain[key]) for key in (
            "visualStudioRoot", "sdkRoot", "systemRoot", "tempRoot"))]
    if any(output.is_relative_to(item) or item.is_relative_to(output) for item in protected):
        raise ValueError("Staging output overlaps a protected input")
    return path, roots, artifacts, toolchain_path, toolchain, git


def lastchange(commit):
    header, message = commit.split(b"\n\n", 1)
    stamp = re.search(rb"(?m)^committer .* ([0-9]+) [+-][0-9]{4}$", header)
    if not stamp:
        raise ValueError("Build commit lacks a valid committer time")
    timestamp = stamp[1].decode("ascii")
    position = re.search(rb"(?m)^Cr-Commit-Position: (.+)$", message)
    revision = next(row[2] for row in SOURCES if row[0] == "build")
    full = revision + ("-" + position[1].decode("utf-8") if position else "")
    year = datetime.datetime.fromtimestamp(int(timestamp), datetime.timezone.utc).year
    return {"src/build/util/LASTCHANGE": f"LASTCHANGE={full}\nLASTCHANGE_YEAR={year}\n".encode(),
            "src/build/util/LASTCHANGE.committime": timestamp.encode()}


def stage(config_path, output):
    output = Path(output)
    path, roots, artifacts, toolchain_path, toolchain, git = configuration(config_path, output)
    inputs = {str(item): file_hash(item) for item in (path, toolchain_path, git, *artifacts.values())}
    own = Path(__file__).resolve().parent
    own_hashes = {item.name: file_hash(item) for item in own.glob("*.py")}
    if any(inputs[str(artifacts[key])] != expected for key, expected in ARTIFACTS.items()):
        raise ValueError("Pinned tool artifact identity mismatch")
    output.mkdir()
    try:
        writer = Materializer(output)
        gitlinks = []
        for pin in SOURCES:
            with GitObjects(git, roots[pin[0]], Path(toolchain["systemRoot"])) as reader:
                commit, omitted = materialize(reader, writer, pin)
            gitlinks.extend(omitted)
            if pin[0] == "build":
                for name, data in lastchange(commit).items():
                    writer.bytes(name, data, {"generatedFromCommit": pin[2]})
        mounts = {"src/" + row[1]: row[2] for row in SOURCES if row[1]}
        for row in gitlinks:
            if row["path"] in mounts:
                if row["gitObject"] != mounts[row["path"]]:
                    raise ValueError("Mounted Git submodule revision mismatch")
                row["materializedSeparately"] = True
            else:
                row["materializedSeparately"] = False
        tools(artifacts, writer)
        profile = prepare_profile(output / "src", output / "src/build", toolchain_path, output / "profile")
        for row in profile["files"]:
            if row["path"] == "environment.json":
                continue
            target = "src/" + row["path"]
            data = (output / "profile" / row["path"]).read_bytes()
            if file_hash(output / "profile" / row["path"]) != row["sha256"]:
                raise ValueError("Profile overlay changed during staging")
            writer.bytes(target, data, {"profile": profile["profile"]}, replace=target in writer.files)
        # 对实际输出再读一遍；清单不是仅对曾计划写入的字节作承诺。
        for row in writer.files.values():
            if file_hash(output / row["path"]) != row["sha256"]:
                raise ValueError("Staged content changed before receipt")
        if any(file_hash(Path(name)) != expected for name, expected in inputs.items()):
            raise ValueError("Staging input changed during materialization")
        if own_hashes != {item.name: file_hash(item) for item in own.glob("*.py")}:
            raise ValueError("Staging sources changed during materialization")
        write_json(output / "files.json", sorted(writer.files.values(), key=lambda row: row["path"]))
        write_json(output / "gitlinks.json", gitlinks)
        receipt = {
            "schemaVersion": 1, "kind": "otter-angle-fixed-build-stage", "profile": profile["profile"],
            "sources": [{"key": row[0], "mount": row[1], "commit": row[2], "tree": row[3]} for row in SOURCES],
            "inputSha256": inputs, "stagingSourceSha256": own_hashes, "filesSha256": file_hash(output / "files.json"),
            "gitlinksSha256": file_hash(output / "gitlinks.json"), "profileSha256": file_hash(output / "profile/profile.json"),
            "fileCount": len(writer.files), "gitlinkCount": len(gitlinks),
            "unmaterializedGitlinks": sum(not row["materializedSeparately"] for row in gitlinks),
            "gitExecuted": True, "buildToolsExecuted": False, "hooksExecuted": False, "downloaded": False,
            "completeSourceClosure": False, "completeToolchainClosure": False,
            "runtimeActivated": False, "releaseApproved": False,
        }
        write_json(output / "stage.json", receipt)
        return receipt
    except BaseException:
        (output / "FAILED.txt").write_text("ANGLE staging failed; preserve this directory.\n", encoding="utf-8")
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        print(json.dumps(stage(args.config, args.output), ensure_ascii=False))
    except (OSError, ValueError) as error:
        print(f"angle-stage: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
