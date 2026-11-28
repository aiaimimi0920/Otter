# Building Otter for development

For Windows x64, use Rust MSVC, Visual Studio C++ Build Tools and the Windows SDK
(`llvm-rc.exe` can also compile Windows resources). Node.js 22 runs version and packaging
helper scripts; no npm dependencies need to be installed. Python 3.11+ runs archive startup
probes and joint tests. The first fetch downloads locked crates and the pinned NeuroDesign
Git commit; no adjacent NeuroDesign checkout is required. Set `OTTER_NATIVE_RC` only when
the resource compiler cannot be found on PATH.

```powershell
cargo fetch --locked --manifest-path native/Cargo.toml
npm run dev
npm run verify:local
```

`npm run dev` builds both the main program and its companion Live DLL. `check:fast` requires
an installed nightly toolchain; `test:nextest` requires `cargo-nextest`. Ordinary builds and
`verify:local` require neither. Select focused regressions by their actual owning crate
to avoid zero matches after tests move:

The product CI uses a pinned Nextest runner with the full default test selection,
one test process at a time, followed by Cargo doctests. Process isolation prevents
a failed shared test lock from poisoning unrelated cases; original failures still
block the same-run package and runtime gates.
Real-foreground smoke cases run first. CI stops on the first failed test without
retrying or promoting a package; a successful run still requires the complete
default inventory, doctests and all subsequent gates.
On disposable GitHub-hosted Windows runners only, the Nextest step temporarily
sets the **in-memory** foreground lock timeout to zero and restores the previous
value before packaging. This allows isolated test processes to request focus
without a human clicking through Windows Terminal. Tests must still obtain and
verify the actual foreground HWND; no assertion, test or product policy is replaced.
On an elevated GitHub-hosted Windows runner, the native Nextest step builds the
full workspace inventory, then executes the same binaries under a temporary
standard local account. This keeps the ordinary-token worker boundary enabled
without filtering its tests. To run its three-mode end-to-end check locally:

```powershell
cargo nextest run --locked --offline --manifest-path native/Cargo.toml -p otter-media --lib -E 'test(=bounded_worker_process::session::tests::terminal_boundary_requires_successful_exit_and_no_extra_output)'
```
This local result does not claim GitHub-hosted worker execution.

Native hit/focus fixtures use a private desktop without switching the user's input
desktop. They retain real HWND, focus, message and cleanup assertions, but do not
replace foreground-behavior acceptance on a shared interactive desktop. The edit-strip
toggle fixture verifies `WM_COMMAND/BN_CLICKED` delivery, not physical button input.
On systems where WIC reports an unavailable component, admitted static WebP stickers
may use the existing bounded Surface child; larger or animated WebP still depends on
WIC and does not receive enlarged worker limits.

```powershell
npm run check:fast
cargo nextest run --locked --manifest-path native/Cargo.toml -p otter-core -p otter-media -p otter-runtime
cargo nextest run --locked --manifest-path native/Cargo.toml -p otter-native --lib --features test-live -E 'test(/live_hover/)'
cargo nextest run --locked --manifest-path native/Cargo.toml -p otter-runtime --lib -E 'test(/loom_handshake/)'
npm run test:effective-lines
python -m unittest discover -s scripts/tests -p 'test_ci_*.py'
```

## Compilation-only CI

`Compile Windows (no package)` (`.github/workflows/compile-windows.yml`) runs on pushes
to `main` or by manual dispatch. It executes `npm run build`'s locked Release workspace
command with Rust 1.95.0, compiling both `otter-native.exe` and `otter_live_hover_hook.dll`.
Only a JSON receipt is uploaded: the exact GitHub/checkout commit, lockfile digest and
the nonempty EXE/DLL sizes and SHA-256 digests. No executable, ZIP or Release is uploaded.
This lane does not run or approve product tests, packaging or runtime acceptance.
The independent `Build Otter EXE` workflow and stable `build-windows-exe` gate still
require their existing tests and package/runtime verification before promoting a package.

## Internal ZIP and runtime startup

Run from the Otter root. Use new paths for candidate, package and runtime-evidence directories.
`-TargetDirectory` is a fixed, reusable candidate build cache: do not write to it concurrently
or recreate it for each run. This packaging script requires output and cache directories to be
outside the repository and not contain one another. Normal iteration still reuses `native/target`.
Keep all active resources on local `C:`, not network drives or redirected user folders.

```powershell
.\scripts\build-native-candidate.ps1 -OutputDirectory ../Release/Otter/candidate -TargetDirectory ../Release/Otter/target
.\scripts\package-native-candidate.ps1 -CandidateDirectory ../Release/Otter/candidate -OutputDirectory ../Release/Otter/package
$zip = Get-ChildItem ../Release/Otter/package -Filter '*-internal.zip' -File
.\scripts\verify-native-package.ps1 -ZipPath $zip.FullName
python scripts/tests/native-package-runtime.py --package-dir ../Release/Otter/package --evidence-dir ../Release/Otter/runtime
```

Cross-check the original package, provenance digests and evidence. Release builds exclude the
development `--smoke-test` modules by default. Debug integration tests cover the full smoke suite;
Release integration smoke tests require `--features smoke-test` in the Cargo command. Unit-test
helpers remain under `cfg(test)`, and default Release integration tests separately verify smoke rejection.
Launch the executable extracted from the ZIP with an isolated profile: execute `--workspace-check` and `--session-check`
with an external PNG asset, including hidden window creation, Direct2D drawing, device recreation
and verification that the input PNG is unchanged. Check that an empty profile creates no workspace
file and that `--smoke-test` and invalid arguments are rejected as expected. Release retains default
unwinding so worker panic reporting and destructor-based file-replacement recovery are not bypassed;
size optimizations must preserve these recovery contracts.
The two hidden startup checks have 60-second hang limits; the three argument-rejection checks have
15-second limits. No session-fixture directories remain after the probe completes.
Receipts record actual duration, limits, exit codes and success markers; these limits are not
cold-start performance metrics.
Do not replace a running Otter instance. Follow [Live joint testing](../features/LIVE_JOINT_TESTING.md)
and [projection joint testing](JOINT_TESTING.md) for real desktop integration checks.
Keep the EXE and adjacent DLL together when trying the complete package. Internal packages are unsigned
and are not public releases.

## Investigating unexpected exits

Preserve the affected package, workspace and original diagnostics. Record EXE/DLL digests,
startup arguments, PID and exit code; distinguish normal exit, external termination and program errors.
First verify actual ZIP startup as described above, then reproduce with a separate profile and
a source window you own, without closing user instances or sending global input. For Live failures,
inspect the source window and recovery journal: process exit alone does not prove that input,
hover windows or source permissions have been released.
