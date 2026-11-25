# Otter and Loom joint verification

| Path to verify | Procedure |
| --- | --- |
| Single Loom, device/group catalog visibility, and actual presentation after automatic acceptance | [Two-machine acceptance](#two-machine-acceptance) |
| QR/link v2: account invitations, PNG sync between two Loom instances on one machine, reopening and unlinking | [Account PNG joint testing](#account-png-joint-testing) |
| Single-Loom LAN Live: WGC, H.264, reconnection, revocation and input release | [Live joint testing](../features/LIVE_JOINT_TESTING.md) |
| Internal archive and startup after extraction | [Internal ZIP and runtime startup](WINDOWS_NATIVE_BUILD.md#internal-zip-and-runtime-startup) |

## Two-machine acceptance

Use the same current Otter version on two Windows computers, with current Loom running on at least
one. Leave real workspaces unchanged and use disposable test pins. Do not expose user screenshots,
invitations, tokens or private keys.

1. [Enable the Loom LAN connection](../features/LOOM_CONNECTION.md#lan-discovery-connection-codes-and-approval).
   Confirm that the other computer can reach the selected address and both ends are paired and approved.
   The test device should be selectable on the sender; choose the Accept receiving policy.
2. On the sender, open pin settings → Projection → Devices, select the receiver and confirm.
   If the state is Waiting, confirm that it eventually becomes Received. Inspect the actual receiver pin
   and edit the source to verify updates. Use a new pin to test Automatic, then separately check how
   Reject and Pause affect new deliveries versus existing projections.
3. Import another source through a QR code or link, then check updates, restart recovery, unlinking
   and retention of the final image. Do not reuse an already consumed invitation to bind a second device.
4. Confirm that disconnecting makes the device offline, that a manual disconnect survives restart,
   and that reconnecting reuses the original device rather than creating duplicates.

Record ordinary-menu, multi-monitor, mixed-DPI, physical-input, cross-machine and real public-network
results separately. Include versions and artifact digests for both ends, sender/receiver roles,
the first missing state and sanitized logs. Reporting only Connected or sender-side counts is insufficient.

## Account PNG joint testing

Use `scripts/tests/account-projection/run.py` to build a test program from current Otter source
and create a separate run directory.

Prerequisites: Windows, Python 3.11+, Node, the Rust MSVC toolchain, a Windows resource compiler,
running Docker with Linux containers, and current Otter, Loom and Platform source.
First run `npm ci` in Platform and `cargo build --locked -p loom-daemon` in Loom.
Their business implementations are reused from their own repositories; this entry point
neither copies server implementations nor introduces dependencies.

Run from the Otter root, adjusting the example paths and IP for your machine:

```powershell
python -B scripts/tests/account-projection/run.py `
  --loom-repo C:/work/Loom `
  --platform-repo C:/work/Platform `
  --daemon C:/work/Loom/target/debug/loom-daemon.exe `
  --lan-host 192.168.1.20 `
  --evidence-root C:/work/test-output `
  --rc 'C:/Program Files/LLVM/bin/llvm-rc.exe'
```

`--lan-host` must be a LAN IPv4 address that this machine can bind. `--cargo-target-dir` optionally
selects a reusable Cargo cache; otherwise Cargo's default configuration applies. Omit `--rc` if
the resource compiler is already configured. Every run asks Cargo to check current Otter source,
resolves the actual test artifact and freezes that run's test EXE; historical Otter test EXEs are not accepted.
The caller must build and supply `--daemon` in advance. The runner records its digest but does not build
Loom or prove that the EXE matches current source in `--loom-repo`; retain separate Loom build-provenance evidence.
The first build may be slow; the output directory is printed at the start. Exit code 0 means both
the joint workflow and cleanup succeeded. `evidence.json` is written only after business checks and
cleanup of the local HTTP service, Redis client and both Loom instances succeed. The outer Redis
container is still cleaned up by the Python runner; check `redis-cleanup.json` and the overall exit code separately.
`artifacts.json` records both EXE digests, and logs retain actual failures.

The runner owns a randomly named, non-persistent, loopback-only Redis container and two isolated
Loom instances. Account approval uses two distinct synthetic users. The eight-step Otter Application
flow runs on a private desktop that is never switched to the foreground. Each actor keeps the same
profile for device identity, TLS trust and workspace across steps and restarts.
The outer runner reuses the repository's Win32 Job owner to reclaim this run's child-process tree
on failure or timeout. At completion it removes only this run's Redis container and temporary Loom
profiles, retaining Otter profiles, raw results and logs for diagnosis.
Tests do not read real accounts or workspaces. Output contains synthetic device details and should not be committed.

This entry point verifies the v2 PNG lifecycle between two Loom instances on one physical machine.
It does not replace two-machine, Live, ordinary desktop-menu, or real OAuth/public-network acceptance.
Check historical acceptance receipts against Git history and their original artifacts; they do not prove
that current code passes.

## Desktop regressions

- Ordinary capture produces an image on release without affecting existing pins, dragging, saving or reopening.
- Open the edit strip, settings strip and parameter panel in turn. During dragging, auxiliary controls
  follow the window without losing focus or uncommitted settings.
- Verify OCR, translation and Art with currently authorized Loom. Revoked catalog entries must not leave
  old actions usable. Record real desktop, mixed-DPI/multi-monitor, IME and long-running stability results;
  do not mark untested scenarios as passed.
