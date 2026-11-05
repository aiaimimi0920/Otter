"""Win32 构建进程树 owner：挂入 kill-on-close Job 后才恢复，显式继承三个标准句柄。"""

import ctypes as C
import hashlib
import msvcrt
import os
import subprocess
import time
from contextlib import ExitStack, contextmanager
from ctypes import wintypes as W

K = C.WinDLL("kernel32", use_last_error=True)


class Startup(C.Structure):
    _fields_ = [("cb", W.DWORD), ("reserved", W.LPWSTR), ("desktop", W.LPWSTR), ("title", W.LPWSTR),
        *[(name, W.DWORD) for name in ("x", "y", "width", "height", "charsX", "charsY", "fill", "flags")],
        ("show", W.WORD), ("reservedBytes", W.WORD), ("reservedData", C.c_void_p),
        ("stdin", W.HANDLE), ("stdout", W.HANDLE), ("stderr", W.HANDLE)]


class StartupEx(C.Structure):
    _fields_ = [("startup", Startup), ("attributes", C.c_void_p)]


class ProcessInfo(C.Structure):
    _fields_ = [("process", W.HANDLE), ("thread", W.HANDLE), ("pid", W.DWORD), ("tid", W.DWORD)]


class BasicLimits(C.Structure):
    _fields_ = [("processTime", C.c_int64), ("jobTime", C.c_int64), ("flags", W.DWORD),
        ("minimum", C.c_size_t), ("maximum", C.c_size_t), ("active", W.DWORD),
        ("affinity", C.c_size_t), ("priority", W.DWORD), ("scheduling", W.DWORD)]


class JobLimits(C.Structure):
    _fields_ = [("basic", BasicLimits), ("io", C.c_uint64 * 6), ("processMemory", C.c_size_t),
        ("jobMemory", C.c_size_t), ("peakProcess", C.c_size_t), ("peakJob", C.c_size_t)]


class Accounting(C.Structure):
    _fields_ = [("times", C.c_int64 * 4), ("faults", W.DWORD), ("total", W.DWORD),
        ("active", W.DWORD), ("terminated", W.DWORD)]


def api(name, arguments, result=W.BOOL):
    function = getattr(K, name)
    function.argtypes, function.restype = arguments, result
    return function


close = api("CloseHandle", [W.HANDLE])
current = api("GetCurrentProcess", [], W.HANDLE)
duplicate = api("DuplicateHandle", [W.HANDLE, W.HANDLE, W.HANDLE, C.POINTER(W.HANDLE), W.DWORD, W.BOOL, W.DWORD])
create_job = api("CreateJobObjectW", [C.c_void_p, W.LPCWSTR], W.HANDLE)
set_job = api("SetInformationJobObject", [W.HANDLE, C.c_int, C.c_void_p, W.DWORD])
query_job = api("QueryInformationJobObject", [W.HANDLE, C.c_int, C.c_void_p, W.DWORD, C.c_void_p])
assign = api("AssignProcessToJobObject", [W.HANDLE, W.HANDLE])
terminate = api("TerminateJobObject", [W.HANDLE, W.UINT])
terminate_process = api("TerminateProcess", [W.HANDLE, W.UINT])
wait = api("WaitForSingleObject", [W.HANDLE, W.DWORD], W.DWORD)
exit_code = api("GetExitCodeProcess", [W.HANDLE, C.POINTER(W.DWORD)])
resume = api("ResumeThread", [W.HANDLE], W.DWORD)
initialize = api("InitializeProcThreadAttributeList", [C.c_void_p, W.DWORD, W.DWORD, C.POINTER(C.c_size_t)])
update = api("UpdateProcThreadAttribute", [C.c_void_p, W.DWORD, C.c_size_t, C.c_void_p, C.c_size_t, C.c_void_p, C.c_void_p])
delete_attributes = api("DeleteProcThreadAttributeList", [C.c_void_p], None)
create_process = api("CreateProcessW", [W.LPCWSTR, W.LPWSTR, C.c_void_p, C.c_void_p, W.BOOL, W.DWORD,
    C.c_void_p, W.LPCWSTR, C.POINTER(StartupEx), C.POINTER(ProcessInfo)])
create_mutex = api("CreateMutexW", [C.c_void_p, W.BOOL, W.LPCWSTR], W.HANDLE)
release_mutex = api("ReleaseMutex", [W.HANDLE])


def require(value):
    if not value:
        raise C.WinError(C.get_last_error())
    return value


def drain_job(job):
    require(terminate(job, 1460))
    deadline = time.monotonic() + 30
    while True:
        account = Accounting()
        require(query_job(job, 1, C.byref(account), C.sizeof(account), None))
        if account.active == 0:
            return
        if time.monotonic() >= deadline:
            raise OSError("Build job still owns active descendants after cancellation")
        time.sleep(0.01)


