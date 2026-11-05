"""暂存合同只写显式临时根；Git 为真实数据工具，合成编译工具永不执行。"""

import hashlib
import io
import json
import os
import shutil
import subprocess
import tarfile
import zipfile

from native_angle_fixture import fixture


def git_source(root):
    source = root / "source"
    source.mkdir()
    executable = shutil.which("git")
    if not executable:
        raise ValueError("Git is required for isolated staging contracts")
    env = {key: value for key, value in os.environ.items() if not key.upper().startswith("GIT_")}
    env.update({"GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull, "GIT_TERMINAL_PROMPT": "0"})

    def run(*args, data=None):
        result = subprocess.run([executable, *args], input=data, cwd=root, env=env, capture_output=True, timeout=30)
        result.check_returncode()
        return result.stdout.strip().decode("ascii")

    run("init", "--bare", str(source / ".git"))

    def obj(kind, data):
        return run("--git-dir=" + str(source / ".git"), "hash-object", "-w", "-t", kind, "--stdin", data=data)

    data = "原始 Git blob\n".encode()
    blob = obj("blob", data)
    nested = obj("tree", b"100644 file.txt\0" + bytes.fromhex(blob))
    tree = obj("tree", b"100644 data.txt\0" + bytes.fromhex(blob) + b"40000 nested\0" + bytes.fromhex(nested)
               + b"160000 stub\0" + bytes.fromhex("1" * 40))
    commit = obj("commit", (f"tree {tree}\nauthor Fixture <fixture@example.invalid> 1700000000 +0000\n"
        "committer Fixture <fixture@example.invalid> 1700000000 +0000\n\nFixture\nCr-Commit-Position: refs/heads/main@{#1}\n").encode())
    # 与对象内容故意不同；materialization 不能消费 worktree 或换行过滤器。
    (source / "data.txt").write_text("worktree must not be consumed", encoding="utf-8")
    return executable, source, commit, tree, data


def artifacts(root):
    paths = {}
    for key in ("gn", "ninja"):
        path = root / (key + ".zip")
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr(key + ".exe", "synthetic data; never execute")
            archive.writestr(".cipdpkg/manifest.json", "{}")
        paths[key] = path
    paths["clang"] = root / "clang.tar.xz"
    with tarfile.open(paths["clang"], "w:xz") as archive:
        for name, data in (("bin/clang-cl.exe", b"synthetic tool; never execute"),
                           ("cr_build_revision", b"llvmorg-23-init-10931-g20b6ec66-8\n")):
            member = tarfile.TarInfo(name)
            member.size = len(data)
            archive.addfile(member, io.BytesIO(data))
    paths["chromium-rc"] = root / "rc.exe"
    paths["chromium-rc"].write_bytes(b"synthetic tool; never execute")
    pins = {key: hashlib.sha256(path.read_bytes()).hexdigest() for key, path in paths.items()}
    return paths, pins


def stage_fixture():
    root, toolchain = fixture()
    toolchain_path = root / "toolchain.json"
    toolchain_path.write_text(json.dumps(toolchain, ensure_ascii=False), encoding="utf-8")
    executable, source, commit, tree, data = git_source(root)
    paths, pins = artifacts(root)
    config = {"schemaVersion": 1, "sourceRoots": {"angle": str(source), "build": str(source)},
              "artifacts": {key: str(path) for key, path in paths.items()}, "toolchainConfig": str(toolchain_path),
              "gitExecutable": executable}
    config_path = root / "stage-config.json"
    config_path.write_text(json.dumps(config, ensure_ascii=False), encoding="utf-8")
    sources = (("angle", "", commit, tree), ("build", "build", commit, tree))
    return root, config_path, config, sources, pins, data
