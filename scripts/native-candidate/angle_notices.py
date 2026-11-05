"""保留暂存许可证/声明与已审阅内联载体的原字节；名称扫描不是完整法律归属审计。"""

import re
from pathlib import PurePosixPath

from build_inputs import JSON_LIMIT, inventory_file
from toolchain_paths import read_json

MAX_TEXT = 4 * 1024 * 1024
MAX_TOTAL = 64 * 1024 * 1024
MAX_FILES = 2048
LICENSE_NAME = re.compile(r"(?i)^(?:licen[sc]es?|copying|copyright|notice|patents)(?:[._-]|$)")
# 这些实际编译文件含 Chromium attribution；仅有 zlib/ANGLE 根许可证不能代替它们。
INLINE_CARRIERS = (
    "src/common/base/anglebase/sha1.cc",
    "third_party/zlib/google/compression_utils_portable.cc",
    "third_party/zlib/crc32_simd.c",
    "third_party/zlib/adler32_simd.c",
)
GAPS = (
    "Staged name scan and reviewed inline carriers are not exhaustive per-file attribution.",
    "Chromium base/zlib refer to Chromium root LICENSE; canonical root-source attribution remains to be closed.",
    "Header-only, discarded objects, implicit MSVC/UCRT and compiler runtime inclusion require link/header evidence.",
    "SDK/MSVC redistribution rights and complete tool/source/license closure are not approved.",
)


def notice_records(records):
    result, total = [], 0
    for row in records:
        name = row["path"]
        if not name.startswith("src/"):
            continue
        relative = name[4:]
        parts = PurePosixPath(relative).parts
        selected = any(LICENSE_NAME.match(part) for part in parts) or parts[-1].lower() == "readme.chromium" or relative in INLINE_CARRIERS
        if not selected:
            continue
        if not 0 < row["bytes"] <= MAX_TEXT or (total := total + row["bytes"]) > MAX_TOTAL or len(result) >= MAX_FILES:
            raise ValueError("ANGLE notice material budget exceeded")
        result.append({"sourcePath": name, "path": "notices/" + relative, "bytes": row["bytes"],
                       "sha256": row["sha256"], "origin": row["origin"],
                       "selection": "reviewed-inline-carrier" if relative in INLINE_CARRIERS else "staged-name-scan"})
    if not any(row["sourcePath"] == "src/LICENSE" for row in result):
        raise ValueError("ANGLE root LICENSE material is required")
    return sorted(result, key=lambda row: row["path"])


def selected_notices(stage):
    result = notice_records(read_json(stage / "files.json", limit=JSON_LIMIT))
    for row in result:
        inventory_file(stage, row["sourcePath"], row["sha256"], row["bytes"])
        data = (stage / row["sourcePath"]).read_bytes()
        if data.startswith(b"\xef\xbb\xbf"):
            raise ValueError("ANGLE notice UTF-8 BOM requires separate preservation policy")
        data.decode("utf-8", errors="strict")
    return result
