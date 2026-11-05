"""构建合同只使用显式外部合成 stage/PE 数据；合成编译工具绝不执行。"""

import json
import struct
import sys
from pathlib import Path

from native_angle_fixture import fixture
from build_inputs import own_sources
from gn_profile import PROFILE
from otter_toolchain_env import create_manifest
from stage_pins import SOURCES
from toolchain_paths import file_hash


def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def build_fixture():
    root, config = fixture()
    stage = root / "stage"
    stage.mkdir()
    rows = []
    for name in ("src/out/otter/args.gn", "src/main.cc", "tools/gn.exe", "tools/ninja.exe"):
        path = stage / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b"synthetic data; never execute\n")
        rows.append({"path": name, "bytes": path.stat().st_size, "sha256": file_hash(path), "origin": {"fixture": True}})
    (stage / "profile").mkdir()
    save(stage / "profile/environment.json", create_manifest(config))
    save(stage / "profile/adapter.json", {"fixture": True})
    save(stage / "profile/profile.json", {"kind": "otter-angle-gn-profile", "profile": PROFILE,
        "python": {"path": str(Path(sys.executable).resolve()), "sha256": file_hash(Path(sys.executable).resolve())},
        "adapterSha256": file_hash(stage / "profile/adapter.json"),
        "files": [{"path": "environment.json", "sha256": file_hash(stage / "profile/environment.json")}]})
    save(stage / "files.json", rows)
    save(stage / "gitlinks.json", [])
    receipt = {"schemaVersion": 1, "kind": "otter-angle-fixed-build-stage", "profile": PROFILE,
        "sources": [{"key": row[0], "mount": row[1], "commit": row[2], "tree": row[3]} for row in SOURCES],
        "stagingSourceSha256": own_sources(), "fileCount": len(rows), "gitlinkCount": 0, "unmaterializedGitlinks": 0,
        "filesSha256": file_hash(stage / "files.json"), "gitlinksSha256": file_hash(stage / "gitlinks.json"),
        "profileSha256": file_hash(stage / "profile/profile.json"), "gitExecuted": True,
        **{name: False for name in ("buildToolsExecuted", "hooksExecuted", "downloaded", "completeSourceClosure",
            "completeToolchainClosure", "runtimeActivated", "releaseApproved")}}
    save(stage / "stage.json", receipt)
    return root, config, stage, file_hash(stage / "stage.json")


def pe(path, *, function_count=1, ordinal=0, function_rva=0x1400, delay=None, executable=True, data_export=False):
    data = bytearray(0x800)

    def pack(pattern, offset, *values):
        struct.pack_into(pattern, data, offset, *values)

    data[:2] = b"MZ"
    pack("<I", 0x3c, 0x80)
    data[0x80:0x84] = b"PE\0\0"
    pack("<HHIIIHH", 0x84, 0x8664, 2 if data_export else 1, 0, 0, 0, 240, 0x2000)
    optional = 0x98
    pack("<H", optional, 0x20b)
    pack("<I", optional + 108, 16)
    pack("<II", optional + 112, 0x1000, 0x80)
    pack("<II", optional + 120, 0x1100, 40)
    pack("<8sIIIIIIHHI", optional + 240, b".rdata\0\0", 0x600, 0x1000, 0x600, 0x200, 0, 0, 0, 0, 0x60000020 if executable else 0x40000040)
    pack("<IIIII", 0x214, 2 if data_export else function_count, 2 if data_export else 1, 0x1080, 0x1090, 0x1098)
    pack("<I", 0x280, function_rva)
    pack("<I", 0x290, 0x1200)
    pack("<H", 0x298, ordinal)
    pack("<IIIII", 0x300, 0x1280, 0, 0, 0x1240, 0x1280)
    data[0x400:0x40b] = b"eglExample\0"
    data[0x440:0x44d] = b"KERNEL32.dll\0"
    if data_export:
        pack("<8sIIIIIIHHI", optional + 280, b".data\0\0\0", 0x100, 0x2000, 0x100, 0x700, 0, 0, 0, 0, 0x40000040)
        pack("<I", 0x284, 0x2000)
        pack("<I", 0x294, 0x1420)
        pack("<H", 0x29a, 1)
        data[0x620:0x62d] = b"optionalData\0"
    if delay:
        pack("<II", optional + 112 + 13 * 8, 0x1300, 64)
        pack("<IIIIIIII", 0x500, 1, 0x1380, 0, 0, 0, 0, 0, 0)
        encoded = delay.encode("ascii") + b"\0"
        data[0x580:0x580 + len(encoded)] = encoded
    path.write_bytes(data)
    return path
