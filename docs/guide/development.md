# Development and verification

[简体中文](development.zh-CN.md) · [Documentation / 文档中心](../README.md)

Run the commands below from the repository root. For contribution rules and ownership, follow [the engineering guide](../../CONTRIBUTING.md).

## Windows-native development

Use Windows 10/11, the Rust x64 MSVC toolchain, Visual Studio C++ Build Tools
and a Windows resource compiler (`rc.exe` or `llvm-rc.exe` on PATH, or set
`OTTER_NATIVE_RC` to its absolute path). Node.js and Python are used by tooling,
not by the application. No WebView2 or sibling NeuroDesign checkout is required.

```powershell
cargo fetch --locked --manifest-path native/Cargo.toml
npm run build
npm run verify:local
# Launch the app in development after exiting other Otter instances normally.
npm run dev
```

Cargo fetches `neuro-mini-ui` from NeuroDesign at the exact Git `rev` recorded in
`native/Cargo.toml` and `native/Cargo.lock`. The release build outputs
`native/target/release/otter-native.exe` plus `otter_live_hover_hook.dll` by default.
For distribution to testers, build the complete **internal** ZIP instead:

```powershell
.\scripts\build-native-candidate.ps1 -OutputDirectory "$env:TEMP/otter-candidate" -TargetDirectory "$env:TEMP/otter-native-target"
.\scripts\package-native-candidate.ps1 -CandidateDirectory "$env:TEMP/otter-candidate" -OutputDirectory "$env:TEMP/otter-package"
.\scripts\verify-native-package.ps1 -ZipPath (Get-ChildItem "$env:TEMP/otter-package/*-internal.zip").FullName
```

Use fresh candidate and package destinations. `cargo fetch` above primes the
offline candidate gate. Public native signing and release are not enabled.

## Historical Tauri/WebView verification

The procedures below document the archived product, not native acceptance.
Check out `archive/webview-main` to reproduce the original WebView product.

Tauri v2 · SolidJS · TypeScript · Rust

```powershell
npm ci
npm run dev:tauri
```


## Requirements

- Windows 10 or Windows 11;
- WebView2 Runtime;
- Node.js 22.12+ for frontend development on the Node 22 line; use the repository's
  locked dependencies (`npm ci`) rather than updating dependency versions to set up a checkout;
- Rust stable with the MSVC toolchain for desktop builds.

HDR capture is available only when the selected Windows 11 display reports HDR
support. Unsupported or SDR-only cases fall back automatically. See
[`docs/features/HDR_CAPTURE.md`](../features/HDR_CAPTURE.md).

## Development verification

Useful checks:

```powershell
npm run typecheck
npm run typecheck:test
npm run lint
npm run test:performance
npm run test:surface-browser
npm run test:parallel
npm test
npm run probe:live-screenshot:phase2
npm run probe:live-screenshot:phase3
cargo fmt --check --manifest-path src-tauri\Cargo.toml
cargo test --manifest-path src-tauri\Cargo.toml
npm run build
```

`npm run test:surface-browser` launches an isolated headless Chromium, imports
the production JavaScript Surface document builder, and exercises healthy,
timer, DOM, CPU long-task, and heap-growth budget paths. It does not launch a
second native Otter process or bypass Otter's global single-instance safety mutex.

The live-capture G2 probe is interactive and runs for 600 seconds by default. It
uses an animated same-integrity WinForms fixture to prove fresh WGC frames,
resize/recreate, source-close fail-closed behavior, session cleanup, and bounded
handle/private-memory growth. Its evidence is written below
`artifacts/live-screenshot-phase2/<run-id>` and is intentionally not committed.
Local WebView presentation uses raw Tauri binary responses carrying bounded JPEG
frames; it does not send frames through `loom.surface.v1` or JSON/Base64.

The G3 probe covers the deliberately narrow Windows 11, single-SDR-display,
same-integrity Win32/WinForms declaration. It verifies changing frames while the
source is a near-transparent compositor window, independent input edges, exact
restore, local reclaim, recovery-journal execution, and worker cleanup. Otter does
not claim native-minimize, elevated, cross-session, WPF, WinUI, or arbitrary
custom-control support from this result.

Validate a built native candidate in two stages. Preflight only hashes the
candidate, checks the CDP dependency/port, and reports any live Otter main or
watchdog process; it never launches or stops Otter:

