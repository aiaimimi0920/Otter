# Native Live joint testing

The maintained entry point is `scripts/tests/live-joint/run.py`. It builds test programs from current
Otter and Loom source, reuses Cargo caches serially, freezes test EXEs from the current Cargo artifacts,
and writes runtime evidence to a fresh directory. It does not consume historical test scripts or
fixed test EXEs from temporary directories.

Requirements: Windows, Python 3.11+, the Rust MSVC toolchain, a Windows resource compiler,
and current Otter and Loom source. NeuroDesign uses the Cargo-locked Git commit; no adjacent
checkout is needed. Run from the Otter root, adjusting paths and the LAN IP for your machine:

```powershell
python -B scripts/tests/live-joint/run.py `
  --loom-repo ../Loom `
  --lan-host 192.168.1.20 `
  --evidence-root .tmp/live-joint `
  --rc 'C:/Program Files/LLVM/bin/llvm-rc.exe' `
  --with-wgc
```

Place output in a dedicated disposable test directory such as `.tmp/live-joint`; identities, profiles, logs
and copied build artifacts there are reproducible. Reuse the repository's existing Cargo cache.
If `--cargo-target-dir` must be set explicitly, point only to an existing main Cargo target on
local `C:`, never the evidence directory or a fresh disposable cold cache. Tests use no real
accounts or workspaces. A Win32 Job bounds execution and reclaims this run's process tree after
failure or timeout. Test-library temporary cleanup covers only fixtures it created within this run's `TEMP/TMP`.

The default sequence is:

| Case | Checks |
| --- | --- |
| `wgc-h264` | Real WGC on an owned window → LAN H.264 → native viewer; checks pixels after color/size changes and restores the source window on stop |
| `h264` | Current H.264 encoding, HTTPS/WSS, system decoding, native GPU pixels and keyframe requests |
| `viewer-reconnect` | Retains the window after a viewer media disconnect, receives updated frames and reclaims the old connection |
| `control-reconnect` | Ordered recovery after a control GET disconnect, without automatically acquiring input control |
| `source-reconnect` | Reconnects after a sender media disconnect and verifies updated viewer frames |
| `input-release` | Current RawBgra Live frames, two keyboard/mouse control rounds, and release on menus/focus loss |
| `direct-revocation` | Directed receiver, two frame updates, revocation cleanup, and rejection of the revoked device after LAN listener stop/restart |

Repeat `--case` to run only selected cases. The default already includes `wgc-h264`;
`--with-wgc` only adds `wgc-cycles`, which verifies 12 real WGC sessions, 36 resizes,
24 input restarts and source-window restoration. It briefly shows owned test windows without
sending system input. Use `--case wgc-cycles` to rerun it alone. WGC cases also build
`otter_live_hover_hook.dll` from current source and place it beside the frozen test EXE;
copying only the EXE is insufficient. H.264 joint cases use no legacy client and do not require
old-format interoperability or version downgrades for acceptance.

Each joint case runs a real Loom service fixture; media actually travels through the machine's
RFC1918 LAN IPv4 specified by `--lan-host`. All cases except `direct-revocation` use two Otter
Application test processes. `wgc-h264` uses real WGC; the others use synthetic GPU textures.
`direct-revocation` uses one Otter test process to view RawBgra frames from a test client through
the production Relay Session; it does not test source Application menus. Distinguish end-to-end
capture transport, media-failure recovery, authorization and local lifecycles.

`evidence.json` contains per-case results and explicit boundaries. `artifacts.json` records
SHA-256 digests of both test programs and the WGC cases' `hoverHookSha256` (`null` if the hook
was not built). H.264 wire frame IDs correspond to viewer decode/GPU-upload frame IDs.
Source-window messages independently verify input tests. Final checks require cleanup of roles,
media connections and background owners. Test EXEs are not ordinary product deliverables.

These tests do not prove Live between two independent Loom instances, physical two-machine
operation, ordinary product GUI behavior or real OAuth. Account v2 between two Loom instances
currently has only the PNG sharing path; see [QR and link projection](QR_PROJECTION.md).
Public-network NAT or relay capability and acceptance require separate verification.

## Single-machine WinForms input gate

On an unlocked interactive Windows desktop, compile the dedicated in-repository fixture
from the Otter root and explicitly run the ignored case:

```powershell
$evidence = Join-Path ([IO.Path]::GetTempPath()) ("otter-winforms-" + [guid]::NewGuid())
[void][IO.Directory]::CreateDirectory($evidence)
$csc = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$env:OTTER_NATIVE_WINFORMS_FIXTURE = Join-Path $evidence 'fixture.exe'
$env:OTTER_NATIVE_WINFORMS_EVIDENCE_ROOT = $evidence
$source = (Resolve-Path -LiteralPath 'scripts/tests/fixtures/LiveInputWinFormsFixture.cs').Path
& $csc /nologo /target:winexe "/out:$env:OTTER_NATIVE_WINFORMS_FIXTURE" /reference:System.Windows.Forms.dll /reference:System.Drawing.dll $source
if ($LASTEXITCODE -ne 0) { throw "WinForms fixture compilation failed: $LASTEXITCODE" }
cargo test --locked --manifest-path native/Cargo.toml -p otter-runtime --lib actual_managed_winforms_click_drag_cancel_and_restore -- --ignored --test-threads=1 --nocapture
```

The case in [`live_input_winforms_tests.rs`](../../native/otter-runtime/src/live_input_winforms_tests.rs)
controls only the CLR process it starts. Each run stores `ready.json` and `state.json` in an exclusive
subdirectory, retained on failure. Without an artifact root, it uses the system temporary directory.
This gate verifies actual clicks, dragging, cancellation and window-style restoration on owned
controls. It sends no system mouse input and does not cover third-party applications, product UI
or cross-machine workflows.

## Diagnostics and evidence boundaries

- Record both ends' versions or commits, package digests, network topology, source/viewer roles,
  dimensions, instance counts, duration and media format, but no tokens or user pixels.
  Performance reports must cover Otter, the hook, Loom and helper processes, not just main-process memory.
- Check discovery, authorization, session state, packet receipt, decoding, GPU upload and presentation
  separately. A successful WebSocket upgrade does not prove receipt of valid frames. For reconnection,
  inspect session, epoch, frame-ID progress and actual presentation; use server-confirmed recovery watermarks.
- GPU pixel readback, wall-clock duration and packet/decode counts do not establish monitor FPS,
  GPU timing or physical-input latency. Keep separate evidence for ordinary desktops, mixed DPI,
  physical two-machine operation and real public networks. Consult Git history and matching artifacts
  for per-version results.
