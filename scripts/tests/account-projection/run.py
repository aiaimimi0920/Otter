"""从当前源码构建并执行账号 v2 联合测试；仅拥有本次进程树和临时 Redis。"""

import argparse
import hashlib
import importlib.util
import ipaddress
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import uuid

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--loom-repo", required=True, type=Path)
    parser.add_argument("--platform-repo", required=True, type=Path)
    parser.add_argument("--daemon", required=True, type=Path)
    parser.add_argument("--lan-host", required=True, help="本机局域网 IPv4 地址")
    parser.add_argument("--evidence-root", type=Path, default=Path(tempfile.gettempdir()))
    parser.add_argument("--cargo-target-dir", type=Path)
    parser.add_argument("--rc", type=Path, help="可选 Windows resource compiler")
    return parser.parse_args()


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def main():
    args = arguments()
    if os.name != "nt":
        raise ValueError("Windows is required for DPAPI and native desktop fixtures")
    for name in ("loom_repo", "platform_repo", "daemon"):
        setattr(args, name, getattr(args, name).resolve(strict=True))
    loader = (args.platform_repo / "node_modules/tsx/dist/loader.mjs").resolve(strict=True)
    (args.loom_repo / "scripts/qr-projection-smoke/runtime.ts").resolve(strict=True)
    host = ipaddress.IPv4Address(args.lan_host)
    if not host.is_private or host.is_loopback or host.is_unspecified or host.is_multicast:
        raise ValueError("--lan-host must be a local LAN IPv4 address")
    with socket.socket() as probe:
        probe.bind((str(host), 0))
    commands = {name: shutil.which(name) for name in ("cargo", "node", "docker")}
    if not all(commands.values()):
        raise ValueError("cargo, node and docker must be available on PATH")
    args.evidence_root.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="otter-account-joint-", dir=args.evidence_root)).resolve()
    print(f"Evidence: {root}", flush=True)
    spec = importlib.util.spec_from_file_location("build_job", REPO / "scripts/native-angle/build_job.py")
    job = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(job)
    environment = os.environ.copy()
    environment.update(TEMP=str(root), TMP=str(root))
    if args.cargo_target_dir:
        environment["CARGO_TARGET_DIR"] = str(args.cargo_target_dir.resolve())
    if args.rc:
        environment["OTTER_NATIVE_RC"] = str(args.rc.resolve(strict=True))

    def run(label, command, env, timeout):
        with (root / f"{label}.stdout.log").open("xb") as out, (root / f"{label}.stderr.log").open("xb") as err:
            receipt = job.run_job(command, REPO, env, out, err, timeout=timeout)
        (root / f"{label}.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        print(f"{label}: {receipt['status']} exit={receipt['exitCode']}", flush=True)
        if receipt["exitCode"] or receipt["status"] != "exited":
            raise RuntimeError(f"{label} failed; see {root}")

    # Cargo 的 artifact 消息是路径真源；每次检查当前源码，不接受旧测试 EXE 参数。
    run("build", [commands["cargo"], "test", "--manifest-path", str(REPO / "native/Cargo.toml"),
                  "--locked", "--lib", "--no-run", "--message-format=json"], environment, 1800)
    artifacts = []
    for line in (root / "build.stdout.log").read_text(encoding="utf-8").splitlines():
        item = json.loads(line)
        if (item.get("reason") == "compiler-artifact" and item.get("profile", {}).get("test")
                and "lib" in item.get("target", {}).get("kind", []) and item.get("executable")):
            artifacts.append(Path(item["executable"]))
    if len(artifacts) != 1:
        raise RuntimeError(f"Expected one native library test artifact, got {len(artifacts)}")
    executable = root / "otter-application-tests.exe"
    shutil.copy2(artifacts[0], executable)
    environment = {key: value for key, value in environment.items()
                   if not key.upper().startswith(("LOOM_", "OTTER_"))}
    environment.update(
        OTTER_APPLICATION_TEST_EXE=str(executable), OTTER_APPLICATION_EVIDENCE_ROOT=str(root),
        OTTER_TEST_LOOM_REPO=str(args.loom_repo), OTTER_TEST_PLATFORM_REPO=str(args.platform_repo),
        OTTER_TEST_PYTHON=sys.executable, OTTER_TEST_LAN_HOST=str(host),
        LOOM_ACCOUNT_TEST_DAEMON=str(args.daemon),
    )
    name = "otter-account-test-" + uuid.uuid4().hex

    def docker(*items):
        return subprocess.check_output([commands["docker"], *items], cwd=REPO, timeout=30, text=True).strip()

    owned = False
    try:
        docker("run", "-d", "--rm", "--name", name, "-p", "127.0.0.1::6379", "redis:8-alpine",
               "redis-server", "--save", "", "--appendonly", "no")
        owned = True
        ports = json.loads(docker("inspect", "--format", "{{json .NetworkSettings.Ports}}", name))
        port = int(ports["6379/tcp"][0]["HostPort"])
        if not 1 <= port <= 65535:
            raise RuntimeError("Isolated Redis has no valid port")
        # docker run returns before Redis accepts connections; wait only for this owned service.
        deadline = time.monotonic() + 15
        while True:
            probe = subprocess.run([commands["docker"], "exec", name, "redis-cli", "ping"],
                                   cwd=REPO, timeout=5, capture_output=True, text=True)
            if probe.returncode == 0 and probe.stdout.strip() == "PONG":
                break
            if "Connection refused" not in probe.stderr or time.monotonic() >= deadline:
                raise RuntimeError(f"Isolated Redis is not ready: {probe.stdout}{probe.stderr}")
            time.sleep(0.1)
        environment["LOOM_ACCOUNT_TEST_REDIS_URL"] = f"redis://127.0.0.1:{port}"
        run("joint", [commands["node"], "--import", loader.as_uri(), str(HERE / "account-fixture.mjs")], environment, 360)
    finally:
        if owned:
            docker("rm", "-f", name)
            (root / "redis-cleanup.json").write_text(json.dumps({"name": name, "removed": True}), encoding="utf-8")
    (root / "artifacts.json").write_text(json.dumps({
        "testSha256": digest(executable), "daemonSha256": digest(args.daemon),
        "otterRepo": str(REPO), "loomRepo": str(args.loom_repo), "platformRepo": str(args.platform_repo),
        "lanHost": str(host), "physicalCrossMachine": False, "publicNetwork": False,
    }, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