The canonical-only acceptance baseline requires release checks to exercise
only the current app-data identity, publisher-qualified packages, and formal
`loom.hook.v1`/`loom.surface.v1` contracts. Obsolete compatibility paths are not
part of acceptance.

The acceptance scripts fail closed on SHA-256. Pass the candidate's matching
expected digest explicitly; historical script defaults do not identify the
current build. Each candidate needs its own 600-second soak, formal Surface
action/resource delivery, clean exit and restart-recovery evidence. Older
acceptance summaries remain in Git history and do not certify a later binary.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\scripts\Invoke-OtterNativeCandidateAcceptance.ps1 `
  -OtterExe ..\release\Otter\candidate\otter.exe `
  -ExpectedSha256 <candidate-sha256> `
  -PreflightOnly
```

After every existing Otter instance has been exited normally, remove
`-PreflightOnly` to run the real WebView2/Tauri IPC probe, global
single-instance refusal, process-tree memory soak, normal exit, watchdog/CDP
teardown, and restart check. The script uses isolated app data and WebView2
storage. It enables `HOOK_NATIVE_ACCEPTANCE=1` only for the spawned candidate so
the gated clean-exit command can return through the normal Tauri shutdown path;
it does not rename/bypass the mutex or disable global Otter behavior.

The packaged Otter/Loom dual-end gate uses the outer orchestrator instead of treating a
daemon-only or browser-only smoke as sufficient. It verifies both candidate
hashes, starts an isolated packaged Loom daemon and fixture Art Store, installs
the real dashboard Surface, and delegates to the native Otter runner:

```powershell
# Safe while another Otter is running: hashes/files/ports/processes only.
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\scripts\Invoke-OtterLoomSurfaceCandidateAcceptance.ps1 `
  -OtterExe ..\release\Otter\candidate\otter.exe `
  -ExpectedOtterSha256 <candidate-sha256> `
  -LoomPackageDir ..\release\Loom\candidate `
  -ExpectedLoomDaemonSha256 <loom-daemon-sha256> `
  -PreflightOnly

# Safe isolated Loom-side rehearsal; does not launch Otter.
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\scripts\Invoke-OtterLoomSurfaceCandidateAcceptance.ps1 `
  -OtterExe ..\release\Otter\candidate\otter.exe `
  -ExpectedOtterSha256 <candidate-sha256> `
  -LoomPackageDir ..\release\Loom\candidate `
  -ExpectedLoomDaemonSha256 <loom-daemon-sha256> `
  -ValidateLoomServicesOnly
```

With all existing Otter main/watchdog processes normally exited, omit both
switches for the full packaged Otter GUI → packaged Loom Surface click/resource/
formal-result loop, ten-minute process-tree soak, clean exit, restart recovery,
and final listener/process cleanup. The native runner waits for Otter's awaited
`frontend-initialized` marker before both the first Surface probe and the restart
probe, so CDP availability alone is not treated as frontend readiness.

When Loom reports a newly registered device as pending, the runner approves that
exact public-key device through the isolated Loom control plane while Otter waits
for the bounded authorization transition. Native acceptance exits through the
normal Tauri run loop and requires exactly one process-exit cleanup record before
checking watchdog, CDP, and restart recovery.

Build a portable executable directly. At the start of a new internal development
iteration, run `npm run version:internal` once and commit `version-state.json`
with that iteration. Do not bump again for a rebuild of the same iteration or
for documentation-only maintenance. Keep public package versions unchanged;
see [versioning](../release/VERSIONING.md).

Resolve the current four-part build identity and use a new output directory:

```powershell
. .\scripts\version-identity.ps1
$identity = Get-OtterVersionIdentity -RepoRoot (Get-Location).Path
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\scripts\build-local-otter-exe.ps1 `
  -OutputDir "..\release\Otter\$($identity.buildVersion)"
```

Do not overwrite an already handed-off candidate. For a repeat build, use a new
candidate subdirectory and retain its provenance and checksum. This build command
does not itself run every release or native acceptance gate described above.

Development rules and module ownership are documented in
[`CONTRIBUTING.md`](../../CONTRIBUTING.md). The current runtime structure is documented
in [`docs/architecture/overview.md`](../architecture/overview.md).



Development CI policy: valid vulnerability, license, lint, format, and size findings
are advisory in independent reporting workflows. Tool/report/upload failures remain
errors; typechecking, builds, and functional tests stay blocking. Release checks and
`npm run verify:local` remain strict. See [dependency security policy](../security/DEPENDENCY_SECURITY.md).