def close_job(job):
    try:
        drain_job(job)
    finally:
        close(job)


def close_process(process):
    try:
        terminate_process(process, 1460)
        if wait(process, 30000) != 0:
            raise OSError("Build root process cleanup did not finish")
    finally:
        close(process)


@contextmanager
def stage_lock(stage):
    # Local 命名 mutex 不留锁文件，也不依赖清理或删除旧文件；只保护本入口的并发调用。
    name = "Local\\OtterAngleBuild_" + hashlib.sha256(str(stage.resolve()).casefold().encode()).hexdigest()
    handle = require(create_mutex(None, False, name))
    owned = False
    try:
        status = wait(handle, 0)
        if status not in (0, 0x80):
            raise ValueError("Build stage is already owned or mutex acquisition failed")
        owned = True
        yield
    finally:
        if owned:
            release_mutex(handle)
        close(handle)


def run_job(arguments, cwd, environment, stdout, stderr, *, timeout, max_log=16 * 1024 * 1024):
    if not arguments or not os.path.isabs(arguments[0]) or timeout <= 0:
        raise ValueError("Explicit executable and positive build timeout required")
    if any(not isinstance(value, str) or "\0" in value for value in arguments):
        raise ValueError("Invalid build arguments")
    if any(not key or "=" in key or "\0" in key + value for key, value in environment.items()):
        raise ValueError("Invalid build environment")
    handles, attributes, initialized = [], None, False
    job, process = None, ProcessInfo()
    start = time.monotonic()
    status = "exited"
    try:
        job = require(create_job(None, None))
        limits = JobLimits()
        limits.basic.flags, limits.basic.active = 0x2000 | 0x8, 32
        require(set_job(job, 9, C.byref(limits), C.sizeof(limits)))
        with open(os.devnull, "rb") as null:
            for stream in (null, stdout, stderr):
                handle = W.HANDLE()
                require(duplicate(current(), msvcrt.get_osfhandle(stream.fileno()), current(), C.byref(handle), 0, True, 2))
                handles.append(handle)
        size = C.c_size_t()
        initialize(None, 1, 0, C.byref(size))
        if not size.value:
            raise C.WinError(C.get_last_error())
        attributes = C.create_string_buffer(size.value)
        require(initialize(attributes, 1, 0, C.byref(size)))
        initialized = True
        inherited = (W.HANDLE * 3)(*(handle.value for handle in handles))
        require(update(attributes, 0, 0x20002, inherited, C.sizeof(inherited), None, None))
        startup = StartupEx()
        startup.startup.cb, startup.startup.flags = C.sizeof(startup), 0x100
        startup.startup.stdin, startup.startup.stdout, startup.startup.stderr = (handle.value for handle in handles)
        startup.attributes = C.cast(attributes, C.c_void_p)
        block = C.create_unicode_buffer("\0".join(key + "=" + value for key, value in sorted(environment.items())) + "\0\0")
        command = C.create_unicode_buffer(subprocess.list2cmdline(arguments))
        require(create_process(arguments[0], command, None, None, True, 0x4 | 0x400 | 0x80000 | 0x08000000,
                               block, str(cwd), C.byref(startup), C.byref(process)))
        # 进程仍 suspended，assign 失败时不可能已经逃逸出一个执行过的 descendant。
        require(assign(job, process.process))
        if resume(process.thread) == 0xFFFFFFFF:
            raise C.WinError(C.get_last_error())
        while True:
            state = wait(process.process, 100)
            if state == 0:
                break
            if state != 0x102:
                raise C.WinError(C.get_last_error())
            if time.monotonic() - start > timeout:
                status = "timeout"
                break
            if any(os.fstat(stream.fileno()).st_size > max_log for stream in (stdout, stderr)):
                status = "output-limit"
                break
        if any(os.fstat(stream.fileno()).st_size > max_log for stream in (stdout, stderr)):
            status = "output-limit"
        if status != "exited":
            require(terminate(job, 1460))
            if wait(process.process, 30000) != 0:
                raise OSError("Build process did not terminate after job cancellation")
        code = W.DWORD()
        require(exit_code(process.process, C.byref(code)))
        return {"pid": process.pid, "exitCode": code.value, "status": status, "seconds": time.monotonic() - start}
    finally:
        # 即使 root 提前退出，关闭 Job 也结束其尚存子进程；Job 是生命周期边界，不是权限沙箱。
        with ExitStack() as cleanup:
            for handle in handles:
                cleanup.callback(close, handle)
            if initialized:
                cleanup.callback(delete_attributes, attributes)
            if process.thread:
                cleanup.callback(close, process.thread)
            if process.process:
                cleanup.callback(close_process, process.process)
            if job:
                cleanup.callback(close_job, job)
