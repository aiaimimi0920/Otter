"""ANGLE 工具链清单的纯文件边界；不发现、安装或执行任何工具。"""

import hashlib
import json
import re
import stat
from pathlib import Path

SDK_VERSION = "10.0.26100.0"
CONFIG_KEYS = {
    "schemaVersion", "visualStudioRoot", "msvcVersion", "sdkRoot", "sdkVersion",
    "architectures", "systemRoot", "tempRoot",
}


def checked_path(value, *, directory=True):
    """拒绝链接祖先、UNC、设备路径及可污染环境/命令行的路径字符。"""
    if not isinstance(value, str) or not value or re.search(r'[\x00-\x1f;"%!?&|<>$`]', value):
        raise ValueError("Unsafe toolchain path")
    # Path 会消去显式 ./，因此必须在构造 Path 前检查原始分量。
    if any(part in (".", "..") for part in re.split(r"[\\/]", value)):
        raise ValueError("Noncanonical toolchain path")
    path = Path(value)
    if not path.is_absolute() or value.startswith(("\\\\", "//")):
        raise ValueError("Toolchain paths must be absolute local or existing mapped-drive paths")
    for part in path.parts[1:]:
        if part in (".", "..") or part.endswith((".", " ")) or ":" in part:
            raise ValueError("Noncanonical toolchain path")
    for ancestor in [*reversed(path.parents), path]:
        info = ancestor.lstat()
        if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
            raise ValueError("Linked toolchain path")
    if not (path.is_dir() if directory else path.is_file()):
        raise ValueError("Toolchain input has wrong file type")
    return path


def file_hash(path):
    path = checked_path(str(path), directory=False)
    if path.stat().st_size > 512 * 1024 * 1024:
        raise ValueError("Toolchain anchor exceeds size limit")
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_json(path, *, limit=65536):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("Duplicate toolchain JSON key")
            result[key] = value
        return result

    if type(limit) is not int or not 0 < limit <= 16 * 1024 * 1024:
        raise ValueError("Invalid bounded JSON limit")
    path = checked_path(str(path), directory=False)
    with path.open("rb") as stream:
        data = stream.read(limit + 1)
    if len(data) > limit:
        raise ValueError("Toolchain JSON exceeds size limit")
    return json.loads(data.decode("utf-8"), object_pairs_hook=unique)


def layout(config):
    """仅支持显式 x86/x64 桌面工具链，不用存在性推断版本或架构。"""
    if not isinstance(config, dict) or set(config) != CONFIG_KEYS:
        raise ValueError("Unexpected toolchain configuration fields")
    if type(config["schemaVersion"]) is not int or config["schemaVersion"] != 1:
        raise ValueError("Unsupported toolchain schema")
    if config["sdkVersion"] != SDK_VERSION:
        raise ValueError("Pinned Windows SDK version required")
    if not isinstance(config["msvcVersion"], str) or not re.fullmatch(r"14\.\d+\.\d+", config["msvcVersion"]):
        raise ValueError("Explicit MSVC tools version required")
    architectures = config["architectures"]
    if not isinstance(architectures, list) or not architectures or any(
        not isinstance(cpu, str) or cpu not in ("x86", "x64") for cpu in architectures
    ) or len(set(architectures)) != len(architectures):
        raise ValueError("Unsupported or duplicate toolchain architecture")
    roots = {key: checked_path(config[key]) for key in (
        "visualStudioRoot", "sdkRoot", "systemRoot", "tempRoot"
    )}
    roots["msvc"] = checked_path(str(roots["visualStudioRoot"] / "VC/Tools/MSVC" / config["msvcVersion"]))
    roots["sdkInclude"] = checked_path(str(roots["sdkRoot"] / "Include" / SDK_VERSION))
    roots["sdkLib"] = checked_path(str(roots["sdkRoot"] / "Lib" / SDK_VERSION))
    checked_path(str(roots["systemRoot"] / "System32"))
    return roots


def paths_for(config, roots, cpu):
    if cpu not in config["architectures"]:
        raise ValueError("Architecture is not admitted by the toolchain manifest")
    includes = [roots["msvc"] / "include"] + [roots["sdkInclude"] / name for name in ("ucrt", "shared", "um", "winrt")]
    libraries = [roots["msvc"] / "lib" / cpu] + [roots["sdkLib"] / name / cpu for name in ("ucrt", "um")]
    binaries = [roots["msvc"] / "bin/Hostx64" / cpu, roots["sdkRoot"] / "bin" / SDK_VERSION / "x64"]
    for directory in includes + libraries + binaries:
        checked_path(str(directory))
    return includes, libraries, binaries


def anchors(config, roots):
    # 这些是错误版本/架构/关键工具替换检测点，不是完整 SDK/MSVC 内容闭包。
    result = {}
    for cpu in config["architectures"]:
        includes, libraries, binaries = paths_for(config, roots, cpu)
        files = [includes[0] / "vcruntime.h", includes[1] / "stdio.h", includes[3] / "Windows.h",
                 libraries[0] / "vcruntime.lib", libraries[0] / "libcmt.lib",
                 libraries[1] / "ucrt.lib", libraries[2] / "kernel32.lib",
                 binaries[0] / "cl.exe", binaries[0] / "link.exe",
                 binaries[1] / "rc.exe", binaries[1] / "mt.exe"]
        for path in files:
            result[str(path)] = file_hash(path)
    return result
