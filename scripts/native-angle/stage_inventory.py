"""暂存目录的 Windows 名称空间、独占写入、容量及逐文件内容身份 owner。"""

import hashlib
import io
import re
from pathlib import PurePosixPath

MAX_FILE = 512 * 1024 * 1024
MAX_TOTAL = 2 * 1024 * 1024 * 1024
MAX_FILES = 30000


def relative_name(value):
    if not isinstance(value, str) or not value or value.startswith("/"):
        raise ValueError("Invalid staging relative path")
    parts = value.split("/")
    for part in parts:
        if not part or part in (".", "..") or part.endswith((".", " ")) or re.search(r'[\x00-\x1f<>:"\\|?*]', part):
            raise ValueError("Unsafe staging path component")
        if part.casefold() == ".git" or re.fullmatch(r"(?i)(?:con|prn|aux|nul|conin\$|conout\$|com[1-9¹²³]|lpt[1-9¹²³])(?:\..*)?", part):
            raise ValueError("Reserved staging path component")
    return PurePosixPath(value)


class Materializer:
    def __init__(self, root):
        self.root = root
        self.files = {}
        self.names = {}
        self.total = 0

    def claim(self, name, *, replace=False, directory=False):
        relative = relative_name(name)
        current = ""
        for index, part in enumerate(relative.parts):
            current = (current + "/" if current else "") + part
            key = current.casefold()
            is_directory = directory or index < len(relative.parts) - 1
            previous = self.names.get(key)
            if previous and (previous != (current, is_directory) or not is_directory and not replace):
                raise ValueError(f"Staging namespace collision: {name}")
            self.names[key] = (current, is_directory)
        if replace and name not in self.files:
            raise ValueError("Only inventoried files may be replaced")
        return self.root.joinpath(*relative.parts)

    def write(self, name, stream, size, origin, *, git_blob=None, replace=False):
        if type(size) is not int or size < 0 or size > MAX_FILE or self.total + size > MAX_TOTAL:
            raise ValueError("Staging byte budget exceeded")
        if not replace and len(self.files) >= MAX_FILES:
            raise ValueError("Staging file count exceeded")
        path = self.claim(name, replace=replace)
        path.parent.mkdir(parents=True, exist_ok=True)
        digest = hashlib.sha256()
        git_digest = hashlib.sha1(b"blob " + str(size).encode() + b"\0") if git_blob else None
        remaining = size
        with path.open("wb" if replace else "xb") as output:
            while remaining:
                block = stream.read(min(1024 * 1024, remaining))
                if not block or len(block) > remaining:
                    raise ValueError("Truncated or oversized staged stream")
                output.write(block)
                digest.update(block)
                if git_digest:
                    git_digest.update(block)
                remaining -= len(block)
        if git_digest and git_digest.hexdigest() != git_blob:
            raise ValueError("Git blob content identity mismatch")
        record = {"path": name, "bytes": size, "sha256": digest.hexdigest(), "origin": origin}
        if replace:
            record["replacedSha256"] = self.files[name]["sha256"]
        self.files[name] = record
        self.total += size
        return record

    def bytes(self, name, data, origin, *, replace=False):
        return self.write(name, io.BytesIO(data), len(data), origin, replace=replace)
