"""只读 Git 对象流；不用 worktree/filter/hooks，不取网，不执行源码中的脚本。"""

import hashlib
import os
import re
import subprocess
import threading

from stage_inventory import MAX_FILE, relative_name
from toolchain_paths import checked_path


class GitObjects:
    def __init__(self, executable, source, system_root, *, timeout=300):
        self.executable = checked_path(str(executable), directory=False)
        self.database = checked_path(str(source / ".git"))
        checked_path(str(self.database / "objects"))
        for name in ("alternates", "http-alternates"):
            try:
                (self.database / "objects/info" / name).lstat()
            except FileNotFoundError:
                continue
            raise ValueError("External Git object alternates are not admitted")
        self.timeout = timeout
        self.process = None
        self.environment = {
            "SYSTEMROOT": str(system_root), "PATH": os.pathsep.join((str(self.executable.parent), str(system_root / "System32"))),
            "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_NO_REPLACE_OBJECTS": "1", "GIT_NO_LAZY_FETCH": "1", "GIT_TERMINAL_PROMPT": "0",
        }

    def __enter__(self):
        # Git 仅运行只读 cat-file；定时终止负责覆盖管道读阻塞，finally 回收进程和 timer。
        self.process = subprocess.Popen([str(self.executable), "--no-replace-objects", "--git-dir=" + str(self.database),
            "-c", "core.hooksPath=" + os.devnull, "-c", "protocol.allow=never", "cat-file", "--batch"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=self.environment,
            cwd=self.database.parent)
        try:
            self.timer = threading.Timer(self.timeout, self.process.kill)
            self.timer.daemon = True
            self.timer.start()
        except BaseException:
            self.close()
            raise
        return self

    def __exit__(self, *error):
        self.close()

    def close(self):
        timer = getattr(self, "timer", None)
        if timer:
            timer.cancel()
        if self.process.poll() is None:
            self.process.kill()
        self.process.wait(timeout=30)
        if timer and timer.ident is not None:
            timer.join()
        self.process.stdin.close()
        self.process.stdout.close()

    def header(self, oid, kind, limit):
        if not re.fullmatch("[0-9a-f]{40}", oid):
            raise ValueError("Invalid pinned Git object name")
        self.process.stdin.write(oid.encode() + b"\n")
        self.process.stdin.flush()
        header = self.process.stdout.readline(128)
        match = re.fullmatch(rb"([0-9a-f]{40}) (blob|tree|commit) ([0-9]+)\n", header)
        if not match or match[1].decode() != oid or match[2].decode() != kind:
            raise ValueError("Missing or unexpected Git object")
        size = int(match[3])
        if size > limit:
            raise ValueError("Git object exceeds bounded input limit")
        return size

    def object(self, oid, kind):
        size = self.header(oid, kind, 4 * 1024 * 1024)
        data = self.process.stdout.read(size)
        if len(data) != size or self.process.stdout.read(1) != b"\n":
            raise ValueError("Truncated Git object")
        if hashlib.sha1(kind.encode() + b" " + str(size).encode() + b"\0" + data).hexdigest() != oid:
            raise ValueError("Git object content identity mismatch")
        return data

    def blob(self, oid, writer, name, origin):
        size = self.header(oid, "blob", MAX_FILE)
        result = writer.write(name, self.process.stdout, size, origin, git_blob=oid)
        if self.process.stdout.read(1) != b"\n":
            raise ValueError("Invalid Git batch delimiter")
        return result


def tree_entries(data):
    index = 0
    seen = set()
    while index < len(data):
        end = data.find(b"\0", index)
        if end < 0 or end + 21 > len(data):
            raise ValueError("Malformed Git tree")
        mode, raw_name = data[index:end].split(b" ", 1)
        name = raw_name.decode("utf-8", errors="strict")
        if "/" in name or name.casefold() in seen:
            raise ValueError("Duplicate or nested Git tree name")
        relative_name(name)
        seen.add(name.casefold())
        yield mode.decode("ascii"), name, data[end + 1:end + 21].hex()
        index = end + 21


def materialize(reader, writer, pin):
    key, mount, revision, tree = pin
    commit = reader.object(revision, "commit")
    if not commit.startswith(b"tree " + tree.encode() + b"\n"):
        raise ValueError("Pinned Git commit/tree mismatch")
    omitted = []
    count = 0

    def visit(oid, prefix, depth):
        nonlocal count
        if depth > 64:
            raise ValueError("Git tree depth exceeded")
        for mode, name, child in tree_entries(reader.object(oid, "tree")):
            count += 1
            if count > 50000:
                raise ValueError("Git tree entry budget exceeded")
            target = prefix + "/" + name
            if mode == "40000":
                visit(child, target, depth + 1)
            elif mode in ("100644", "100755"):
                reader.blob(child, writer, target, {"source": key, "gitBlob": child, "gitMode": mode})
            elif mode == "160000":
                omitted.append({"path": target, "gitMode": mode, "gitObject": child})
            else:
                raise ValueError("Unsupported Git tree mode; links are never followed")

    visit(tree, "src" + ("/" + mount if mount else ""), 0)
    return commit, omitted
