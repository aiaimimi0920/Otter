"""工具链合同的合成头/库/EXE 均是文本，绝不作为编译工具执行；证据留在显式临时根。"""

import json
import os
import sys
import tempfile
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS / "native-angle"))
from toolchain_paths import SDK_VERSION, paths_for, layout


def fixture():
    base = os.environ.get("OTTER_ANGLE_TEST_ROOT")
    if not base or not Path(base).is_absolute():
        raise ValueError("Set absolute OTTER_ANGLE_TEST_ROOT outside the repository")
    base = Path(base)
    if base.is_relative_to(SCRIPTS.parent):
        raise ValueError("Synthetic toolchain fixtures must stay outside the repository")
    base.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="angle-contract-", dir=base))
    config = {
        "schemaVersion": 1, "visualStudioRoot": str(root / "Visual Studio"),
        "msvcVersion": "14.44.35207", "sdkRoot": str(root / "SDK 受限数据"),
        "sdkVersion": SDK_VERSION, "architectures": ["x86", "x64"],
        "systemRoot": str(root / "Windows"), "tempRoot": str(root / "temp"),
    }
    msvc = Path(config["visualStudioRoot"]) / "VC/Tools/MSVC" / config["msvcVersion"]
    sdk = Path(config["sdkRoot"])
    directories = [msvc / "include", Path(config["systemRoot"]) / "System32", Path(config["tempRoot"])]
    directories += [sdk / "Include" / SDK_VERSION / name for name in ("ucrt", "shared", "um", "winrt")]
    for cpu in config["architectures"]:
        directories += [msvc / "lib" / cpu, msvc / "bin/Hostx64" / cpu]
        directories += [sdk / "Lib" / SDK_VERSION / name / cpu for name in ("ucrt", "um")]
    directories += [sdk / "bin" / SDK_VERSION / "x64"]
    for directory in directories:
        directory.mkdir(parents=True, exist_ok=True)
    batch = Path(config["visualStudioRoot"]) / "VC/Auxiliary/Build/vcvarsall.bat"
    batch.parent.mkdir(parents=True)
    batch.write_text("synthetic discovery tripwire; never execute\n", encoding="utf-8")
    roots = layout(config)
    for cpu in config["architectures"]:
        includes, libraries, binaries = paths_for(config, roots, cpu)
        files = [includes[0] / "vcruntime.h", includes[1] / "stdio.h", includes[3] / "Windows.h",
                 libraries[0] / "vcruntime.lib", libraries[0] / "libcmt.lib",
                 libraries[1] / "ucrt.lib", libraries[2] / "kernel32.lib",
                 binaries[0] / "cl.exe", binaries[0] / "link.exe", binaries[1] / "rc.exe", binaries[1] / "mt.exe"]
        for path in files:
            path.write_text("synthetic toolchain data; never execute\n", encoding="utf-8")
    return root, config


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return str(path)
