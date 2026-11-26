"""Run an isolated real Loom bridge stop/start around native workspace archive and send."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile
import time
from urllib.error import URLError
from urllib.request import HTTPRedirectHandler, ProxyHandler, Request, build_opener

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRATCH = REPO_ROOT / ".tmp"
EXACT = ("desktop::application::restore_batch::workflow::cycle::real_loom_tests::"
         "saved_workflow_survives_real_loom_bridge_restart")


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def wait(process, probe, seconds=15):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        if process.poll() is not None:
            raise RuntimeError(f"Fixture process exited: {process.returncode}")
        result = probe()
        if result:
            return result
        time.sleep(0.05)
    raise TimeoutError(f"Fixture deadline: {probe}")


def terminate(process):
    if process.poll() is None:
        process.terminate()
        process.wait(timeout=10)


def port():
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("daemon", type=Path)
    parser.add_argument("otter_test_exe", type=Path)
    args = parser.parse_args()
    daemon = args.daemon.resolve(strict=True)
    native = args.otter_test_exe.resolve(strict=True)
    if any(path.drive.upper() != "C:" or not path.is_file() for path in (daemon, native)):
        parser.error("Both binaries must be files on local C:")
    listed = subprocess.run([native, "--list", "--ignored"], check=True, text=True,
                            stdout=subprocess.PIPE, timeout=10).stdout.splitlines()
    tests = [line[:-6] for line in listed if line.endswith(
        "::saved_workflow_survives_real_loom_bridge_restart: test")]
    if tests != [EXACT]:
        raise RuntimeError(f"Expected exactly {EXACT}, got {tests}")

    scratch = SCRATCH.resolve()
    if scratch == REPO_ROOT or not scratch.is_relative_to(REPO_ROOT) or scratch.drive.upper() != "C:":
        parser.error("Repository .tmp must resolve inside the local C: repository")
    scratch.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="otter-workflow-loom-restart-", dir=scratch))
    print(root, flush=True)
    for name in ("manifest", "appdata", "tmp"):
        (root / name).mkdir()
    management = port()
    token = secrets.token_hex(32)
    env = {key: value for key, value in os.environ.items()
           if not key.upper().startswith(("LOOM_", "OTTER_", "HOOK_"))}
    env.update(APPDATA=str(root / "appdata"), TEMP=str(root / "tmp"), TMP=str(root / "tmp"),
               LOOM_DAEMON_HOST="127.0.0.1", LOOM_DAEMON_PORT=str(management),
               LOOM_DAEMON_TOKEN=token, LOOM_CONTROL_PLANE_ROOT=str(root / "loom"),
               LOOM_RUN_STORE_PATH=str(root / "runs.sqlite3"), LOOM_LOG_DIR=str(root / "logs"),
               LOOM_CAPABILITY_MANIFEST_DIR=str(root / "manifest"),
               LOOM_OTTER_SESSION_PATH=str(root / "session.json"))
    opener = build_opener(ProxyHandler({}), NoRedirect())

    def api(route, data=None):
        body = None if data is None else json.dumps(data).encode("utf-8")
        req = Request(f"http://127.0.0.1:{management}/v1/hook-bridge/{route}", body,
                      {"Authorization": f"Bearer {token}", "Content-Type": "application/json"})
        with opener.open(req, timeout=5) as response:
            return json.load(response)

    def ready(server):
        manifest = root / "manifest/loom.json"
        if not manifest.is_file():
            return None
        data = json.loads(manifest.read_bytes())
        assert data["pid"] == server.pid
        return True

    proof = {"otterSha256": digest(native), "loomSha256": digest(daemon), "test": EXACT}
    # Loom's WorkflowStore maps hook-live to its compatibility filename.
    workflow = root / "loom/workflows/latest.yaml"
    with (root / "loom.log").open("xb") as server_log, (root / "otter.log").open("xb") as child_log:
        server = subprocess.Popen([daemon], env=env, cwd=root, stdout=server_log,
                                  stderr=subprocess.STDOUT, creationflags=subprocess.CREATE_NO_WINDOW)
        child = None
        try:
            wait(server, lambda: ready(server))
            first = api("start", {"port": 0})["port"]
            second = port()
            assert first != second
            child = subprocess.Popen([native, EXACT, "--exact", "--ignored", "--nocapture"], cwd=root,
                                     env=dict(env, OTTER_WORKFLOW_LOOM_FIXTURE=str(root)),
                                     stdout=child_log, stderr=subprocess.STDOUT,
                                     creationflags=subprocess.CREATE_NO_WINDOW)
            wait(child, lambda: (root / "initial-capability-read").is_dir())
            api("stop", {})
            with socket.socket() as probe:
                probe.settimeout(2)
                assert probe.connect_ex(("127.0.0.1", first)) != 0
            (root / "bridge-stopped").mkdir()
            wait(child, lambda: (root / "disconnected-outcome-verified").is_dir())
            assert not workflow.exists()
            assert api("start", {"port": second})["port"] == second
            (root / "bridge-started").mkdir()
            wait(child, lambda: (root / "fresh-capability-read").is_dir())
            assert not workflow.exists()
            proof["noWriteBeforeExplicitResend"] = True
            (root / "explicit-resend").mkdir()
            proof["otterExitCode"] = child.wait(timeout=25)
            assert child.returncode == 0
            native_result = json.loads((root / "otter-result.json").read_bytes())
            assert native_result["manualResendAcknowledged"] == 1
            assert native_result["sameCommittedBytes"] and native_result["archiveRetained"]
            saved = workflow.read_text(encoding="utf-8")
            assert native_result["nodeId"] in saved
            canvas = api("canvas")
            assert canvas["workflowId"] == "hook-live"
            assert native_result["nodeId"] in [node["id"] for node in canvas["nodes"]]
            proof.update(nativeResult=native_result, serverWorkflowSha256=digest(workflow),
                         serverCanvasNodeIds=[node["id"] for node in canvas["nodes"]], passed=True)
        finally:
            if child is not None:
                terminate(child)
            try:
                if server.poll() is None:
                    api("stop", {})
            except (URLError, TimeoutError):
                pass
            finally:
                terminate(server)
                proof["fixtureDaemonExited"] = server.poll() is not None
                (root / "joint-evidence.json").write_text(json.dumps(proof, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
