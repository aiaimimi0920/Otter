"""解包并验证当前内部产品包的实际启动；所有配置与运行证据写入独立目录。"""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package-dir", required=True, type=Path)
    parser.add_argument("--evidence-dir", required=True, type=Path)
    args = parser.parse_args()
    package = args.package_dir.resolve(strict=True)
    root = args.evidence_dir.resolve()
    root.mkdir(parents=True, exist_ok=False)
    verification = json.loads((package / "archive-verification.json").read_text(encoding="utf-8-sig"))
    archives = list(package.glob("Otter-native-*-windows-x64-internal.zip"))
    if len(archives) != 1:
        raise ValueError("Expected one verified native package archive")
    archive = archives[0]
    unpacked = root / "unpacked"
    unpacked.mkdir()
    with zipfile.ZipFile(archive) as bundle:
        for name in bundle.namelist():
            if not (unpacked / name).resolve().is_relative_to(unpacked):
                raise ValueError("Archive entry escapes the isolated destination")
        bundle.extractall(unpacked)
    exe = unpacked / "otter-native.exe"
    digest = hashlib.sha256(exe.read_bytes()).hexdigest()
    if digest.lower() != verification["executableSha256"].lower():
        raise ValueError("Extracted executable does not match archive verification")
    if not (unpacked / "otter_live_hover_hook.dll").is_file():
        raise ValueError("Native hover DLL is missing")
    repo = Path(__file__).resolve().parents[2]
    spec = importlib.util.spec_from_file_location("build_job", repo / "scripts/native-angle/build_job.py")
    job = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(job)
    checks = []
    for label, arguments, expected, marker in [
        ("smoke", ["--smoke-test"], 0, "destroyed=true"),
        ("pin-missing-image", ["--pin"], 1, "requires an image"),
        ("unknown-option", ["--unknown-option"], 1, "Usage:"),
    ]:
        work = root / label
        work.mkdir()
        temp, profile = work / "temp", work / "profile"
        temp.mkdir()
        profile.mkdir()
        env = {key: value for key, value in os.environ.items()
               if not key.upper().startswith(("OTTER_", "LOOM_"))}
        env.update(TEMP=str(temp), TMP=str(temp), OTTER_NATIVE_PROFILE_DIR=str(profile))
        # 只启动本次解包产物，失败和超时由已有 Job owner 收回进程树。
        with (work / "stdout.log").open("xb") as out, (work / "stderr.log").open("xb") as err:
            result = job.run_job([str(exe), *arguments], work, env, out, err, timeout=15)
        output = (work / ("stdout.log" if expected == 0 else "stderr.log")).read_text(encoding="utf-8")
        row = dict(label=label, **result, expectedExitCode=expected, markerFound=marker in output)
        checks.append(row)
        print(json.dumps(row), flush=True)
        if result["exitCode"] != expected or result["status"] != "exited" or marker not in output:
            raise RuntimeError(f"{label} failed; see {work}")
    receipt = dict(executableSha256=digest, archiveSha256=hashlib.sha256(archive.read_bytes()).hexdigest(),
                   unpackedProduct=str(exe), checks=checks, visibleMenuTested=False)
    (root / "runtime.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
