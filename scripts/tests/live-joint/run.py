"""Verify single-Loom LAN Live from current Otter/Loom sources; not dual-Loom or ordinary-product acceptance."""

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

REPO = Path(__file__).resolve().parents[3]
CASES = {
    "wgc-h264": ("live_h264_native_wgc_source_resize_and_pixels", "OTTER_LIVE_H264_EVIDENCE_DIR"),
    "h264": ("live_h264_native_source_viewer_current_keyframes", "OTTER_LIVE_H264_EVIDENCE_DIR"),
    "viewer-reconnect": ("live_h264_native_source_viewer_media_disconnect_recovery", "OTTER_LIVE_H264_EVIDENCE_DIR"),
    "control-reconnect": ("live_h264_native_source_viewer_control_disconnect_recovery", "OTTER_LIVE_H264_EVIDENCE_DIR"),
    "source-reconnect": ("live_h264_native_source_viewer_source_disconnect_recovery", "OTTER_LIVE_H264_EVIDENCE_DIR"),
    "input-release": ("live_viewer_source_native_menu_input_and_release", "OTTER_LIVE_VIEWER_SOURCE_EVIDENCE_DIR"),
    "direct-revocation": ("live_direct_native_joint_updates_and_revocation", "OTTER_LIVE_LAN_EVIDENCE_DIR"),
}
WGC = "actual_wgc_repeated_resize_input_restart_and_source_restore"


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--loom-repo", required=True, type=Path)
    parser.add_argument("--lan-host", required=True, help="可绑定的本机局域网 IPv4 地址")
    parser.add_argument("--evidence-root", required=True, type=Path)
    parser.add_argument("--cargo-target-dir", type=Path)
    parser.add_argument("--rc", type=Path)
    parser.add_argument("--case", choices=[*CASES, "wgc-cycles"], action="append")
    parser.add_argument("--with-wgc", action="store_true", help="另测自有窗口真实捕获；会短暂显示测试窗口")
    args = parser.parse_args()
    if os.name != "nt":
        raise ValueError("Windows is required for native Live tests")
    loom = args.loom_repo.resolve(strict=True)
    host = ipaddress.IPv4Address(args.lan_host)
    if not any(host in ipaddress.IPv4Network(block) for block in ("10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16")):
        raise ValueError("--lan-host must be a local LAN IPv4 address")
    with socket.socket() as probe:
        probe.bind((str(host), 0))
    cargo = shutil.which("cargo")
    if not cargo:
        raise ValueError("cargo must be available on PATH")
    args.evidence_root.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="otter-live-joint-", dir=args.evidence_root)).resolve()
    print(f"Evidence: {root}", flush=True)
    spec = importlib.util.spec_from_file_location("build_job", REPO / "scripts/native-angle/build_job.py")
    job = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(job)
    environment = {k: v for k, v in os.environ.items() if not k.upper().startswith(("LOOM_", "OTTER_"))}
    environment.update(TEMP=str(root), TMP=str(root), OTTER_LIVE_TEST_LAN_HOST=str(host))
    if args.cargo_target_dir:
        environment["CARGO_TARGET_DIR"] = str(args.cargo_target_dir.resolve())
    if args.rc:
        environment["OTTER_NATIVE_RC"] = str(args.rc.resolve(strict=True))

    def run(label, command, cwd, env, timeout):
        with (root / f"{label}.stdout.log").open("xb") as out, (root / f"{label}.stderr.log").open("xb") as err:
            receipt = job.run_job(command, cwd, env, out, err, timeout=timeout)
        (root / f"{label}.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        print(f"{label}: {receipt['status']} exit={receipt['exitCode']}", flush=True)
        return receipt

    def build(label, cwd, options, crate):
        receipt = run(label, [cargo, "test", "--locked", *options, "--lib", "--no-run", "--message-format=json"],
                      cwd, environment, 1800)
        if receipt["exitCode"] or receipt["status"] != "exited":
            raise RuntimeError(f"{label} failed; see {root}")
        artifacts = []
        for line in (root / f"{label}.stdout.log").read_text(encoding="utf-8").splitlines():
            item = json.loads(line)
            if (item.get("reason") == "compiler-artifact" and item.get("profile", {}).get("test")
                    and item.get("target", {}).get("name") == crate and item.get("executable")):
                artifacts.append(Path(item["executable"]))
        if len(artifacts) != 1:
            raise RuntimeError(f"Expected one {crate} test artifact, got {len(artifacts)}")
        frozen = root / f"{crate}-tests.exe"
        shutil.copy2(artifacts[0], frozen)
        return frozen

    # Both repositories may share Cargo caches, but builds are serial; test paths come only from this run's Cargo artifacts.
    otter = build("build-otter", REPO, ["--manifest-path", str(REPO / "native/Cargo.toml")], "otter_native")
    daemon = build("build-loom", loom, ["-p", "loom-daemon"], "loom_daemon")
    selected = list(dict.fromkeys(args.case or CASES))
    if args.with_wgc and "wgc-cycles" not in selected:
        selected.append("wgc-cycles")
    hook = None
    if any(case in selected for case in ("wgc-cycles", "wgc-h264")):
        receipt = run("build-hook", [cargo, "build", "--locked", "--manifest-path", str(REPO / "native/Cargo.toml"),
                      "-p", "otter-live-hover-hook", "--lib", "--message-format=json"], REPO, environment, 600)
        if receipt["exitCode"] or receipt["status"] != "exited":
            raise RuntimeError(f"build-hook failed; see {root}")
        libraries = []
        for line in (root / "build-hook.stdout.log").read_text(encoding="utf-8").splitlines():
            item = json.loads(line)
            if item.get("reason") == "compiler-artifact" and item.get("target", {}).get("name") == "otter_live_hover_hook":
                libraries.extend(Path(p) for p in item["filenames"] if p.endswith(".dll"))
        if len(libraries) != 1:
            raise RuntimeError(f"Expected one hover hook DLL, got {libraries}")
        hook = root / libraries[0].name
        shutil.copy2(libraries[0], hook)
    environment["OTTER_NATIVE_TEST_EXE"] = str(otter)
    artifacts = {"otterTestSha256": digest(otter), "loomTestSha256": digest(daemon),
                 "hoverHookSha256": digest(hook) if hook else None,
                 "otterRepo": str(REPO), "loomRepo": str(loom), "lanHost": str(host)}
    (root / "artifacts.json").write_text(json.dumps(artifacts, indent=2) + "\n", encoding="utf-8")
    results = []
    for label in selected:
        env = environment.copy()
        if label == "wgc-cycles":
            executable, suffix = otter, WGC
        else:
            suffix, output_variable = CASES[label]
            executable = daemon
            env[output_variable] = str(root / f"evidence-{label}")
        profile = root / f"profile-{label}"
        profile.mkdir()
        env["OTTER_NATIVE_PROFILE_DIR"] = str(profile)
        listing = subprocess.check_output([str(executable), "--list"], cwd=root, env=env, timeout=30, text=True)
        tests = [line.removesuffix(": test") for line in listing.splitlines()
                 if line.endswith("::" + suffix + ": test")]
        if len(tests) != 1:
            raise RuntimeError(f"Expected one test for {suffix}, got {tests}")
        receipt = run(label, [str(executable), tests[0], "--exact", "--ignored", "--test-threads=1", "--nocapture"],
                      root, env, 240)
        log = (root / f"{label}.stdout.log").read_text(encoding="utf-8", errors="replace")
        passed = receipt["exitCode"] == 0 and receipt["status"] == "exited" and "1 passed; 0 failed" in log
        results.append({"case": label, "test": tests[0], "passed": passed, **receipt})
    report = {"passed": all(r["passed"] for r in results), "results": results, "lanHost": str(host),
              "loomInstancesPerJoint": 1,
              "syntheticGpuMediaSource": any(case not in ("wgc-h264", "wgc-cycles", "direct-revocation") for case in selected),
              "directRawWireSource": "direct-revocation" in selected,
              "wgcTestedSeparately": "wgc-cycles" in selected,
              "wgcToNetworkViewer": any(r["case"] == "wgc-h264" and r["passed"] for r in results),
              "crossLoomLive": False, "physicalCrossMachine": False, "ordinaryProductGui": False,
              "publicNetwork": "deferred"}
    (root / "evidence.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
