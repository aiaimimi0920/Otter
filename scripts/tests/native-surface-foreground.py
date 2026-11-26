"""Authorize Surface real-foreground tests individually from a user-activated owned window, without proactively stealing focus."""

import argparse
import ctypes as C
from ctypes import wintypes as W
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import threading


REPO_ROOT = Path(__file__).resolve().parents[2]
TEMP_ROOT = REPO_ROOT / ".tmp"
READY = b"OTTER_FOREGROUND_READY"
TEST_NAME = re.compile(r"[A-Za-z_][A-Za-z_0-9]*(?:::[A-Za-z_][A-Za-z_0-9]*)+")

def scratch_root():
    root = TEMP_ROOT.resolve()
    if root == REPO_ROOT or not root.is_relative_to(REPO_ROOT) or root.drive.upper() != "C:":
        raise ValueError("Repository .tmp must resolve inside the local C: repository")
    root.mkdir(parents=True, exist_ok=True)
    return root


def job_runner():
    path = Path(__file__).resolve().parents[1] / "native-angle/build_job.py"
    spec = importlib.util.spec_from_file_location("build_job", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.run_job


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def inventory(path):
    manifest = json.loads(path.read_text(encoding="utf-8-sig"))
    cases = [case["test"] for case in manifest["cases"]]
    expected = manifest["driverSha256"].lower()
    if not re.fullmatch(r"[0-9a-f]{64}", expected):
        raise ValueError("Invalid driver SHA-256")
    if (not 0 < len(cases) == manifest["count"] <= 128
            or len(set(cases)) != len(cases)
            or any(len(name) > 300 or not TEST_NAME.fullmatch(name) for name in cases)):
        raise ValueError("Invalid exact case inventory")
    return expected, cases


class Foreground:
    def __init__(self, hwnd):
        user = C.WinDLL("user32", use_last_error=True)
        user.GetAncestor.argtypes = [W.HWND, W.UINT]
        user.GetAncestor.restype = W.HWND
        user.GetForegroundWindow.argtypes = []
        user.GetForegroundWindow.restype = W.HWND
        user.AllowSetForegroundWindow.argtypes = [W.DWORD]
        user.AllowSetForegroundWindow.restype = W.BOOL
        self.window = user.GetAncestor(hwnd, 2)
        if not self.window:
            raise C.WinError(C.get_last_error())
        self.user = user

    def owned(self):
        return self.user.GetForegroundWindow() == self.window

    def grant(self, pid):
        if not self.owned():
            return False
        if not self.user.AllowSetForegroundWindow(pid):
            raise C.WinError(C.get_last_error())
        return True


class Run:
    def __init__(self, driver, expected, root):
        self.driver, self.expected, self.root = driver, expected, root
        self.run_job = job_runner()
        self.env = {key: value for key, value in os.environ.items()
                    if not key.upper().startswith(("OTTER_", "LOOM_"))}

    def preflight(self, cases):
        if digest(self.driver) != self.expected:
            raise ValueError("Driver SHA-256 mismatch")
        stdout, stderr = self.root / "list.txt", self.root / "list-stderr.txt"
        with stdout.open("xb") as out, stderr.open("xb") as err:
            result = self.run_job([str(self.driver), "--list"], self.root, self.env,
                                  out, err, timeout=15)
        listed = set(stdout.read_text(encoding="utf-8").splitlines())
        missing = [name for name in cases if name + ": test" not in listed]
        if result["status"] != "exited" or result["exitCode"] != 0 or missing:
            raise ValueError(f"Inventory does not match the exact driver: {missing}")

    def case(self, index, name, grant, stop):
        if digest(self.driver) != self.expected:
            raise ValueError("Driver SHA-256 mismatch")
        work = self.root / f"{index + 1:03}"
        work.mkdir()
        temp, profile = work / "temp", work / "profile"
        temp.mkdir()
        profile.mkdir()
        env = dict(self.env, OTTER_NATIVE_FOREGROUND_SPONSOR="1",
                   TEMP=str(temp), TMP=str(temp), OTTER_NATIVE_PROFILE_DIR=str(profile))
        stdout, stderr = work / "stdout.txt", work / "stderr.txt"
        granted = False
        rfd, wfd = os.pipe()
        with os.fdopen(rfd, "rb") as permission, os.fdopen(wfd, "wb", buffering=0) as signal:
            def poll(pid):
                nonlocal granted
                if stop.is_set():
                    return False
                if not granted:
                    with stdout.open("rb") as stream:
                        ready = READY in stream.read(65536).splitlines()
                    if ready and grant(pid):
                        signal.write(b"G")
                        granted = True
                return True

            with stdout.open("xb") as out, stderr.open("xb") as err:
                result = self.run_job([str(self.driver), "--exact", name, "--nocapture"],
                                      work, env, out, err, timeout=120,
                                      stdin=permission, on_poll=poll)
        output = stdout.read_text(encoding="utf-8", errors="replace")
        passed = (granted and result["status"] == "exited" and result["exitCode"] == 0
                  and "test result: ok. 1 passed; 0 failed;" in output)
        receipt = dict(test=name, driverSha256=self.expected, foregroundGrant=granted,
                       passed=passed, stdoutSha256=digest(stdout), stderrSha256=digest(stderr), **result)
        (work / "evidence.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        return receipt


def ui(run, cases):
    import tkinter as tk

    root = tk.Tk()
    root.title("Otter Surface 前台验证")
    text = tk.StringVar(value="点击「运行下一项」授权；每项只启动一个隔离测试。")
    tk.Label(root, textvariable=text, padx=20, pady=14, wraplength=510).pack()
    stop = threading.Event()
    thread, index, result = None, 0, None

    def start():
        nonlocal thread, result
        if not foreground.owned():
            text.set("先激活此窗口，再点击运行。")
            return
        button.config(state="disabled")
        text.set(f"{index + 1}/{len(cases)} 运行中。若前台丢失，请重新激活此窗口。")
        result = None

        def worker():
            nonlocal result
            try:
                result = run.case(index, cases[index], foreground.grant, stop)
            except Exception as error:
                result = error

        thread = threading.Thread(target=worker)
        thread.start()
        root.after(100, finish)

    def finish():
        nonlocal index
        if thread.is_alive():
            root.after(100, finish)
            return
        thread.join()
        if stop.is_set():
            root.destroy()
            return
        if isinstance(result, Exception):
            text.set(f"启动器错误：{result}；请关闭窗口并查看日志。")
            return
        index += 1
        outcome = "通过" if result["passed"] else "未通过"
        text.set(f"{index}/{len(cases)} {outcome}。证据：{run.root}")
        if index < len(cases):
            button.config(state="normal")

    def close():
        stop.set()
        if thread is None or not thread.is_alive():
            root.destroy()
        else:
            text.set("正在回收本次启动的测试进程。")

    button = tk.Button(root, text="运行下一项", command=start)
    button.pack(padx=20, pady=(0, 14))
    root.protocol("WM_DELETE_WINDOW", close)
    root.update_idletasks()
    foreground = Foreground(root.winfo_id())
    root.mainloop()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("driver", "inventory", "evidence-dir"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--preflight", action="store_true", help="只核对摘要和测试清单，不显示窗口")
    parser.add_argument("--case", help="只运行清单中的一个完整测试名")
    args = parser.parse_args()
    try:
        scratch = scratch_root()
    except ValueError as error:
        parser.error(str(error))
    root = args.evidence_dir.resolve()
    driver = args.driver.resolve(strict=True)
    if (not root.is_relative_to(scratch) or root == scratch
            or driver.drive.upper() != "C:" or driver.suffix.lower() != ".exe"):
        parser.error("Use a local C: driver and a fresh evidence directory under .tmp")
    expected, cases = inventory(args.inventory)
    if args.case:
        if args.case not in cases:
            parser.error("Exact case is not in the inventory")
        cases = [args.case]
    root.mkdir()
    run = Run(driver, expected, root)
    run.preflight(cases)
    (root / "inventory.json").write_text(json.dumps(
        dict(driverSha256=expected, cases=cases), indent=2) + "\n", encoding="utf-8")
    print(f"Exact inventory verified: {len(cases)}; SHA-256: {expected}", flush=True)
    if not args.preflight:
        ui(run, cases)


if __name__ == "__main__":
    main()
