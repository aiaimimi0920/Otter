"""Run the explicit hidden Otter color-transfer gate against an isolated Loom daemon."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import secrets
import sqlite3
import struct
import subprocess
import time
from urllib.error import HTTPError
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener, ProxyHandler
import tempfile
import zipfile
import zlib

ART = "custom-1770131241684"
EXACT = ("desktop::application::execution_start::tests::shader::color_transfer::"
         "installed_loom_color_transfer_commits_native_png")
REPO_ROOT = Path(__file__).resolve().parents[2]
SCRATCH = REPO_ROOT / ".tmp"
FILES = ("manifest.json", "art.runtime.json", "runtime/main.py", "runtime/shader_payload.py",
         "runtime/color_transfer.hlsl", "runtime/color_transfer.vert", "runtime/color_transfer.frag")


def sha(data):
    return hashlib.sha256(data).hexdigest()


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def write_input(path):
    def chunk(tag, data):
        return (struct.pack(">I", len(data)) + tag + data
                + struct.pack(">I", zlib.crc32(tag + data)))

    rgba = bytes([255, 128, 64, 128, 16, 255, 77, 0, 1, 2, 3, 255])
    path.write_bytes(b"\x89PNG\r\n\x1a\n"
                     + chunk(b"IHDR", struct.pack(">IIBBBBB", 3, 1, 8, 6, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(b"\x00" + rgba))
                     + chunk(b"IEND", b""))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ("loom_root", "daemon", "framework", "otter_test_exe"):
        parser.add_argument(arg, type=Path)
    args = parser.parse_args()
    for name, path in vars(args).items():
        resolved = path.resolve(strict=True)
        if resolved.drive.upper() != "C:":
            parser.error(f"{name} must resolve to the local C: drive")
        setattr(args, name, resolved)
    scratch = SCRATCH.resolve()
    if scratch == REPO_ROOT or not scratch.is_relative_to(REPO_ROOT) or scratch.drive.upper() != "C:":
        parser.error("Repository .tmp must resolve inside the local C: repository")
    scratch.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="otter-color-transfer-", dir=scratch))
    manifest = root / "discovery/loom.json"
    token = secrets.token_hex(32)
    sample = args.loom_root / "art-packages/samples/color-transfer"
    print(root, flush=True)
    for name in ("tmp", "discovery", "configuration"):
        (root / name).mkdir()
    archive = root / "color-transfer.zip"
    contents = {name: (sample / name).read_bytes() for name in FILES}
    with zipfile.ZipFile(archive, "x", zipfile.ZIP_DEFLATED) as out:
        for name, data in contents.items():
            out.writestr(zipfile.ZipInfo(name), data, compress_type=zipfile.ZIP_DEFLATED)
    art = archive.read_bytes()
    framework = args.framework.read_bytes()
    receipt = {"artSha256": sha(art), "frameworkSha256": sha(framework),
               "files": {name: sha(data) for name, data in contents.items()}}
    env = {k: v for k, v in os.environ.items() if not k.upper().startswith(
        ("LOOM_", "OTTER_", "NEURO_"))}
    env.update(TEMP=str(root / "tmp"), TMP=str(root / "tmp"),
               LOOM_DAEMON_HOST="127.0.0.1", LOOM_DAEMON_PORT="0", LOOM_DAEMON_TOKEN=token,
               LOOM_CONTROL_PLANE_ROOT=str(root / "control"),
               LOOM_CONFIGURATION_ROOT=str(root / "configuration"),
               LOOM_RUN_STORE_PATH=str(root / "runs.sqlite3"),
               LOOM_CAPABILITY_MANIFEST_DIR=str(manifest.parent),
               LOOM_BUNDLED_ART_SHA256_ALLOWLIST=sha(art))
    with args.daemon.open("rb") as exe:
        receipt["daemonSha256"] = hashlib.file_digest(exe, "sha256").hexdigest()
    with args.otter_test_exe.open("rb") as exe:
        receipt["otterSha256"] = hashlib.file_digest(exe, "sha256").hexdigest()
    child = None
    bridge = False
    try:
        with (root / "daemon.log").open("wb") as log:
            child = subprocess.Popen([args.daemon], cwd=root, env=env, stdout=log, stderr=log)
            deadline = time.monotonic() + 20
            while not manifest.is_file():
                assert child.poll() is None and time.monotonic() < deadline
                time.sleep(.05)
            doc = json.loads(manifest.read_text(encoding="utf-8"))
            url = doc["transport"]["baseUrl"]
            parsed = urlsplit(url)
            assert child.pid == doc["pid"]
            assert parsed.scheme == "http" and parsed.hostname == "127.0.0.1" and parsed.port
            assert not (parsed.username or parsed.password or parsed.path or parsed.query or parsed.fragment)
            opener = build_opener(ProxyHandler({}), NoRedirect())

            def api(path, request):
                req = Request(url + path, json.dumps(request).encode(), {
                    "Authorization": "Bearer " + token, "Content-Type": "application/json"
                }, method="POST")
                try:
                    with opener.open(req, timeout=40) as reply:
                        return json.load(reply)
                except HTTPError as error:
                    raise RuntimeError(f"{path}: {error.code}: {error.read(4096)!r}") from None

            receipt["framework"] = api("/v1/frameworks/install", {
                "zipBase64": base64.b64encode(framework).decode("ascii")})
            receipt["art"] = api("/v1/arts/install", {
                "zipBase64": base64.b64encode(art).decode("ascii"), "bundledCatalog": True})
            image = root / "input.png"
            write_input(image)
            data = "data:image/png;base64," + base64.b64encode(image.read_bytes()).decode("ascii")
            params = {"output_mode": "shader", "shader_abi": "otter.hlsl.v1", "strength": 0}
            run = api("/v1/tools/" + ART + "/execute", {"arguments": {
                "inputs": {"input": data, "reference": data}, "params": params}})
            (root / "result.json").write_text(json.dumps(run), encoding="utf-8")
            assert run["status"] == "succeeded", run.get("error")
            output = run["result"]
            assert output["type"] == "shader" and output["abi"] == "otter.hlsl.v1"
            assert output["source"].strip() == contents["runtime/color_transfer.hlsl"].decode().strip()
            assert output["textures"]["lut"].startswith("data:image/png;base64,")
            receipt["httpHlslLutVerified"] = True
            bridge = api("/v1/hook-bridge/start", {"port": 0})["running"]
            assert bridge
            assert json.loads(manifest.read_text(encoding="utf-8"))["hookBridge"]
            fixture = root / "otter-input.json"
            fixture.write_text(json.dumps({"manifest": str(manifest), "image": str(image)}),
                               encoding="utf-8")
            test_env = dict(env, OTTER_COLOR_TRANSFER_INPUT=str(fixture),
                            OTTER_NATIVE_PROFILE_DIR=str(root / "otter-profile"))
            with (root / "otter.log").open("wb") as test_log:
                run = subprocess.run([args.otter_test_exe, "--exact", EXACT, "--ignored",
                                      "--nocapture"], cwd=root, env=test_env,
                                     stdout=test_log, stderr=test_log, timeout=40)
            receipt["otterExitCode"] = run.returncode
            db = sqlite3.connect((root / "runs.sqlite3").as_uri() + "?mode=ro", uri=True)
            try:
                runs = [json.loads(row[0]) for row in db.execute("SELECT run_json FROM runs")]
            finally:
                db.close()
            joint = [run for run in runs if run.get("surface") == "loom_hook_v1"]
            receipt["jointRuns"] = joint
            assert run.returncode == 0, f"Otter exited {run.returncode}; see {root / 'otter.log'}"
            assert len(joint) == 1, joint
            assert joint[0]["toolId"] == "neuro.official/" + ART, joint
            assert joint[0]["status"] == "succeeded", joint
    finally:
        try:
            if bridge and child.poll() is None:
                receipt["bridgeStopped"] = api("/v1/hook-bridge/stop", {})["running"] is False
        finally:
            if child is not None:
                if child.poll() is None:
                    child.terminate()
                child.wait(timeout=10)
            manifest.unlink(missing_ok=True)
            (root / "receipt.json").write_text(json.dumps(receipt, indent=2), encoding="utf-8")
    assert receipt.get("bridgeStopped") is True, "owned bridge did not confirm shutdown"


if __name__ == "__main__":
    main()
