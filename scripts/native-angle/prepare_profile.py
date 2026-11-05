"""生成固定 Clang GN profile 与既有工具链 adapter；不下载、暂存完整源码或执行工具。"""

import argparse
import json
import sys
from pathlib import Path

# -I 禁用隐式脚本目录搜索；CLI 只显式导入此已选定文件所在目录的兄弟模块。
if __name__ == "__main__":
    sys.path.insert(0, str(Path(__file__).resolve().parent))

from gn_profile import ANCHORS, ANGLE_REVISION, PROFILE, profile_files
from otter_toolchain_env import create_manifest
from prepare_toolchain import BUILD_REVISION, prepare, write_json
from toolchain_paths import checked_path, file_hash, read_json


def prepare_profile(angle_source, build_source, config_file, output):
    angle = checked_path(str(angle_source))
    build = checked_path(str(build_source))
    config_file = checked_path(str(config_file), directory=False)
    config_hash = file_hash(config_file)
    config = read_json(config_file)
    # 先复用现有配置/工具链准入，不能先把未经校验的值插入 GN。
    create_manifest(config)
    if set(config["architectures"]) != {"x86", "x64"}:
        raise ValueError("GN profile requires both x86 and x64 toolchain environments")
    inputs = {name: angle / name if name == ".gn" else build / name.removeprefix("build/") for name in ANCHORS}
    originals = {}
    for name, path in inputs.items():
        if file_hash(path) != ANCHORS[name]:
            raise ValueError(f"Pinned GN profile identity mismatch: {name}")
        originals[name] = path.read_bytes()
    python = checked_path(str(Path(sys.executable).resolve()), directory=False)
    python_hash = file_hash(python)
    files = profile_files(originals, config, str(python))
    output = Path(output)
    if output.is_relative_to(angle) or angle.is_relative_to(output):
        raise ValueError("Output overlaps a protected ANGLE source tree")
    own = Path(__file__).resolve().parent
    own_hashes = {name: file_hash(own / name) for name in ("prepare_profile.py", "gn_profile.py")}
    adapter = prepare(build, config_file, output)
    try:
        for name, data in files.items():
            destination = output / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            with destination.open("xb") as stream:
                stream.write(data)
        if file_hash(config_file) != config_hash or any(file_hash(path) != ANCHORS[name] for name, path in inputs.items()):
            raise ValueError("GN profile inputs changed during preparation")
        if any(file_hash(own / name) != expected for name, expected in own_hashes.items()):
            raise ValueError("GN profile sources changed during preparation")
        if file_hash(python) != python_hash:
            raise ValueError("GN profile Python executable changed during preparation")
        receipt = {
            "schemaVersion": 1, "kind": "otter-angle-gn-profile", "profile": PROFILE,
            "angleRevision": ANGLE_REVISION, "buildRevision": BUILD_REVISION,
            "configSha256": config_hash, "python": {"path": str(python), "sha256": python_hash},
            "inputSha256": ANCHORS, "profileSourceSha256": own_hashes,
            "adapterSha256": file_hash(output / "adapter.json"),
            "files": [*adapter["files"], *({"path": name, "sha256": file_hash(output / name)} for name in files)],
            "generatedNinjaPatched": False, "toolsExecuted": False,
            "completeSourceClosure": False, "completeToolchainClosure": False,
            "runtimeActivated": False, "releaseApproved": False,
        }
        write_json(output / "profile.json", receipt)
        return receipt
    except BaseException:
        (output / "FAILED.txt").write_text("GN profile preparation failed; preserve this directory.\n", encoding="utf-8")
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--angle-source", required=True)
    parser.add_argument("--build-source", required=True)
    parser.add_argument("--config", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        print(json.dumps(prepare_profile(args.angle_source, args.build_source, args.config, args.output), ensure_ascii=False))
    except (OSError, ValueError) as error:
        print(f"angle-gn-profile: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
