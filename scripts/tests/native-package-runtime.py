"""解包并验证当前内部产品包的实际启动；所有配置与运行证据写入独立目录。"""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import struct
import zipfile
import zlib


def fixture_png():
    def chunk(kind, data):
        return (struct.pack(">I", len(data)) + kind + data
                + struct.pack(">I", zlib.crc32(kind + data)))

    width, height = 96, 72
    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    pixels = (b"\0" + b"\x13\x67\xd3" * width) * height
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header)
            + chunk(b"IDAT", zlib.compress(pixels)) + chunk(b"IEND", b""))


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
    pin_image = root / "pinned-fixture.png"
    pin_bytes = fixture_png()
    pin_image.write_bytes(pin_bytes)
    # smoke 会批量创建/恢复窗口并执行完整功能检查；预算用于终止挂起，不是冷启动性能阈值。
    # 参数拒绝不进入这些检查，继续保留较短预算；所有路径仍核退出码、成功标记和进程树清理。
    for label, arguments, expected, markers, timeout in [
        ("smoke", ["--smoke-test"], 0, ("destroyed=true",), 60),
        ("pinned-image-smoke", [str(pin_image), "--pin", "--smoke-test"], 0, (
            "native-document-owner-smoke: relocated=true state-stable=true icons=true",
            "native-edit-strip-smoke: button=true selection=true silent=true",
            "native-drop-smoke: shell=true worker=true timer=true source-preserved=true",
            "native-marquee-smoke: pointer=true shift=true capture=true escape=true wheel=true undo=true",
            "native-window-smoke: pinned=true",
        ), 60),
        ("pin-missing-image", ["--pin"], 1, ("requires an image",), 15),
        ("unknown-option", ["--unknown-option"], 1, ("Usage:",), 15),
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
            result = job.run_job([str(exe), *arguments], work, env, out, err, timeout=timeout)
        output = (work / ("stdout.log" if expected == 0 else "stderr.log")).read_text(encoding="utf-8")
        missing = [marker for marker in markers if marker not in output]
        row = dict(label=label, **result, timeoutSeconds=timeout,
                   expectedExitCode=expected, markerFound=not missing, missingMarkers=missing)
        if expected == 0:
            row["sessionFixtureResidues"] = len(list(temp.glob("otter-session-fixture-*")))
        if label == "pinned-image-smoke":
            row["sourceUnchanged"] = pin_image.is_file() and pin_image.read_bytes() == pin_bytes
        checks.append(row)
        print(json.dumps(row), flush=True)
        if (result["exitCode"] != expected or result["status"] != "exited"
                or missing or row.get("sessionFixtureResidues", 0)
                or not row.get("sourceUnchanged", True)):
            raise RuntimeError(f"{label} failed; see {work}")
    receipt = dict(executableSha256=digest, archiveSha256=hashlib.sha256(archive.read_bytes()).hexdigest(),
                   unpackedProduct=str(exe), checks=checks, visibleMenuTested=False)
    (root / "runtime.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
