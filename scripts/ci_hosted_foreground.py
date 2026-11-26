"""Run the native test gate with an interactive foreground on a disposable GitHub VM."""
import ctypes
from ctypes import wintypes
import os
import subprocess
import sys


class ForegroundTimeout:
    def __init__(self):
        self.spi = ctypes.WinDLL("user32", use_last_error=True).SystemParametersInfoW
        self.spi.argtypes = (wintypes.UINT, wintypes.UINT, ctypes.c_void_p, wintypes.UINT)
        self.spi.restype = wintypes.BOOL

    def read(self):
        value = wintypes.DWORD()
        if not self.spi(0x2000, 0, ctypes.byref(value), 0):  # SPI_GETFOREGROUNDLOCKTIMEOUT
            raise ctypes.WinError(ctypes.get_last_error())
        return value.value

    def set(self, value):
        # Setter takes the value, not a pointer to a DWORD. No profile writes or broadcasts.
        if not self.spi(0x2001, 0, value, 0):  # SPI_SETFOREGROUNDLOCKTIMEOUT
            raise ctypes.WinError(ctypes.get_last_error())


def run(arguments, env, policy_factory=ForegroundTimeout, invoke=subprocess.call):
    if not (env.get("GITHUB_ACTIONS") == "true"
            and env.get("OTTER_CI_RUNNER_ENVIRONMENT") == "github-hosted"
            and env.get("RUNNER_OS") == "Windows"):
        raise ValueError("Foreground setup is restricted to disposable GitHub-hosted Windows runners")
    if arguments[:3] != ["cargo", "nextest", "run"]:
        raise ValueError("Expected the native Nextest gate")
    policy = policy_factory()
    saved = policy.read()
    if saved == 0:
        return invoke(arguments)
    policy.set(0)
    try:
        if policy.read() != 0:
            raise RuntimeError("Runner foreground timeout was not applied")
        print(f"Hosted foreground timeout: {saved} -> 0 ms for Nextest", flush=True)
        return invoke(arguments)
    finally:
        policy.set(saved)
        if policy.read() != saved:
            raise RuntimeError("Runner foreground timeout was not restored")
        print(f"Restored hosted foreground timeout: {saved} ms", flush=True)


if __name__ == "__main__":
    sys.exit(run(sys.argv[1:], os.environ))
