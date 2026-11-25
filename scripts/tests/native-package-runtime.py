"""Unpack and verify actual startup of the current internal product package; write all configuration and runtime evidence to an isolated directory."""

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
    assets_dir = root / "assets"
    assets_dir.mkdir()
    pin_image = assets_dir / "pinned-fixture.png"
    pin_bytes = fixture_png()
    pin_image.write_bytes(pin_bytes)
    session_file = root / "pinned-session.json"
    session_bytes = json.dumps({
        "stickers": [{
            "id": "pinned-fixture",
            "x": 32,
            "y": 32,
            "w": 96,
            "h": 72,
            "src": str(pin_image),
            "filePath": str(pin_image),
        }],
        "links": [],
        "otterNativeCoordinateSpace": {"version": 1, "origin": [0, 0], "scale": 1},
    }).encode("utf-8")
    session_file.write_bytes(session_bytes)
    # Release packages exclude development smoke modules; hidden read-only startup probes verify the actual EXE,
    # D2D drawing/device recreation and external-asset decoding, plus expected rejection of --smoke-test and invalid arguments.
    for label, arguments, expected, markers, timeout in [
        ("workspace-check", ["--workspace-check"], 0, (
            "native-workspace-startup: loaded=false readonly=false manual-target=false",
            "native-session-check: windows=1",
            "native-session-render-check: paints=2 device-recreated=true",
        ), 60),
        ("pinned-session-check", [
            "--session", str(session_file),
            "--session-assets", str(assets_dir),
            "--session-check",
        ], 0, (
            "native-session-check: windows=1",
            "native-session-render-check: paints=2 device-recreated=true",
        ), 60),
        ("smoke-disabled", ["--smoke-test"], 1, ("disabled in release builds",), 15),
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
        # Start only this run's unpacked artifact; the existing Job owner reclaims its process tree on failure or timeout.
        with (work / "stdout.log").open("xb") as out, (work / "stderr.log").open("xb") as err:
            result = job.run_job([str(exe), *arguments], work, env, out, err, timeout=timeout)
        output = (work / ("stdout.log" if expected == 0 else "stderr.log")).read_text(encoding="utf-8")
        missing = [marker for marker in markers if marker not in output]
        row = dict(label=label, **result, timeoutSeconds=timeout,
                   expectedExitCode=expected, markerFound=not missing, missingMarkers=missing)
        if expected == 0:
            row["sessionFixtureResidues"] = len(list(temp.glob("otter-session-fixture-*")))
            row["workspaceUnwritten"] = not (profile / "workspace.json").exists()
        if label == "pinned-session-check":
            row["sourceUnchanged"] = (pin_image.is_file() and pin_image.read_bytes() == pin_bytes
                                      and session_file.is_file()
                                      and session_file.read_bytes() == session_bytes)
        checks.append(row)
        print(json.dumps(row), flush=True)
        if (result["exitCode"] != expected or result["status"] != "exited"
                or missing or row.get("sessionFixtureResidues", 0)
                or not row.get("workspaceUnwritten", True)
                or not row.get("sourceUnchanged", True)):
            raise RuntimeError(f"{label} failed; see {work}")
    receipt = dict(executableSha256=digest, archiveSha256=hashlib.sha256(archive.read_bytes()).hexdigest(),
                   unpackedProduct=str(exe), checks=checks, visibleMenuTested=False)
    (root / "runtime.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
