"""仅展开已钉摘要的工具归档；拒绝链接、路径别名及容量扩张，不执行工具。"""

import tarfile
import zipfile

from stage_inventory import relative_name
from stage_pins import ARTIFACTS
from toolchain_paths import file_hash


def tools(paths, writer):
    for key, expected in ARTIFACTS.items():
        if file_hash(paths[key]) != expected:
            raise ValueError(f"Pinned tool artifact identity mismatch: {key}")
    for key in ("gn", "ninja"):
        with zipfile.ZipFile(paths[key]) as archive:
            members = archive.infolist()
            if len(members) > 32 or len({entry.filename.casefold() for entry in members}) != len(members):
                raise ValueError("Unexpected tool ZIP inventory")
            for entry in members:
                relative_name(entry.filename.rstrip("/"))
                if entry.flag_bits & 1 or (entry.external_attr >> 16) & 0o170000 == 0o120000:
                    raise ValueError("Linked or encrypted tool ZIP entry")
            entry = archive.getinfo(key + ".exe")
            with archive.open(entry) as source:
                writer.write("tools/" + key + ".exe", source, entry.file_size, {"artifact": key, "archiveSha256": ARTIFACTS[key]})
    prefix = "src/third_party/llvm-build/Release+Asserts/"
    with tarfile.open(paths["clang"], "r:xz") as archive:
        for index, entry in enumerate(archive):
            if index >= 1000 or entry.linkname or not (entry.isdir() or entry.isfile()):
                raise ValueError("Unsupported Clang archive entry")
            name = entry.name.removeprefix("./").rstrip("/")
            relative_name(name)
            target = prefix + name
            if entry.isdir():
                writer.claim(target, directory=True).mkdir(parents=True, exist_ok=True)
            else:
                with archive.extractfile(entry) as source:
                    writer.write(target, source, entry.size, {"artifact": "clang", "archiveSha256": ARTIFACTS["clang"]})
    with paths["chromium-rc"].open("rb") as source:
        writer.write("src/build/toolchain/win/rc/win/rc.exe", source, paths["chromium-rc"].stat().st_size,
                     {"artifact": "chromium-rc", "sha256": ARTIFACTS["chromium-rc"]})
    marker = writer.root / (prefix + "cr_build_revision")
    if marker.read_text("utf-8").strip() != "llvmorg-23-init-10931-g20b6ec66-8":
        raise ValueError("Clang build revision mismatch")
    if any(file_hash(paths[key]) != expected for key, expected in ARTIFACTS.items()):
        raise ValueError("Tool artifacts changed during staging")
