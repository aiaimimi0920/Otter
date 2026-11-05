"""固定 Chromium setup_toolchain 的显式数据入口；没有发现或回退分支。"""

import copy
import os
import sys
from pathlib import Path

from toolchain_paths import anchors, checked_path, layout, paths_for, read_json


def create_manifest(config):
    roots = layout(config)
    return {
        "schemaVersion": 1,
        "kind": "otter-angle-toolchain-environment",
        "config": copy.deepcopy(config),
        "anchorSha256": anchors(config, roots),
        "completeToolchainClosure": False,
        "releaseApproved": False,
    }


def load_environment(manifest_path, cpu, toolchain_root, sdk_dir, target_store):
    """只返回固定路径环境；上游调用者不能加入 runtime_dirs 或系统旧 SDK。"""
    if os.environ.get("DEPOT_TOOLS_WIN_TOOLCHAIN") != "0":
        raise ValueError("Explicit DEPOT_TOOLS_WIN_TOOLCHAIN=0 required")
    if not manifest_path:
        raise ValueError("OTTER_ANGLE_TOOLCHAIN_MANIFEST is required; discovery is disabled")
    if target_store:
        raise ValueError("Store toolchains are not admitted")
    manifest = read_json(manifest_path)
    if not isinstance(manifest, dict) or set(manifest) != {
        "schemaVersion", "kind", "config", "anchorSha256", "completeToolchainClosure", "releaseApproved"
    }:
        raise ValueError("Unexpected toolchain manifest fields")
    if type(manifest["schemaVersion"]) is not int or manifest["schemaVersion"] != 1 or (
        manifest["kind"] != "otter-angle-toolchain-environment"
        or manifest["completeToolchainClosure"] is not False or manifest["releaseApproved"] is not False
    ):
        raise ValueError("Unsupported or falsely approved toolchain manifest")
    config = manifest["config"]
    roots = layout(config)
    if checked_path(toolchain_root) != roots["visualStudioRoot"] or checked_path(sdk_dir) != roots["sdkRoot"]:
        raise ValueError("GN toolchain roots differ from the explicit manifest")
    includes, libraries, binaries = paths_for(config, roots, cpu)
    if manifest["anchorSha256"] != anchors(config, roots):
        raise ValueError("Toolchain anchor identity changed")
    # 不继承 PATH/INCLUDE/LIB/LIBPATH。Python 自身及完整 SDK/MSVC 闭包仍须独立绑定。
    python_dir = Path(os.path.realpath(sys.executable)).parent
    checked_path(str(python_dir))
    return {
        "INCLUDE": ";".join(map(str, includes)),
        "LIB": ";".join(map(str, libraries)),
        "LIBPATH": ";".join(map(str, libraries)),
        "PATH": os.pathsep.join(map(str, [*binaries, python_dir, roots["systemRoot"] / "System32"])),
        "PATHEXT": ".COM;.EXE;.BAT;.CMD",
        # 显式请求英文诊断；若本机没有英文资源，消费方仍须验证真实 /showIncludes 前缀。
        "VSLANG": "1033",
        "SYSTEMROOT": str(roots["systemRoot"]),
        "TEMP": str(roots["tempRoot"]),
        "TMP": str(roots["tempRoot"]),
    }
