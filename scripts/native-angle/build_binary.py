"""纯字节检查实际 x64 DLL 的 PE、必需导出及静态 CRT 导入边界；不加载 DLL。"""

import re
import struct
from pathlib import Path

from toolchain_paths import file_hash


class Image:
    def __init__(self, path):
        self.sha256 = file_hash(path)
        self.data = path.read_bytes()
        if self.data[:2] != b"MZ":
            raise ValueError("Missing PE DOS header")
        header = self.number(0x3c)
        if self.data[header:header + 4] != b"PE\0\0":
            raise ValueError("Missing PE signature")
        machine, count, _, _, _, optional, flags = self.unpack("<HHIIIHH", header + 4)
        if machine != 0x8664 or not 0 < count <= 96 or flags & 0x2000 == 0 or optional < 128:
            raise ValueError("Expected bounded x64 PE DLL")
        self.optional = header + 24
        self.directory_count = self.number(self.optional + 108)
        if self.unpack("<H", self.optional)[0] != 0x20b or not 2 <= self.directory_count <= 16 or optional < 112 + self.directory_count * 8:
            raise ValueError("Expected PE32+ data directories")
        self.sections = [self.unpack("<8sIIIIIIHHI", self.optional + optional + index * 40) for index in range(count)]

    def unpack(self, pattern, offset):
        if type(offset) is not int or offset < 0 or offset + struct.calcsize(pattern) > len(self.data):
            raise ValueError("PE range out of bounds")
        return struct.unpack_from(pattern, self.data, offset)

    def number(self, offset):
        return self.unpack("<I", offset)[0]

    def rva(self, value, size=1, *, executable=False):
        found = []
        for _, virtual_size, address, raw_size, raw, _, _, _, _, flags in self.sections:
            if address <= value and value + size <= address + min(virtual_size, raw_size):
                offset = raw + value - address
                if offset + size <= len(self.data):
                    found.append((offset, flags))
        if len(found) != 1:
            raise ValueError("Unmapped or ambiguous PE RVA")
        if executable and found[0][1] & 0x20000000 == 0:
            raise ValueError("Required PE function export is not executable")
        return found[0][0]

    def string(self, value):
        offset = self.rva(value)
        end = self.data.find(b"\0", offset, offset + 4096)
        if end < 0:
            raise ValueError("Unterminated PE string")
        self.rva(value, end - offset + 1)
        return self.data[offset:end].decode("ascii", errors="strict")

    def exports(self):
        address, size = self.unpack("<II", self.optional + 112)
        if size < 40:
            raise ValueError("Truncated PE export directory")
        header = self.rva(address, 40)
        functions, count, function_table, table, ordinals = self.unpack("<IIIII", header + 20)
        if not 0 < count <= functions <= 10000:
            raise ValueError("Invalid PE export count")
        function_offset = self.rva(function_table, functions * 4)
        ordinal_offset = self.rva(ordinals, count * 2)
        offset = self.rva(table, count * 4)
        names = {}
        for index in range(count):
            ordinal = self.unpack("<H", ordinal_offset + index * 2)[0]
            if ordinal >= functions:
                raise ValueError("Invalid PE export ordinal")
            target = self.number(function_offset + ordinal * 4)
            if not target or address <= target < address + size:
                raise ValueError("Empty or forwarded PE exports are not admitted")
            self.rva(target)
            name = self.string(self.number(offset + index * 4))
            if name in names:
                raise ValueError("Duplicate PE exports")
            names[name] = target
        return names

    def import_directory(self, index, width):
        address, size = self.unpack("<II", self.optional + 112 + index * 8)
        if address == size == 0:
            return []
        if not address or not width <= size <= 256 * width:
            raise ValueError("Invalid PE import directory size")
        result = []
        for row in range(size // width):
            fields = self.unpack("<" + "I" * (width // 4), self.rva(address + row * width, width))
            if not any(fields):
                return result
            if width == 32 and fields[0] != 1:
                raise ValueError("Only RVA-based PE delay imports are admitted")
            result.append(self.string(fields[1 if width == 32 else 3]))
        raise ValueError("Unterminated PE import directory")

    def imports(self):
        return self.import_directory(1, 20) + (self.import_directory(13, 32) if self.directory_count > 13 else [])


def verify_dll(path, required, system_root):
    image = Image(path)
    exports = image.exports()
    if not required or not set(required) <= exports.keys():
        raise ValueError("Missing required Otter DLL exports")
    # 仅必需 API 必须指向可执行节；其他合法数据导出不被误当作函数。
    for name in required:
        image.rva(exports[name], executable=True)
    imports = image.imports()
    for name in imports:
        if not re.fullmatch(r"[\w.-]+\.dll", name, re.I) or re.match(r"(?:msvcp|vcruntime|ucrtbase|msvcr|api-ms-win-crt-)", name, re.I):
            raise ValueError("Invalid or dynamic MSVC CRT DLL import")
        if name.lower() not in ("libglesv2.dll", "api-ms-win-core-synch-l1-2-0.dll") and not (Path(system_root) / "System32" / name).is_file():
            raise ValueError("Unavailable system DLL dependency")
    if image.sha256 != file_hash(path):
        raise ValueError("DLL changed during PE verification")
    return {"path": path.name, "bytes": len(image.data), "sha256": image.sha256, "machine": "x64",
            "requiredExportCount": len(required), "dependencies": imports, "dynamicMsvcCrt": False}
