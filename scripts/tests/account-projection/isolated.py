"""在从未切换到前台的私有桌面运行单步 HWND 夹具；继承联合 runner 的进程 Job。"""

import ctypes as C
from ctypes import wintypes as W
import json
import msvcrt
import os
from pathlib import Path
import re
import subprocess
import sys
import time

RUN = Path(os.environ["OTTER_APPLICATION_EVIDENCE_ROOT"]).resolve(strict=True)
EXE = Path(os.environ["OTTER_APPLICATION_TEST_EXE"]).resolve(strict=True)
LABEL, *TESTS = sys.argv[1:]
if not re.fullmatch(r"[a-zA-Z0-9-]+", LABEL) or not TESTS:
    raise ValueError("A simple output label and test arguments are required")
runtime = RUN / ("test-runtime-" + LABEL)
runtime.mkdir()
actor_profile = os.environ.get("OTTER_PROJECTION_ACCOUNT_PROFILE")
profile = Path(actor_profile).resolve() if actor_profile else runtime / "profile"
# 每次重启沿用 actor 的设备身份和本次连接取得的 TLS 信任根。
profile.mkdir(parents=True, exist_ok=True)
os.environ.update(TEMP=str(runtime), TMP=str(runtime), OTTER_NATIVE_PROFILE_DIR=str(profile))
K = C.WinDLL("kernel32", use_last_error=True)
U = C.WinDLL("user32", use_last_error=True)


class Startup(C.Structure):
    _fields_ = [("cb", W.DWORD), ("lpReserved", W.LPWSTR), ("lpDesktop", W.LPWSTR),
                ("lpTitle", W.LPWSTR), ("dwX", W.DWORD), ("dwY", W.DWORD),
                ("dwXSize", W.DWORD), ("dwYSize", W.DWORD), ("dwXCountChars", W.DWORD),
                ("dwYCountChars", W.DWORD), ("dwFillAttribute", W.DWORD), ("dwFlags", W.DWORD),
                ("wShowWindow", W.WORD), ("cbReserved2", W.WORD), ("lpReserved2", C.c_void_p),
                ("hStdInput", W.HANDLE), ("hStdOutput", W.HANDLE), ("hStdError", W.HANDLE)]


class Process(C.Structure):
    _fields_ = [("hProcess", W.HANDLE), ("hThread", W.HANDLE),
                ("dwProcessId", W.DWORD), ("dwThreadId", W.DWORD)]


U.CreateDesktopW.argtypes = [W.LPCWSTR, W.LPCWSTR, C.c_void_p, W.DWORD, W.DWORD, C.c_void_p]
U.CreateDesktopW.restype = W.HANDLE
U.CloseDesktop.argtypes = [W.HANDLE]
U.CloseDesktop.restype = W.BOOL
K.CreateProcessW.argtypes = [W.LPCWSTR, W.LPWSTR, C.c_void_p, C.c_void_p, W.BOOL,
                           W.DWORD, C.c_void_p, W.LPCWSTR, C.POINTER(Startup), C.POINTER(Process)]
K.CreateProcessW.restype = W.BOOL
K.WaitForSingleObject.argtypes = [W.HANDLE, W.DWORD]
K.WaitForSingleObject.restype = W.DWORD
K.GetExitCodeProcess.argtypes = [W.HANDLE, C.POINTER(W.DWORD)]
K.GetExitCodeProcess.restype = W.BOOL
K.TerminateProcess.argtypes = [W.HANDLE, W.UINT]
K.TerminateProcess.restype = W.BOOL
K.CloseHandle.argtypes = [W.HANDLE]
K.CloseHandle.restype = W.BOOL


def checked(value):
    if not value:
        raise C.WinError(C.get_last_error())
    return value


name = "OtterAccountTest_" + str(os.getpid())
# 不请求 DESKTOP_SWITCHDESKTOP，也不调用 SwitchDesktop 或发送系统输入。
desktop = checked(U.CreateDesktopW(name, None, None, 0, 0xC3, None))
info = Process()
forced = False
start = time.monotonic()
try:
    with (RUN / (LABEL + ".log")).open("xb") as log, open(os.devnull, "rb") as null:
        handle = msvcrt.get_osfhandle(log.fileno())
        stdin = msvcrt.get_osfhandle(null.fileno())
        os.set_handle_inheritable(handle, True)
        os.set_handle_inheritable(stdin, True)
        startup = Startup()
        startup.cb = C.sizeof(startup)
        startup.lpDesktop = "winsta0\\" + name
        startup.dwFlags = 0x100
        startup.hStdInput = stdin
        startup.hStdOutput = startup.hStdError = handle
        args = [str(EXE), *TESTS, "--test-threads=1", "--nocapture"]
        checked(K.CreateProcessW(str(EXE), C.create_unicode_buffer(subprocess.list2cmdline(args)),
                                 None, None, True, 0x08000000, None, str(RUN), C.byref(startup), C.byref(info)))
        checked(K.CloseHandle(info.hThread))
        status = K.WaitForSingleObject(info.hProcess, 240000)
        if status != 0:
            forced = True
            checked(K.TerminateProcess(info.hProcess, 124))
            if K.WaitForSingleObject(info.hProcess, 5000) != 0:
                raise OSError("Owned native test did not terminate")
        code = W.DWORD()
        checked(K.GetExitCodeProcess(info.hProcess, C.byref(code)))
        receipt = {"exitCode": code.value, "pid": info.dwProcessId, "tests": TESTS,
                   "seconds": round(time.monotonic() - start, 3), "forcedTermination": forced,
                   "desktopSwitched": False, "systemInput": False}
finally:
    if info.hProcess:
        # 异常时也只终止自有句柄；联合 runner 的 Job 负责整个进程树。
        if K.WaitForSingleObject(info.hProcess, 0) == 0x102:
            checked(K.TerminateProcess(info.hProcess, 124))
            K.WaitForSingleObject(info.hProcess, 5000)
        checked(K.CloseHandle(info.hProcess))
    checked(U.CloseDesktop(desktop))
(RUN / (LABEL + ".json")).write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
print(json.dumps(receipt), flush=True)
if receipt["exitCode"] or forced:
    raise SystemExit(1)
