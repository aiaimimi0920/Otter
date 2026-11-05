"""独立核验外部预期摘要绑定的本地材料集合；不需要原 stage、VS 或 SDK 目录。"""

import os
from pathlib import Path, PurePosixPath

# 初始化唯一构建 owner 的 import 路径；不执行任何构建工具。
import angle_intake
from angle_materials_evidence import FALSE_BOUNDARIES, SBOMS, expected_evidence
from angle_notices import GAPS
from angle_sbom import documents
from build_inputs import JSON_LIMIT, inventory_file
from stage_inventory import Materializer
from toolchain_paths import checked_path, file_hash, read_json

MAX_FILES = 4096
MAX_FILE = 16 * 1024 * 1024
MAX_TOTAL = 256 * 1024 * 1024
SCOPE = "local-angle-materials-not-runtime-dependency-closure"


def verify(output, expected):
    output = checked_path(str(output))
    inventory_file(output, "materials.json", expected)
    manifest = read_json(output / "materials.json", limit=JSON_LIMIT)
    fields = {"schemaVersion", "kind", "scope", "stageSha256", "buildSha256", "producerSourceSha256",
        "files", "notices", "gaps", "unmaterializedGitlinks", *FALSE_BOUNDARIES}
    if set(manifest) != fields or type(manifest["schemaVersion"]) is not int or manifest["schemaVersion"] != 1 or manifest["kind"] != "otter-angle-local-materials" or manifest["scope"] != SCOPE:
        raise ValueError("Unsupported ANGLE materials manifest")
    if any(manifest[key] is not False for key in FALSE_BOUNDARIES) or manifest["gaps"] != list(GAPS):
        raise ValueError("ANGLE materials falsely approve an unresolved boundary")
    rows = manifest["files"]
    if not isinstance(rows, list) or not 1 <= len(rows) <= MAX_FILES:
        raise ValueError("Invalid materials file count")
    namespace, total, hashes, directories = Materializer(output), 0, {}, set()
    namespace.claim("materials.json")
    for row in rows:
        if not isinstance(row, dict) or set(row) != {"path", "bytes", "sha256"}:
            raise ValueError("Invalid materials file record")
        name, size = row["path"], row["bytes"]
        namespace.claim(name)
        if type(size) is not int or not 0 <= size <= MAX_FILE or (total := total + size) > MAX_TOTAL:
            raise ValueError("Materials byte budget exceeded")
        inventory_file(output, name, row["sha256"], size)
        hashes[name] = row["sha256"]
        directories.update(str(parent) for parent in PurePosixPath(name).parents if str(parent) != ".")
    if rows != sorted(rows, key=lambda row: row["path"]):
        raise ValueError("Materials records must be sorted")
    actual, actual_directories = set(), set()
    for base, dirs, files in os.walk(output, followlinks=False):
        for name in [*dirs, *files]:
            path = checked_path(str(Path(base) / name), directory=name in dirs)
            relative = path.relative_to(output).as_posix()
            (actual_directories if name in dirs else actual).add(relative)
        if len(actual) + len(actual_directories) > MAX_FILES * 8:
            raise ValueError("Unexpected materials tree exceeds budget")
    if actual != set(hashes) | {"materials.json"} or actual_directories != directories:
        raise ValueError("Unexpected or missing materials entries")
    evidence = expected_evidence(output, manifest)
    if hashes != {**evidence, **{name: hashes[name] for name in SBOMS}}:
        raise ValueError("Materials files differ from explicit evidence allowlist")
    for row in manifest["notices"]:
        data = (output / row["path"]).read_bytes()
        if data.startswith(b"\xef\xbb\xbf"):
            raise ValueError("Notice BOM is not supported")
        data.decode("utf-8", errors="strict")
    cdx = read_json(output / "sbom.cdx.json", limit=JSON_LIMIT)
    records = [row for row in rows if row["path"] not in SBOMS]
    sboms = documents(records, manifest["buildSha256"], stamp=cdx["metadata"]["timestamp"], serial=cdx["serialNumber"])
    if any(read_json(output / name, limit=JSON_LIMIT) != value for name, value in sboms.items()):
        raise ValueError("Materials SBOM inventory or scope mismatch")
    if file_hash(output / "materials.json") != expected:
        raise ValueError("Materials manifest changed during verification")
    return manifest
