"""Explicit data entry for pinned Chromium setup_toolchain, without discovery or fallback branches."""

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
    """Return only fixed-path environments; upstream callers cannot add runtime_dirs or legacy system SDKs."""
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
    # Do not inherit PATH/INCLUDE/LIB/LIBPATH. Python itself and complete SDK/MSVC closures still require independent binding.
    python_dir = Path(os.path.realpath(sys.executable)).parent
    checked_path(str(python_dir))
    return {
        "INCLUDE": ";".join(map(str, includes)),
        "LIB": ";".join(map(str, libraries)),
        "LIBPATH": ";".join(map(str, libraries)),
        "PATH": os.pathsep.join(map(str, [*binaries, python_dir, roots["systemRoot"] / "System32"])),
        "PATHEXT": ".COM;.EXE;.BAT;.CMD",
        # Explicitly request English diagnostics; without local English resources, consumers must still verify actual /showIncludes prefixes.
        "VSLANG": "1033",
        "SYSTEMROOT": str(roots["systemRoot"]),
        "TEMP": str(roots["tempRoot"]),
        "TMP": str(roots["tempRoot"]),
    }
