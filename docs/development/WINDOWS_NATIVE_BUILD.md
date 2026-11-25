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

```powershell
npm run check:fast
cargo nextest run --locked --manifest-path native/Cargo.toml -p otter-core -p otter-media -p otter-runtime
cargo nextest run --locked --manifest-path native/Cargo.toml -p otter-native --lib --features test-live -E 'test(/live_hover/)'
cargo nextest run --locked --manifest-path native/Cargo.toml -p otter-runtime --lib -E 'test(/loom_handshake/)'
npm run test:effective-lines
python -m unittest discover -s scripts/tests -p 'test_ci_*.py'
```

## Internal ZIP and runtime startup

Run from the Otter root. Use new paths for candidate, package and runtime-evidence directories.
`-TargetDirectory` is a fixed, reusable candidate build cache: do not write to it concurrently
or recreate it for each run. This packaging script requires output and cache directories to be
outside the repository and not contain one another. Normal iteration still reuses `native/target`.
Keep all active resources on local `C:`, not network drives or redirected user folders.

```powershell
.\scripts\build-native-candidate.ps1 -OutputDirectory C:/build/otter-candidate -TargetDirectory C:/build/otter-target
.\scripts\package-native-candidate.ps1 -CandidateDirectory C:/build/otter-candidate -OutputDirectory C:/build/otter-package
$zip = Get-ChildItem C:/build/otter-package -Filter '*-internal.zip' -File
.\scripts\verify-native-package.ps1 -ZipPath $zip.FullName
python scripts/tests/native-package-runtime.py --package-dir C:/build/otter-package --evidence-dir C:/build/otter-runtime
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
